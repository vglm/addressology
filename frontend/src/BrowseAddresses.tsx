import React, { useEffect, useMemo, useRef, useState } from "react";
import { backendFetch } from "./common/BackendCall";

import "./BrowseAddresses.css";
import { ethers } from "ethers";
import { DateTime } from "luxon";
import { Link, useSearchParams } from "react-router-dom";
import { Fancy, FancyCategoryInfo, FancyStats, PublicKeyBase } from "./model/Fancy";
import {
    Button,
    Chip,
    IconButton,
    LinearProgress,
    MenuItem,
    Select,
    TextField,
    ToggleButton,
    ToggleButtonGroup,
    Tooltip,
} from "@mui/material";

interface SelectCategoryProps {
    selectedCategory: string;
    setSelectedCategory: (category: string) => void;
}

export const SelectCategory = (props: SelectCategoryProps) => {
    const [categories, setCategories] = useState<FancyCategoryInfo[] | null>(null);
    const loadCategories = async () => {
        const response = await backendFetch("/api/fancy/categories", {
            method: "Get",
        });
        const addresses = await response.json();

        setCategories(addresses);
    };

    useEffect(() => {
        loadCategories().then();
    }, []);

    return (
        <Select
            variant={"outlined"}
            defaultValue={props.selectedCategory}
            onChange={(e) => props.setSelectedCategory(e.target.value)}
        >
            <MenuItem value={"all"}>All</MenuItem>
            {categories &&
                categories.map((category) => {
                    return (
                        <MenuItem key={category.key} value={category.key}>
                            {category.name}
                        </MenuItem>
                    );
                })}
        </Select>
    );
};

type PatternMode = "prefix" | "contains" | "suffix";
type SortMode = "score" | "newest" | "oldest";
type TimeRange = "any" | "1h" | "24h" | "7d" | "30d";

const GPU_SPEED_MH: { [gpu: string]: number } = {
    "RTX 3060": 350,
    "RTX 3090": 990,
    "RTX 4090": 2300,
};
const GPU_COST_PER_HOUR = 0.1;
const PAGE_SIZES = [25, 50, 100];

const TIME_RANGES: { key: TimeRange; label: string; seconds: number | null }[] = [
    { key: "any", label: "Any time", seconds: null },
    { key: "1h", label: "Last hour", seconds: 3600 },
    { key: "24h", label: "Last 24h", seconds: 86400 },
    { key: "7d", label: "Last 7 days", seconds: 7 * 86400 },
    { key: "30d", label: "Last 30 days", seconds: 30 * 86400 },
];

function normalizePattern(raw: string): string {
    const trimmed = raw.trim().toLowerCase();
    return trimmed.startsWith("0x") ? trimmed.slice(2) : trimmed;
}

function patternError(raw: string, mode: PatternMode): string | null {
    const pattern = normalizePattern(raw);
    if (pattern.length === 0) {
        return null;
    }
    if (!/^[0-9a-f]*$/.test(pattern)) {
        return "Only hex digits are allowed (0-9, a-f)";
    }
    if (pattern.length > 40) {
        return "Addresses have only 40 hex digits";
    }
    if (mode !== "prefix" && pattern.length < 3) {
        return "Use at least 3 digits for contains / ends with";
    }
    return null;
}

function displayDifficulty(difficulty: number): string {
    const units = ["", "kH", "MH", "GH", "TH", "PH", "EH", "ZH", "YH"];
    let unitIndex = 0;

    while (difficulty >= 1000 && unitIndex < units.length - 1) {
        difficulty /= 1000;
        unitIndex++;
    }

    const precision = difficulty < 10 ? 3 : difficulty < 100 ? 2 : 1;
    return difficulty.toFixed(precision) + units[unitIndex];
}

function displayTime(seconds: number): string {
    const units = [
        { label: "month", seconds: 2_592_000, cutoff: 604_800 * 9.9 },
        { label: "week", seconds: 604_800, cutoff: 86_400 * 9.9 },
        { label: "day", seconds: 86_400, cutoff: 3600 * 48 },
        { label: "hour", seconds: 3_600, cutoff: 60 * 99 },
        { label: "minute", seconds: 60, cutoff: 99 },
        { label: "second", seconds: 1, cutoff: 0 },
    ];

    for (const unit of units) {
        if (seconds >= unit.cutoff) {
            const value = seconds / unit.seconds;
            const precision = value < 10 ? 2 : value < 100 ? 1 : 0;
            return `${value.toFixed(precision)} ${unit.label}${value >= 2 ? "s" : ""}`;
        }
    }

    return "0 seconds";
}

