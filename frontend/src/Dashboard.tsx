import React, { useEffect, useState } from "react";

import { Routes, Route, Link, useNavigate, useLocation } from "react-router-dom";
import LoginScreen from "./LoginScreen";
import { LoginStatus, useLoginEvent, useLoginOrNull } from "./LoginProvider";
import { Avatar, Button, Menu, MenuItem } from "@mui/material";
import { backendFetch } from "./common/BackendCall";

import ChangePassScreen from "./ChangePassScreen";
import ContractFromSources from "./ContractFromSources";
import CompiledContract from "./CompiledContract";
import MyContracts from "./MyContracts";
import CompiledContractTemplate from "./CompiledContractTemplate";
import BrowseAddresses from "./BrowseAddresses";
import { UserTokenResponse } from "./model/Fancy";
import { AddressCardForRoute } from "./AddressCard";

const Dashboard = () => {
    const loginInformation = useLoginOrNull();
    const navigate = useNavigate();

    const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null);
    const urlParams = new URLSearchParams(window.location.search);
    const reset_token = urlParams.get("reset_token");

    const isLoggedIn = loginInformation.loginData != null;
    const [_logoutInProgress, setLogoutInProgress] = React.useState(false);
    const updateLogin = useLoginEvent();
    const [userTokens, setUserTokens] = useState<UserTokenResponse | null>(null);

    const loadUserTokens = async () => {
        const response = await backendFetch("/api/user/tokens", {
            method: "Get",
        });
        const userTokenResponse = await response.json();
        setUserTokens(userTokenResponse);
        console.log("User token response: ", userTokenResponse);
    };

    useEffect(() => {
        if (isLoggedIn) {
            loadUserTokens().then();
        }
    }, [isLoggedIn]);

    function getMarginLeft() {
        return Math.max((window.innerWidth - 1500) / 2, 15);
    }
    window.onresize = () => {
        const marginLeft = getMarginLeft();
        document.getElementsByClassName("main-page")[0].setAttribute("style", `margin-left: ${marginLeft}px`);
    };
    const logout = async () => {
        setLogoutInProgress(true);
        localStorage.clear();
        const response = await backendFetch("/api/logout", {
            method: "Post",
        });
        const data = await response.text();
        if (data === "Logged out") {
            updateLogin({
                loggedIn: null,
                loggedOut: true,
            });
        }

        window.location.href = "/";
        setLogoutInProgress(false);
    };

    const location = useLocation();

    if (loginInformation.loginData == null && loginInformation.loginStatus === LoginStatus.LOGIN_IN_PROGRESS) {
        return <div>{loginInformation.loginError}</div>;
    }
    const handleClick = (event: React.MouseEvent<HTMLButtonElement>) => {
        setAnchorEl(event.currentTarget);
    };
    const handleLogout = () => {
        logout().then();
        setAnchorEl(null);
    };
    const handleClose = () => {
        setAnchorEl(null);
    };
    const handleChangePass = () => {
        window.location.href = "/dashboard/change_pass";
        setAnchorEl(null);
    };

    const handleChangeOrg = (newOrg: string) => {
        localStorage.setItem("organization", newOrg);
        localStorage.removeItem("scans");
        localStorage.removeItem("address");

        window.location.href = "/dashboard";
    };
    const open = Boolean(anchorEl);
    const marginLeft = getMarginLeft();

    const compiledContractStr = localStorage.getItem("currentContract");
    let compiledContract = null;
    try {
        compiledContract = compiledContractStr ? JSON.parse(compiledContractStr) : null;
    } catch (e) {
        console.error(e);
    }

    const deleteTemplate = () => {
        localStorage.removeItem("currentContract");
        navigate("/");
    };
    return (
        <div className="main-page" style={{ marginLeft: marginLeft }}>
            <div className="top-header">
                {isLoggedIn && (
                    <>
                        <div className="top-header-navigation">
                            <Button
                                disabled={location.pathname === "/"}
                                onClick={() => {
                                    navigate("/");
                                }}
                            >
                                Create Contract
                            </Button>
                            <Button
                                disabled={location.pathname === "/contract"}
                                onClick={() => {
                                    navigate("/contract");
                                }}
                            >
                                Edit Contract
                            </Button>
                            <Button
                                disabled={location.pathname === "/template"}
                                onClick={() => {
                                    navigate("/template");
                                }}
                            >
                                {" "}
                                Template
                            </Button>
                            <Button
                                disabled={location.pathname === "/contracts"}
                                onClick={() => {
                                    navigate("/contracts");
                                }}
                            >
                                Contracts
                            </Button>
                            <Button
                                disabled={location.pathname === "/addresses"}
                                onClick={() => {
                                    navigate("/addresses");
                                }}
                            >
                                Addresses
                            </Button>
                            <Button
                                disabled={location.pathname === "/charts"}
                                onClick={() => {
                                    navigate("/charts");
                                }}
                            >
                                Charts
                            </Button>
                            <Button
                                disabled={location.pathname === "/charts2"}
                                onClick={() => {
                                    navigate("/charts2");
                                }}
                            >
                                Charts2
                            </Button>
                            <div className="filler"></div>
                            <div style={{ padding: 10 }}>Tokens: {userTokens?.tokens ?? "N/A"}</div>

                            {loginInformation.loginData ? (
                                <div className={"top-header-navigation-right"}>
                                    <Button
                                        id="basic-button"
                                        sx={{ padding: 0, margin: 0 }}
                                        aria-controls={open ? "basic-menu" : undefined}
                                        aria-haspopup="true"
                                        aria-expanded={open ? "true" : undefined}
                                        onClick={handleClick}
                                    >
                                        <Avatar sx={{ marginRight: 1 }}></Avatar>
                                        {loginInformation.loginData.email}
                                    </Button>
                                    <Menu
                                        id="basic-menu"
                                        anchorEl={anchorEl}
                                        open={open}
                                        onClose={handleClose}
                                        MenuListProps={{
                                            "aria-labelledby": "basic-button",
                                        }}
                                    >
                                        <MenuItem onClick={handleChangePass}>Change password</MenuItem>
                                        {localStorage.getItem("organization") !== "LIDO" ? (
                                            <MenuItem onClick={(_) => handleChangeOrg("LIDO")}>
                                                Change organization to LIDO
                                            </MenuItem>
                                        ) : (
                                            <MenuItem onClick={(_) => handleChangeOrg("golem.network")}>
                                                Change organization to golem.network
                                            </MenuItem>
                                        )}
                                        <MenuItem onClick={handleLogout}>Logout</MenuItem>
                                    </Menu>
                                </div>
                            ) : (
                                <Link to="/login">Login</Link>
                            )}
                        </div>
                    </>
                )}
            </div>
            <div className="main-content">
                <Routes>
                    <Route path="/" element={<div>{isLoggedIn && <ContractFromSources />}</div>} />
                    <Route path="/login" element={<div>{reset_token ? <ChangePassScreen /> : <LoginScreen />}</div>} />
                    <Route path="/change_pass" element={<div>{isLoggedIn && <ChangePassScreen />}</div>} />
                    <Route
                        path="/template"
                        element={
                            <div>
                                {isLoggedIn && (
                                    <CompiledContractTemplate contract={compiledContract} onDelete={deleteTemplate} />
                                )}
                            </div>
                        }
                    />
                    <Route path="/contracts" element={<div>{isLoggedIn && <MyContracts></MyContracts>}</div>} />
                    <Route path="/contract/:contractId" element={<div>{isLoggedIn && <CompiledContract />}</div>} />
                    <Route path="/address/:address" element={<div>{isLoggedIn && <AddressCardForRoute />}</div>} />
                    <Route path="/addresses" element={<div>{isLoggedIn && <BrowseAddresses></BrowseAddresses>}</div>} />
                </Routes>
            </div>
        </div>
    );
};

export default Dashboard;
