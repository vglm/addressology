import React, { useEffect, useState } from "react";

import "prismjs/components/prism-clike";
import "prismjs/components/prism-solidity";
import "prismjs/themes/prism.css";
import { backendFetch } from "./common/BackendCall";
import { Button, MenuItem, Select } from "@mui/material";
import { ethers } from "ethers"; //Example style, you can use another
import { CompilerMetadata, ContractCompiled } from "./model/Contract";
import "./CompiledContract.css";

interface CompiledContractProps {
    contract?: ContractCompiled;
}

const CompiledContractTemplate = (props: CompiledContractProps) => {
    const [network, _setNetwork] = useState("holesky");
    const [address, setAddress] = useState();
    const [bytecode, _setBytecode] = useState(props.contract?.contract.evm.bytecode.object ?? "");
    const [constructorArgs, setConstructorArgs] = useState("");
    const [networks, setNetworks] = useState<string[]>([]);

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
        getAddress().then();
        getNetworks().then(setNetworks);
    }, []);

    if (!props.contract) {
        return <div>No contract</div>;
    }

    const metadata = JSON.parse(props.contract.contract.metadata) as CompilerMetadata;

    const saveSourceCode = async () => {
        const bytecodeBytes = ethers.getBytes("0x" + bytecode.replace("0x", ""));
        const constructorArgsBytes = ethers.getBytes("0x" + constructorArgs.replace("0x", ""));

        const response = await backendFetch("/api/contract/new", {
            method: "Post",
            body: JSON.stringify({
                data: JSON.stringify({
                    bytecode: ethers.hexlify(bytecodeBytes),
                    constructorArgs: ethers.hexlify(constructorArgsBytes),
                    sourceCode: props.contract?.contract.singleFileCode ?? "",
                    metadata: JSON.stringify(metadata),
                    name: props.contract?.name ?? "",
                }),
                network: network,
                address: null,
            }),
        });
        const deploy = await response.json();
        console.log(deploy);
    };

    return (
        <div>
            <h3>Contract info</h3>
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
                value={props.contract.contract.singleFileCode}
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
            <Button onClick={(_e) => saveSourceCode()}>Save to</Button>
            <Select
                variant={"filled"}
                value={network}
                onChange={(e) => _setNetwork(e.target.value)}
                style={{ width: "100px" }}
            >
                {networks.map((network) => (
                    <MenuItem key={network} value={network}>
                        {network}
                    </MenuItem>
                ))}
            </Select>
            <div style={{ height: 300 }}>Empty</div>
        </div>
    );
};

export default CompiledContractTemplate;