function gpuSeconds(score: number, gpu: string): number {
    const mhs = GPU_SPEED_MH[gpu] ?? 350;
    return score / 1_000_000 / mhs;
}

function displayEstimatedCost(score: number, gpu: string): string {
    const cost = (gpuSeconds(score, gpu) / 3600) * GPU_COST_PER_HOUR;
    if (cost > 0 && cost < 0.01) {
        return "<$0.01";
    }
    return `$${cost.toFixed(2)}`;
}

function formatCount(count: number): string {
    if (count >= 1_000_000) {
        return `${(count / 1_000_000).toFixed(count >= 10_000_000 ? 0 : 1)}M`;
    }
    if (count >= 1_000) {
        return `${(count / 1_000).toFixed(count >= 100_000 ? 0 : 1)}k`;
    }
    return count.toString();
}

function parseCreated(created: string): DateTime {
    return DateTime.fromISO(created, { zone: "utc" });
}

function displayRelative(created: string): string {
    return parseCreated(created).toRelative() ?? created;
}

interface HighlightedAddressProps {
    address: string;
    pattern: string;
    mode: PatternMode;
}

const HighlightedAddress = (props: HighlightedAddressProps) => {
    const mixedCase = ethers.getAddress(props.address);
    const body = mixedCase.slice(2);
    const lowerBody = props.address.toLowerCase().slice(2);

    let matchStart = -1;
    let matchEnd = -1;
    if (props.pattern) {
        if (props.mode === "prefix" && lowerBody.startsWith(props.pattern)) {
            matchStart = 0;
            matchEnd = props.pattern.length;
        } else if (props.mode === "suffix" && lowerBody.endsWith(props.pattern)) {
            matchStart = lowerBody.length - props.pattern.length;
            matchEnd = lowerBody.length;
        } else if (props.mode === "contains") {
            const idx = lowerBody.indexOf(props.pattern);
            if (idx >= 0) {
                matchStart = idx;
                matchEnd = idx + props.pattern.length;
            }
        }
    } else {
        // No search pattern: emphasize the leading run of identical characters, if any.
        let run = 1;
        while (run < lowerBody.length && lowerBody[run] === lowerBody[0]) {
            run++;
        }
        if (run >= 4) {
            matchStart = 0;
            matchEnd = run;
        }
    }

    if (matchStart < 0) {
        return (
            <span className="addr-mono">
                <span className="addr-0x">0x</span>
                {body}
            </span>
        );
    }
    return (
        <span className="addr-mono">
            <span className="addr-0x">0x</span>
            {body.slice(0, matchStart)}
            <mark className="addr-match">{body.slice(matchStart, matchEnd)}</mark>
            {body.slice(matchEnd)}
        </span>
    );
};

const CopyButton = (props: { text: string }) => {
    const [copied, setCopied] = useState(false);
    const copy = async () => {
        try {
            await navigator.clipboard.writeText(props.text);
            setCopied(true);
            setTimeout(() => setCopied(false), 1200);
        } catch (e) {
            console.error("Clipboard copy failed", e);
        }
    };
    return (
        <Tooltip title={copied ? "Copied!" : "Copy address"} placement="top">
            <IconButton size="small" className="copy-btn" onClick={copy} aria-label="Copy address">
                {copied ? (
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#16a34a" strokeWidth="3">
                        <polyline points="20 6 9 17 4 12" />
                    </svg>
                ) : (
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <rect x="9" y="9" width="13" height="13" rx="2" />
                        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                    </svg>
                )}
            </IconButton>
        </Tooltip>
    );
};

const URL_KEYS = ["q", "mode", "cat", "time", "avail", "key", "sort", "page", "ps"];

