import React, { useEffect, useState } from "react";

import "prismjs/components/prism-clike";
import "prismjs/components/prism-solidity";
import "prismjs/themes/prism.css";
import { backendFetch } from "./common/BackendCall";
import { Button, MenuItem, Select } from "@mui/material";
import { ContractCompiled, ContractSaved } from "./model/Contract";
import "./CompiledContract.css";
import { useParams } from "react-router-dom";
import InputParameters, {
    decodeConstructorParameters,
    encodeConstructorDefaults,
} from "./InputParameters";
import { Fancy } from "./model/Fancy";

const CompiledContract = () => {
    const [contractDetails, setContractDetails] = useState<ContractSaved | null>(null);
    const [contractName, setContractName] = useState("");
    const [networkCopyTo, setNetworkCopyTo] = useState("holesky");

    const [availableAddresses, setAvailableAddresses] = useState<Fancy[]>([]);
    const [networks, setNetworks] = useState<string[]>([]);
    const [bytecode, setBytecode] = useState<string | null>(null);
    const [metadata, setMetadata] = useState<any | null>(null);
    const [sourceCode, setSourceCode] = useState<string | null>(null);
    const { contractId } = useParams();
    const [updateToken, setUpdateToken] = useState(0);

    const [constructorBinary, setConstructorBinary] = useState<string>("");

    const getContractDetails = async () => {
        const response = await backendFetch(`/api/contract/${contractId}`, {
            method: "Get",
        });
        const contract: ContractSaved = await response.json();
        console.log(contract);

        const data: ContractCompiled = JSON.parse(contract.data);

        setBytecode(data.contract.evm.bytecode.object);
        setContractName(data.name);
        setConstructorBinary(data.constructorArgs);
        setMetadata(JSON.parse(data.contract.metadata));
        setSourceCode(data.contract.singleFileCode);

        setContractDetails(contract);
    };

    const getNetworks = async () => {
        return ["holesky", "amoy"];
    };

    const searchAddresses = async () => {
        const response = await backendFetch("/api/fancy/list?free=mine", {
            method: "Get",
        });
        const addresses = await response.json();
        setAvailableAddresses(addresses);
    };

    useEffect(() => {
        if (contractDetails) {
            try {
                console.log("Trying to decode constructor parameters");
                const bytesLike = "0x" + constructorBinary;
                decodeConstructorParameters(JSON.stringify(metadata.output.abi), bytesLike);
            } catch (e) {
                console.error("Error decoding constructor parameters, setting defaults:", e);
                const defaults = encodeConstructorDefaults(JSON.stringify(metadata.output.abi));
                try {
                    const bytesLike = "0x" + defaults;
                    decodeConstructorParameters(JSON.stringify(metadata.output.abi), bytesLike);
                    setConstructorBinary(defaults);
                } catch (e) {
                    console.error("Error decoding defaults, setting empty");
                    setConstructorBinary("");
                }
            }
        }
    }, [contractDetails]);

    useEffect(() => {
        getContractDetails().then();

        getNetworks().then(setNetworks);
    }, [updateToken]);

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
                address: contractDetails.address,
            }),
        });
        const deploy = await response.json();
        console.log(deploy);
    };

    const assignAddress = async (address: string) => {
        console.log("Assigning address", address);
        const data: ContractCompiled = JSON.parse(contractDetails.data);
        const newContract: ContractSaved = {
            ...contractDetails,
            data: JSON.stringify(data),
            network: networkCopyTo,
            address,
        };

        const response = await backendFetch("/api/contract/update", {
            method: "Post",
            body: JSON.stringify(newContract),
        });
        const deploy = await response.json();
        console.log(deploy);
        setUpdateToken(updateToken + 1);
    };

    const saveChanges = async () => {
        const data: ContractCompiled = JSON.parse(contractDetails.data);
        data.constructorArgs = constructorBinary;
        const newContract: ContractSaved = {
            ...contractDetails,
            data: JSON.stringify(data),
            network: networkCopyTo,
            address: contractDetails.address,
        };

        const response = await backendFetch("/api/contract/update", {
            method: "Post",
            body: JSON.stringify(newContract),
        });
        const deploy = await response.json();
        console.log(deploy);
        setUpdateToken(updateToken + 1);
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
            <div>Address {contractDetails?.address ?? "Unassigned"}</div>
            <Button onClick={(_e) => searchAddresses()}>Assign address...</Button>
            <div>
                Addresses:
                {availableAddresses.map((fancy) => (
                    <div key={fancy.address}>
                        <div>{fancy.address}</div>
                        <button onClick={(_e) => assignAddress(fancy.address)}>Choose</button>
                    </div>
                ))}
            </div>
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
            <Button
                onClick={(_e) => setConstructorBinary(encodeConstructorDefaults(JSON.stringify(metadata.output.abi)))}
            >
                Set defaults
            </Button>
            <textarea
                value={constructorBinary}
                onChange={(e) => setConstructorBinary(e.target.value)}
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
            <InputParameters
                abi={JSON.stringify(metadata.output.abi)}
                constructorBinary={constructorBinary}
                setConstructorBinary={setConstructorBinary}
            ></InputParameters>
            <Button onClick={(_e) => deploySourceCode()}>Deploy</Button>
            <div style={{ height: 300 }}>Empty</div>
        </div>
    );
};

export default CompiledContract;
