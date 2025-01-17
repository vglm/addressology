import React, { useEffect, useState } from "react";

import "prismjs/components/prism-clike";
import "prismjs/components/prism-solidity";
import "prismjs/themes/prism.css";
import { backendFetch } from "./common/BackendCall";
import { Button, MenuItem, Select } from "@mui/material";
import { CompilerMetadata, ContractCompiled } from "./model/Contract";
import "./CompiledContract.css";

interface CompiledContractProps {
    contract?: ContractCompiled;

    onDelete: () => void;
}

const CompiledContractTemplate = (props: CompiledContractProps) => {
    const [network, _setNetwork] = useState("holesky");
    const [bytecode, _setBytecode] = useState(props.contract?.contract.evm.bytecode.object ?? "");
    const [constructorArgs, setConstructorArgs] = useState("");
    const [networks, setNetworks] = useState<string[]>([]);

    const getNetworks = async () => {
        return ["holesky", "amoy"];
    };

    useEffect(() => {
        getNetworks().then(setNetworks);
    }, []);

    if (!props.contract) {
        return <div>No contract</div>;
    }

    const metadata = JSON.parse(props.contract.contract.metadata) as CompilerMetadata;

    const saveSourceCode = async () => {
        const response = await backendFetch("/api/contract/new", {
            method: "Post",
            body: JSON.stringify({
                data: JSON.stringify(props.contract),
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
            <div>Address not set at this point</div>
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
            <Button onClick={(_e) => props.onDelete()}>Delete this template</Button>
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
