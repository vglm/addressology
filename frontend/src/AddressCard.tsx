import React, { useEffect, useState } from "react";
import { backendFetch } from "./common/BackendCall";

import "./AddressCard.css";
import { ethers } from "ethers";
import { useParams } from "react-router-dom";
import { FancyCategoryInfo, FancyScore } from "./model/Fancy";
import { Button } from "@mui/material";
import { SelectCategory } from "./BrowseAddresses";

interface AddressCardProps {
    initialAddress: string;
}

const AddressCard = (props: AddressCardProps) => {
    const [fancy, setFancy] = useState<FancyScore | null>(null);
    const [address, setAddress] = useState<string>(props.initialAddress);
    const [categories, setCategories] = useState<FancyCategoryInfo[] | null>(null);
    const [randomCategory, setRandomCategory] = useState<string>("all");
    const [token, setToken] = useState<number>(0);

    const loadCategories = async () => {
        const response = await backendFetch("/api/fancy/categories", {
            method: "Get",
        });
        const addresses = await response.json();

        setCategories(addresses);
    };

    const reserveAddress = async () => {
        const response = await backendFetch(`/api/fancy/buy/${address}`, {
            method: "Post",
        });
        const reserve = await response.text();

        console.log(reserve);

        setToken(token + 1);
    };

    const getRandomAddress = async () => {
        const response = await backendFetch(`/api/fancy/random?category=${randomCategory}`, {
            method: "Get",
        });
        const address = await response.json();
        setAddress(address.address);
    };

    const loadFancy = async (addr: string) => {
        try {
            const parsedAddress = ethers.getAddress(addr.toLowerCase());
            const response = await backendFetch(`/api/fancy/score/${parsedAddress}`, {
                method: "Get",
            });
            const scoreResp = await response.json();

            setFancy(scoreResp);
        } catch (e) {
            console.error(e);
        }
    };

    useEffect(() => {
        loadCategories().then();
        loadFancy(address).then();
    }, [address, token]);

    if (!fancy) {
        return <div>Loading...</div>;
    }
    if (!categories) {
        return <div>Loading...</div>;
    }

    const addressCategory = categories.find((category) => category.key === fancy.score.category);
    if (!addressCategory) {
        return <div>Category not found...</div>;
    }
    const scoreInfo = fancy.score.scores[fancy.score.category];

    return (
        <div className={"address-card"}>
            <h1>Address card</h1>

            <div>Full address:</div>
            <div className={"address-card-address-entry-box"}>
                <span className={"address-card-address-entry"}>{fancy.score.addressMixedCase}</span>
            </div>
            <div>Shortened address:</div>
            <div className={"address-card-address-entry-box"}>
                <span className={"address-card-address-entry"}>{fancy.score.addressShortEtherscan}</span>
            </div>
            <div>Unique in category</div>
            <div className={"address-card-address-entry-box"}>
                <span>{addressCategory.name}</span>: <span>{scoreInfo.score}</span>
            </div>
            <div>Score</div>
            <div className={"address-card-address-entry-box"}>{fancy.score.totalScore}</div>
            <div>Reservation price</div>
            <div className={"address-card-address-entry-box"}>{fancy.price}</div>
            <div>Mined by</div>
            <div className={"address-card-address-entry-box"}>{fancy.miner}</div>
            <div>Mined at</div>
            <div className={"address-card-address-entry-box"}>{fancy.mined}</div>

            <SelectCategory selectedCategory={randomCategory} setSelectedCategory={setRandomCategory}></SelectCategory>
            <Button onClick={(_e) => getRandomAddress()}>Next random</Button>

            <Button onClick={(_e) => reserveAddress()}>Reserve</Button>
        </div>
    );
};

export const AddressCardForRoute = () => {
    const { address } = useParams();

    if (!address) {
        return <div>No address</div>;
    }

    return <AddressCard initialAddress={address} />;
};