const BrowseAddresses = () => {
    const [searchParams, setSearchParams] = useSearchParams();
    const stored = useMemo<{ [key: string]: string }>(() => {
        try {
            return JSON.parse(localStorage.getItem("browseFilters") ?? "{}");
        } catch (_e) {
            return {};
        }
    }, []);
    const hasUrlState = useMemo(() => URL_KEYS.some((key) => searchParams.has(key)), []);
    const initial = (key: string, def: string): string => {
        if (hasUrlState) {
            return searchParams.get(key) ?? def;
        }
        return stored[key] ?? def;
    };

    const [qInput, setQInput] = useState<string>(() => initial("q", ""));
    const [mode, setMode] = useState<PatternMode>(() => initial("mode", "prefix") as PatternMode);
    const [category, setCategory] = useState<string>(() => initial("cat", "all"));
    const [time, setTime] = useState<TimeRange>(() => initial("time", "any") as TimeRange);
    const [avail, setAvail] = useState<string>(() => initial("avail", "free"));
    const [keyBase, setKeyBase] = useState<string>(() => initial("key", "all"));
    const [sort, setSort] = useState<SortMode>(() => initial("sort", "score") as SortMode);
    const [page, setPage] = useState<number>(() => parseInt(initial("page", "0")) || 0);
    const [pageSize, setPageSize] = useState<number>(() => parseInt(initial("ps", "50")) || 50);
    const [gpu, setGpu] = useState<string>(() => localStorage.getItem("gpu") ?? "RTX 3060");

    const [debouncedQ, setDebouncedQ] = useState<string>(qInput);
    const [entries, setEntries] = useState<Fancy[]>([]);
    const [total, setTotal] = useState<number>(0);
    const [totalCapped, setTotalCapped] = useState<boolean>(false);
    const [loading, setLoading] = useState<boolean>(true);
    const [error, setError] = useState<string | null>(null);
    const [stats, setStats] = useState<FancyStats | null>(null);
    const [categoryNames, setCategoryNames] = useState<{ [key: string]: string }>({});
    const [publicKeyBases, setPublicKeyBases] = useState<PublicKeyBase[]>([]);
    const requestCounter = useRef(0);

    const qError = patternError(qInput, mode);
    const pattern = qError ? "" : normalizePattern(debouncedQ);
    const appliedPattern = qError ? "" : normalizePattern(qInput);

    useEffect(() => {
        const timer = setTimeout(() => setDebouncedQ(qInput), 350);
        return () => clearTimeout(timer);
    }, [qInput]);

    const setFilter = <T,>(setter: (value: T) => void) => {
        return (value: T) => {
            setter(value);
            setPage(0);
        };
    };

    const isDefaultFilters =
        qInput === "" &&
        category === "all" &&
        time === "any" &&
        avail === "free" &&
        keyBase === "all" &&
        sort === "score";

    const resetFilters = () => {
        setQInput("");
        setDebouncedQ("");
        setMode("prefix");
        setCategory("all");
        setTime("any");
        setAvail("free");
        setKeyBase("all");
        setSort("score");
        setPage(0);
    };

    const sinceForTime = (range: TimeRange): string | null => {
        const preset = TIME_RANGES.find((t) => t.key === range);
        if (!preset || preset.seconds == null) {
            return null;
        }
        const since = new Date(Date.now() - preset.seconds * 1000);
        return since.toISOString().replace(/\.\d{3}Z$/, "Z");
    };

    const loadAddresses = async () => {
        const requestId = ++requestCounter.current;
        const params = new URLSearchParams();
        if (pattern) {
            params.set("q", pattern);
            params.set("mode", mode);
        }
        if (category !== "all") {
            params.set("category", category);
        }
        const since = sinceForTime(time);
        if (since) {
            params.set("since", since);
        }
        params.set("free", avail);
        params.set("key", keyBase);
        params.set("order", sort === "score" ? "score" : "created");
        params.set("dir", sort === "oldest" ? "asc" : "desc");
        params.set("limit", pageSize.toString());
        params.set("offset", (page * pageSize).toString());

        setLoading(true);
        setError(null);
        try {
            const response = await backendFetch(`/api/fancy/search?${params.toString()}`, {
                method: "Get",
            });
            if (!response.ok) {
                throw new Error(`${response.status} ${await response.text()}`);
            }
            const result = await response.json();
            if (requestId !== requestCounter.current) {
                return;
            }
            setEntries(result.entries);
            setTotal(result.total);
            setTotalCapped(result.totalCapped);
        } catch (e) {
            if (requestId !== requestCounter.current) {
                return;
            }
            console.error("Search failed", e);
            setError(`Search failed: ${e}`);
        } finally {
            if (requestId === requestCounter.current) {
                setLoading(false);
            }
        }
    };

    const loadStats = async () => {
        try {
            const response = await backendFetch("/api/fancy/stats", { method: "Get" });
            setStats(await response.json());
        } catch (e) {
            console.error("Failed to load stats", e);
        }
    };

    const loadCategoryNames = async () => {
        try {
            const response = await backendFetch("/api/fancy/categories", { method: "Get" });
            const categories: FancyCategoryInfo[] = await response.json();
            const names: { [key: string]: string } = {};
            for (const cat of categories) {
                names[cat.key] = cat.name;
            }
            setCategoryNames(names);
        } catch (e) {
            console.error("Failed to load categories", e);
        }
    };

    const loadPublicKeyBases = async () => {
        try {
            const response = await backendFetch("/api/public_key_base/list", { method: "Get" });
            setPublicKeyBases(await response.json());
        } catch (e) {
            console.error("Failed to load public key bases", e);
        }
    };

    useEffect(() => {
        loadStats().then();
        loadCategoryNames().then();
        loadPublicKeyBases().then();
    }, []);

    useEffect(() => {
        loadAddresses().then();
    }, [pattern, mode, category, time, avail, keyBase, sort, page, pageSize]);

    useEffect(() => {
        const urlState: { [key: string]: string } = {};
        if (qInput) urlState.q = qInput;
        if (mode !== "prefix") urlState.mode = mode;
        if (category !== "all") urlState.cat = category;
        if (time !== "any") urlState.time = time;
        if (avail !== "free") urlState.avail = avail;
        if (keyBase !== "all") urlState.key = keyBase;
        if (sort !== "score") urlState.sort = sort;
        if (page > 0) urlState.page = page.toString();
        if (pageSize !== 50) urlState.ps = pageSize.toString();
        setSearchParams(urlState, { replace: true });
        localStorage.setItem(
            "browseFilters",
            JSON.stringify({
                q: qInput,
                mode,
                cat: category,
                time,
                avail,
                key: keyBase,
                sort,
                ps: pageSize.toString(),
            }),
        );
    }, [qInput, mode, category, time, avail, keyBase, sort, page, pageSize]);

    useEffect(() => {
        localStorage.setItem("gpu", gpu);
    }, [gpu]);

    const sortedCategoryStats = stats?.categories ?? [];
    const categoryLabel = (key: string) => categoryNames[key] ?? key;

    const pageCount = Math.max(1, Math.ceil(total / pageSize));
    const firstShown = total === 0 ? 0 : page * pageSize + 1;
    const lastShown = Math.min(total, (page + 1) * pageSize);
    const totalDisplay = `${total.toLocaleString("en-US")}${totalCapped ? "+" : ""}`;
    const canGoNext = (page + 1) * pageSize < total || (totalCapped && entries.length === pageSize);

    const newestOverall = stats?.newest ? displayRelative(stats.newest) : null;

    return (
        <div className="browse-page">
            <div className="browse-header">
                <h1>Browse Addresses</h1>
                <div className="browse-stats">
                    <div className="stat-card">
                        <div className="stat-value">{stats ? stats.total.toLocaleString("en-US") : "…"}</div>
                        <div className="stat-label">addresses indexed</div>
                    </div>
                    <div className="stat-card">
                        <div className="stat-value">{newestOverall ?? "…"}</div>
                        <div className="stat-label">latest address mined</div>
                    </div>
                    <div className="stat-card">
                        <div className="stat-value">
                            {stats ? `${displayDifficulty(stats.estimatedWorkTh * 1e12)}` : "…"}
                        </div>
                        <div className="stat-label">estimated total work</div>
                    </div>
                </div>
            </div>

            <div className="browse-toolbar">
                <div className="toolbar-row">
                    <TextField
                        className="search-input"
                        size="small"
                        placeholder="Search hex pattern, e.g. dead or 0000…"
                        value={qInput}
                        error={qError != null}
                        helperText={qError ?? " "}
                        onChange={(e) => {
                            setQInput(e.target.value);
                            setPage(0);
                        }}
                        inputProps={{ spellCheck: false, className: "search-input-inner" }}
                    />
                    <ToggleButtonGroup
                        exclusive
                        size="small"
                        value={mode}
                        onChange={(_e, value) => {
                            if (value) {
                                setMode(value);
                                setPage(0);
                            }
                        }}
                        aria-label="Pattern position"
                    >
                        <ToggleButton value="prefix">Starts with</ToggleButton>
                        <ToggleButton value="contains">Contains</ToggleButton>
                        <ToggleButton value="suffix">Ends with</ToggleButton>
                    </ToggleButtonGroup>
                    <div className="toolbar-spacer" />
                    <Button variant="outlined" size="small" onClick={() => loadAddresses().then(loadStats)}>
                        Refresh
                    </Button>
                    <Button variant="text" size="small" onClick={resetFilters} disabled={isDefaultFilters}>
                        Clear filters
                    </Button>
                </div>

                <div className="toolbar-row category-row">
                    <Chip
                        label={
                            <span>
                                All <span className="chip-count">{stats ? formatCount(stats.total) : ""}</span>
                            </span>
                        }
                        size="small"
                        className={category === "all" ? "cat-chip cat-chip-selected" : "cat-chip"}
                        onClick={() => setFilter(setCategory)("all")}
                    />
                    {sortedCategoryStats.map((stat) => (
                        <Chip
                            key={stat.category}
                            label={
                                <span>
                                    {categoryLabel(stat.category)}{" "}
                                    <span className="chip-count">{formatCount(stat.total)}</span>
                                </span>
                            }
                            size="small"
                            className={category === stat.category ? "cat-chip cat-chip-selected" : "cat-chip"}
                            onClick={() => setFilter(setCategory)(stat.category)}
                        />
                    ))}
                </div>

                <div className="toolbar-row selects-row">
                    <label className="select-group">
                        <span className="select-label">Sort</span>
                        <Select
                            size="small"
                            value={sort}
                            onChange={(e) => setFilter(setSort)(e.target.value as SortMode)}
                        >
                            <MenuItem value="score">Best score</MenuItem>
                            <MenuItem value="newest">Newest first</MenuItem>
                            <MenuItem value="oldest">Oldest first</MenuItem>
                        </Select>
                    </label>
                    <label className="select-group">
                        <span className="select-label">Mined</span>
                        <Select
                            size="small"
                            value={time}
                            onChange={(e) => setFilter(setTime)(e.target.value as TimeRange)}
                        >
                            {TIME_RANGES.map((range) => (
                                <MenuItem key={range.key} value={range.key}>
                                    {range.label}
                                </MenuItem>
                            ))}
                        </Select>
                    </label>
                    <label className="select-group">
                        <span className="select-label">Availability</span>
                        <Select size="small" value={avail} onChange={(e) => setFilter(setAvail)(e.target.value)}>
                            <MenuItem value="free">Free</MenuItem>
                            <MenuItem value="reserved">Reserved</MenuItem>
                            <MenuItem value="mine">Mine</MenuItem>
                            <MenuItem value="all">Any</MenuItem>
                        </Select>
                    </label>
                    <label className="select-group">
                        <span className="select-label">Key base</span>
                        <Select size="small" value={keyBase} onChange={(e) => setFilter(setKeyBase)(e.target.value)}>
                            <MenuItem value="all">Any key</MenuItem>
                            <MenuItem value="none">No key (salt based)</MenuItem>
                            {publicKeyBases.map((pkb) => (
                                <MenuItem key={pkb.uid} value={pkb.hex}>
                                    {pkb.hex.slice(0, 10)}…{pkb.hex.slice(-4)}
                                </MenuItem>
                            ))}
                            {keyBase !== "all" &&
                                keyBase !== "none" &&
                                !publicKeyBases.some((pkb) => pkb.hex === keyBase) && (
                                    <MenuItem value={keyBase}>
                                        {keyBase.slice(0, 10)}…{keyBase.slice(-4)}
                                    </MenuItem>
                                )}
                        </Select>
                    </label>
                    <label className="select-group">
                        <span className="select-label">Your GPU</span>
                        <Select size="small" value={gpu} onChange={(e) => setGpu(e.target.value)}>
                            {Object.keys(GPU_SPEED_MH).map((gpuName) => (
                                <MenuItem key={gpuName} value={gpuName}>
                                    {gpuName}
                                </MenuItem>
                            ))}
                        </Select>
                    </label>
                </div>
            </div>

            <div className="browse-result-bar">
                <div className="result-count">
                    {loading ? "Searching…" : `${totalDisplay} matching address${total === 1 ? "" : "es"}`}
                </div>
                <div className="pagination">
                    <span className="page-info">
                        {firstShown.toLocaleString("en-US")}–{lastShown.toLocaleString("en-US")} of {totalDisplay}
                    </span>
                    <Button size="small" disabled={page === 0 || loading} onClick={() => setPage(page - 1)}>
                        ‹ Prev
                    </Button>
                    <Button size="small" disabled={!canGoNext || loading} onClick={() => setPage(page + 1)}>
                        Next ›
                    </Button>
                    <Select
                        size="small"
                        value={pageSize}
                        onChange={(e) => {
                            setPageSize(Number(e.target.value));
                            setPage(0);
                        }}
                    >
                        {PAGE_SIZES.map((size) => (
                            <MenuItem key={size} value={size}>
                                {size} / page
                            </MenuItem>
                        ))}
                    </Select>
                </div>
            </div>

            {loading && <LinearProgress className="browse-progress" />}
            {error && (
                <div className="browse-error">
                    {error}{" "}
                    <Button size="small" onClick={() => loadAddresses()}>
                        Retry
                    </Button>
                </div>
            )}

            {!loading && !error && entries.length === 0 ? (
                <div className="browse-empty">
                    <div className="browse-empty-title">No addresses found</div>
                    <div className="browse-empty-hints">
                        {time !== "any" && (
                            <Button variant="outlined" size="small" onClick={() => setFilter(setTime)("any")}>
                                Search any time
                            </Button>
                        )}
                        {category !== "all" && (
                            <Button variant="outlined" size="small" onClick={() => setFilter(setCategory)("all")}>
                                All categories
                            </Button>
                        )}
                        {avail !== "all" && (
                            <Button variant="outlined" size="small" onClick={() => setFilter(setAvail)("all")}>
                                Any availability
                            </Button>
                        )}
                        <Button variant="outlined" size="small" onClick={resetFilters}>
                            Clear all filters
                        </Button>
                    </div>
                </div>
            ) : (
                <div className="browse-table-wrap">
                    <table className="browse-table">
                        <thead>
                            <tr>
                                <th>Address</th>
                                <th>Category</th>
                                <th>Difficulty</th>
                                <th>GPU time</th>
                                <th>Est. cost</th>
                                <th>Price</th>
                                <th>Mined</th>
                                <th>Provider</th>
                            </tr>
                        </thead>
                        <tbody>
                            {entries.map((fancy) => {
                                const mixedCaseForm = ethers.getAddress(fancy.address);
                                return (
                                    <tr key={fancy.address}>
                                        <td className="addr-cell">
                                            <Link to={`/address/${mixedCaseForm}`} className="addr-link">
                                                <HighlightedAddress
                                                    address={fancy.address}
                                                    pattern={appliedPattern}
                                                    mode={mode}
                                                />
                                            </Link>
                                            <CopyButton text={mixedCaseForm} />
                                        </td>
                                        <td>
                                            <span className="category-tag" data-category={fancy.category}>
                                                {categoryLabel(fancy.category)}
                                            </span>
                                        </td>
                                        <td className="num-cell">{displayDifficulty(fancy.score)}</td>
                                        <td className="num-cell">{displayTime(gpuSeconds(fancy.score, gpu))}</td>
                                        <td className="num-cell">{displayEstimatedCost(fancy.score, gpu)}</td>
                                        <td className="num-cell">{fancy.price.toLocaleString("en-US")}</td>
                                        <td>
                                            <Tooltip
                                                title={parseCreated(fancy.created)
                                                    .toLocal()
                                                    .toFormat("yyyy-MM-dd HH:mm:ss")}
                                                placement="top"
                                            >
                                                <span className="created-cell">{displayRelative(fancy.created)}</span>
                                            </Tooltip>
                                        </td>
                                        <td>
                                            {fancy.provName ? (
                                                <a
                                                    className="provider-link"
                                                    href={`https://stats.golem.network/network/provider/${fancy.provNodeId}?node_id=${fancy.provRewardAddr}`}
                                                    target="_blank"
                                                    rel="noreferrer"
                                                >
                                                    {fancy.provName}
                                                </a>
                                            ) : (
                                                <span className="provider-none">—</span>
                                            )}
                                        </td>
                                    </tr>
                                );
                            })}
                        </tbody>
                    </table>
                </div>
            )}

            <div className="browse-footer">
                {total > 0 && (
                    <div className="pagination">
                        <span className="page-info">
                            Page {page + 1} of {totalCapped ? `${pageCount}+` : pageCount}
                        </span>
                        <Button size="small" disabled={page === 0 || loading} onClick={() => setPage(page - 1)}>
                            ‹ Prev
                        </Button>
                        <Button size="small" disabled={!canGoNext || loading} onClick={() => setPage(page + 1)}>
                            Next ›
                        </Button>
                    </div>
                )}
            </div>
        </div>
    );
};

export default BrowseAddresses;
