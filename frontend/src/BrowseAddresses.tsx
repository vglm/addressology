import React, { useEffect, useState } from "react";
import { backendFetch } from "./common/BackendCall";

import "./BrowseAddresses.css";
import { ethers } from "ethers";
import { Fancy, FancyCategoryInfo } from "./model/Fancy";
import { Checkbox, FormControlLabel, MenuItem, Select } from "@mui/material";

interface TotalHashInfo {
    estimatedWorkTH: number;
}

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

const BrowseAddresses = () => {
    const [fancies, setFancies] = useState<Fancy[]>([]);
    const [totalHash, setTotalHash] = useState<TotalHashInfo | null>(null);
    const [selectedCategory, setSelectedCategory] = useState<string>("all");
    const [newest, setNewest] = useState<boolean>(false);

    const [gpu, setGpu] = useState<string>("RTX 3060");

    const [showType, setShowType] = useState<string>("today");
    const [showFree, setShowFree] = useState<string>("free");

    const showTypeToSince = (showType: string): string => {
        if (showType == "today") {
            const today = new Date();
            today.setUTCHours(0, 0, 0, 0);
            return today.toISOString().replace(/\.\d{3}Z$/, "Z");
        }
        if (showType == "last hour") {
            const today = new Date();
            today.setUTCHours(today.getUTCHours() - 1, 0, 0, 0);
            return today.toISOString().replace(/\.\d{3}Z$/, "Z");
        }
        return "2021-01-01T00:00:00Z";
    };

    const loadAddresses = async () => {
        const order = newest ? "created" : "score";
        const since = showTypeToSince(showType);
        const response = await backendFetch(
            `/api/fancy/list?limit=1000&order=${order}&category=${selectedCategory}&since=${since}&free=${showFree}`,
            {
                method: "Get",
            },
        );
        const addresses = await response.json();

        setFancies(addresses);
    };

    const loadTotalHashes = async () => {
        const since = showTypeToSince(showType);

        const response = await backendFetch(`/api/fancy/total_hash?since=${since}`, {
            method: "Get",
        });
        const totalHash = await response.json();
        console.log("Total hashes: ", totalHash);
        setTotalHash(totalHash);
    };

    const displayDifficulty = (difficulty: number): string => {
        const units = ["", "kH", "MH", "GH", "TH", "PH", "EH", "ZH", "YH"];
        let unitIndex = 0;

        while (difficulty >= 1000 && unitIndex < units.length - 1) {
            difficulty /= 1000;
            unitIndex++;
        }

        const precision = difficulty < 10 ? 3 : difficulty < 100 ? 2 : 1;
        return difficulty.toFixed(precision) + units[unitIndex];
    };

    const displayTime = (extraLabel: string, seconds: number): string => {
        const units = [
            { label: "month", seconds: 2_592_000, cutoff: 604_800 * 9.9 }, // 30 days
            { label: "week", seconds: 604_800, cutoff: 86_400 * 9.9 }, // 7 days
            { label: "day", seconds: 86_400, cutoff: 3600 * 48 }, // 24 hours
            { label: "hour", seconds: 3_600, cutoff: 60 * 99 }, // 60 minutes
            { label: "minute", seconds: 60, cutoff: 99 }, // 60 seconds
            { label: "second", seconds: 1, cutoff: 0 },
        ];

        for (const unit of units) {
            if (seconds >= unit.cutoff) {
                const value = seconds / unit.seconds;
                const precision = value < 10 ? 2 : value < 100 ? 1 : 0;
                return `${value.toFixed(precision)} ${extraLabel}${unit.label}${value >= 2 ? "s" : ""}`;
            }
        }

        return "0 seconds"; // Edge case for 0 input
    };

    const displayDifficultyAndGpuTime = (score: number): string => {
        const difficulty = score;
        const gpuTime = score / 1_000_000;
        let mhs = 1;
        const extraLabel = "GPU ";
        if (gpu === "RTX 3060") {
            mhs = 350;
        } else if (gpu === "RTX 3090") {
            mhs = 990;
        } else if (gpu === "RTX 4090") {
            mhs = 2300;
        }
        return `${displayDifficulty(difficulty)} (${displayTime(extraLabel, gpuTime / mhs)})`;
    };
    const displayEstimatedCost = (score: number): string => {
        const gpuTime = score / 1_000_000 / 350;

        const cost = (gpuTime / 3600) * 0.1;
        return `$${cost.toFixed(2)}`;
    };

    useEffect(() => {
        loadTotalHashes().then();
    }, [showType]);

    useEffect(() => {
        loadAddresses().then();
    }, [selectedCategory, newest, showType]);

    if (!fancies) {
        return <div>Loading...</div>;
    }
    if (!totalHash) {
        return <div>Loading...</div>;
    }
    if (!selectedCategory) {
        return <div>Loading...</div>;
    }

    return (
        <div>
            <h1>Browse Addresses</h1>
            <Select variant={"outlined"} defaultValue={showType} onChange={(e) => setShowType(e.target.value)}>
                <MenuItem value={"today"}>Today</MenuItem>
                <MenuItem value={"last hour"}>Last hour</MenuItem>
                <MenuItem value={"all"}>All</MenuItem>
            </Select>
            <Select variant={"outlined"} defaultValue={showFree} onChange={(e) => setShowFree(e.target.value)}>
                <MenuItem value={"free"}>Free</MenuItem>
                <MenuItem value={"reserved"}>Reserved</MenuItem>
                <MenuItem value={"all"}>All</MenuItem>
                <MenuItem value={"my"}>My</MenuItem>
            </Select>
            <FormControlLabel
                label="Show newest"
                control={<Checkbox value={newest} onChange={(e) => setNewest(e.target.checked)}></Checkbox>}
            ></FormControlLabel>

            <div>
                <h2>Estimated total work: {displayDifficultyAndGpuTime(totalHash.estimatedWorkTH * 1e12)}</h2>
            </div>

            <Select variant={"outlined"} defaultValue={gpu} onChange={(e) => setGpu(e.target.value)}>
                <MenuItem value={"RTX 3060"}>RTX 3060</MenuItem>
                <MenuItem value={"RTX 3090"}>RTX 3090</MenuItem>
                <MenuItem value={"RTX 4090"}>RTX 4090</MenuItem>
            </Select>

            <SelectCategory
                selectedCategory={selectedCategory}
                setSelectedCategory={setSelectedCategory}
            ></SelectCategory>

            <table>
                <thead>
                    <tr>
                        <th>Address</th>
                        <th>Short Etherscan</th>
                        <th>Difficulty</th>
                        <th>Price</th>
                        <th>Est. cost</th>
                        <th>Category</th>
                        <th>Created</th>
                        <th>Miner</th>
                        <th>Provider</th>
                    </tr>
                </thead>
                <tbody>
                    {fancies.map((fancy) => {
                        //0xa27CEF8a...Ae96F491F
                        //0xd2674dA9...211369A4B
                        //0x00000000...072C22734
                        //0x0000000000001fF3684f28c67538d4D072C22734
                        //0x000000000000001d48ffbd0c0da7c129137a9c55

                        const mixedCaseForm = ethers.getAddress(fancy.address);
                        const etherscanForm = mixedCaseForm.slice(0, 10) + "..." + mixedCaseForm.slice(33);
                        return (
                            <tr key={fancy.address}>
                                <td>
                                    <a className={"fancy-address-entry"} href={`/dashboard/address/${mixedCaseForm}`}>
                                        {mixedCaseForm}
                                    </a>
                                </td>
                                <td>
                                    <span className={"fancy-address-entry"}>{etherscanForm}</span>
                                </td>

                                <td>{displayDifficultyAndGpuTime(fancy.score)}</td>
                                <td>{fancy.price}</td>
                                <td>{displayEstimatedCost(fancy.score)}</td>
                                <td>{fancy.category}</td>
                                <td>{fancy.created}</td>
                                <td>{fancy.provName}</td>
                                <td>
                                    <a
                                        href={`https://stats.golem.network/network/provider/${fancy.provNodeId}?node_id=${fancy.provRewardAddr}`}
                                    >
                                        Golem Stats: {fancy.provName}
                                    </a>
                                </td>
                            </tr>
                        );
                    })}
                </tbody>
            </table>
        </div>
    );
};

export default BrowseAddresses;
