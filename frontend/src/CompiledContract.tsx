import React, { useEffect, useState } from "react";

import "prismjs/components/prism-clike";
import "prismjs/components/prism-solidity";
import "prismjs/themes/prism.css";
import { backendFetch } from "./common/BackendCall";
import { Button, MenuItem, Select } from "@mui/material";
import { ContractCompiled, ContractSaved } from "./model/Contract";
import "./CompiledContract.css";
import { useParams } from "react-router-dom";
import InputParameters from "./InputParameters";

const CompiledContract = () => {
    const [contractDetails, setContractDetails] = useState<ContractSaved | null>(null);
    const [contractName, setContractName] = useState("");
    const [network, setNetwork] = useState("holesky");
    const [networkCopyTo, setNetworkCopyTo] = useState("holesky");

    const [address, setAddress] = useState<string | null>(null);
    const [constructorArgs, setConstructorArgs] = useState("");
    const [networks, setNetworks] = useState<string[]>([]);
    const [bytecode, setBytecode] = useState<string | null>(null);
    const [metadata, setMetadata] = useState<any | null>(null);
    const [sourceCode, setSourceCode] = useState<string | null>(null);
    const { contractId } = useParams();

    const getContractDetails = async () => {
        const response = await backendFetch(`/api/contract/${contractId}`, {
            method: "Get",
        });
        const contract: ContractSaved = await response.json();
        console.log(contract);

        const data: ContractCompiled = JSON.parse(contract.data);

        setNetwork(contract.network);
        setBytecode(data.contract.evm.bytecode.object);
        setContractName(data.name);
        setConstructorArgs(data.constructorArgs);
        setMetadata(JSON.parse(data.contract.metadata));
        setSourceCode(data.contract.singleFileCode);

        setContractDetails(contract);
    };

    const getNetworks = async () => {
        return ["holesky", "amoy"];
    };

    const getAddress = async () => {
        const response = await backendFetch("/api/fancy/random", {
            method: "Get",
        });
        const address = await response.json();
        setAddress(address.address);
    };

    useEffect(() => {
        getContractDetails().then();

        getNetworks().then(setNetworks);
    }, []);

    const deploySourceCode = async () => {
        const response = await backendFetch(`/api/fancy/deploy/${contractId}`, {
            method: "Post",
        });
        const deploy = await response.json();
        console.log(deploy);
    };

    if (!contractDetails || !metadata || !bytecode) {
        return <div>No contract</div>;
    }

    const copyContract = async () => {
        const data: ContractCompiled = JSON.parse(contractDetails.data);

        const response = await backendFetch("/api/contract/new", {
            method: "Post",
            body: JSON.stringify({
                data: JSON.stringify(data),
                network: networkCopyTo,
                address: address,
            }),
        });
        const deploy = await response.json();
        console.log(deploy);
    };

    const saveChanges = async () => {
        const data: ContractCompiled = JSON.parse(contractDetails.data);
        const newContract: ContractSaved = {
            ...contractDetails,
            data: JSON.stringify(data),
            network: networkCopyTo,
            address: address,
        };

        const response = await backendFetch("/api/contract/update", {
            method: "Post",
            body: JSON.stringify(newContract),
        });
        const deploy = await response.json();
        console.log(deploy);
    };
    /*
    const metadata = JSON.parse(props.contract.metadata) as CompilerMetadata;

    const saveSourceCode = async () => {
        const bytecodeBytes = ethers.getBytes("0x" + bytecode.replace("0x", ""));
        const constructorArgsBytes = ethers.getBytes("0x" + constructorArgs.replace("0x", ""));
     */

    return (
        <div>
            <h3>Contract {contractName}</h3>
            <div>Address {address}</div>
            <Button onClick={(_e) => getAddress()}>Get Random Address</Button>
            <div>
                Compiler version: {metadata.language} - {metadata.compiler.version}
            </div>
            <div>
                Optimizer enabled:
                <span style={{ fontWeight: "bold" }}>
                    {metadata.settings.optimizer.enabled ? "true" : "false"}
                </span>{" "}
                runs:
                <span style={{ fontWeight: "bold" }}>{metadata.settings.optimizer.runs}</span>
            </div>
            <textarea
                value={bytecode}
                onChange={(_e) => {
                    console.log("Readonly");
                }}
                style={{
                    backgroundColor: "#f5f5f5",
                    border: "1px solid #ddd",
                    borderRadius: "5px",
                    boxShadow: "0 2px 4px rgba(0,0,0,0.1)",
                    fontSize: "14px",
                    lineHeight: "20px",
                    width: "100%",
                    height: "200px",
                }}
            ></textarea>
            ABI
            <textarea
                value={JSON.stringify(metadata.output.abi, null, 2)}
                style={{
                    backgroundColor: "#f5f5f5",
                    border: "1px solid #ddd",
                    borderRadius: "5px",
                    boxShadow: "0 2px 4px rgba(0,0,0,0.1)",
                    fontSize: "14px",
                    lineHeight: "20px",
                    width: "100%",
                    height: "200px",
                }}
            />
            Constructor binary data
            <textarea
                value={constructorArgs}
                onChange={(e) => setConstructorArgs(e.target.value)}
                style={{
                    backgroundColor: "#f5f5f5",
                    border: "1px solid #ddd",
                    borderRadius: "5px",
                    boxShadow: "0 2px 4px rgba(0,0,0,0.1)",
                    fontSize: "14px",
                    lineHeight: "20px",
                    width: "100%",
                    height: "200px",
                }}
            ></textarea>
            Source code
            <textarea
                value={sourceCode ?? ""}
                style={{
                    backgroundColor: "#f5f5f5",
                    border: "1px solid #ddd",
                    borderRadius: "5px",
                    boxShadow: "0 2px 4px rgba(0,0,0,0.1)",
                    fontSize: "14px",
                    lineHeight: "20px",
                    width: "100%",
                    height: "200px",
                }}
            ></textarea>
            <div>
                <Select
                    variant={"filled"}
                    value={networkCopyTo}
                    onChange={(e) => setNetworkCopyTo(e.target.value)}
                    style={{ width: "100px" }}
                >
                    {networks.map((network) => (
                        <MenuItem key={network} value={network}>
                            {network}
                        </MenuItem>
                    ))}
                </Select>
                <Button onClick={(_e) => copyContract()}>Copy to</Button>
                <Button onClick={(_e) => saveChanges()}>Save changes</Button>
            </div>
            <Select
                variant={"filled"}
                value={network}
                onChange={(e) => setNetwork(e.target.value)}
                style={{ width: "100px" }}
            >
                {networks.map((network) => (
                    <MenuItem key={network} value={network}>
                        {network}
                    </MenuItem>
                ))}
            </Select>
            <InputParameters
                abi={JSON.stringify(metadata.output.abi)}
                constructorArgs={constructorArgs}
                setConstructorArgs={setConstructorArgs}
            ></InputParameters>
            <Button onClick={(_e) => deploySourceCode()}>Deploy</Button>
            <div style={{ height: 300 }}>Empty</div>
        </div>
    );
};

export default CompiledContract;
