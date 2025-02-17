import React, { useMemo } from "react";
import { BytesLike, ethers } from "ethers";
import InputParameter from "./InputParameter";

interface InputParametersProps {
    abi: string;
    constructorBinary: string;
    setConstructorBinary: (binary: string) => void;
}

function extractConstructorDefinitionFromAbi(abiStr: string) {
    // Create an Interface from the ABI
    const abi = JSON.parse(abiStr);
    const contractInterface = new ethers.Interface(abi);

    contractInterface.forEachFunction((func) => {
        console.log("Function:", func);
    });

    console.log(contractInterface.deploy);

    return contractInterface.deploy;
}

export function decodeConstructorParameters(abiStr: string, binary: BytesLike) {
    const fragment = extractConstructorDefinitionFromAbi(abiStr);
    const decoded = ethers.AbiCoder.defaultAbiCoder().decode(fragment.inputs, binary);
    for (let idx = 0; idx < fragment.inputs.length; idx++) {
        console.log(fragment.inputs[idx].name, decoded[idx]);
    }
    return decoded;
}

export function encodeConstructorDefaults(abiStr: string) {
    const fragment = extractConstructorDefinitionFromAbi(abiStr);
    const binary = [];
    for (const input of fragment.inputs) {
        if (input.type === "uint256") {
            binary.push("0".repeat(64));
        } else if (input.type === "address") {
            binary.push("0".repeat(64));
        } else {
            throw new Error(`Unsupported type ${input.type}`);
        }
    }
    return binary.join("");
}

export function encodeConstructorParameters(abiStr: string, argStr: string) {
    const args = argStr.split(",");
    const fragment = extractConstructorDefinitionFromAbi(abiStr);
    if (args.length !== fragment.inputs.length) {
        throw new Error("Invalid number of arguments");
    }

    const binary = [];
    for (let idx = 0; idx < args.length; idx++) {
        const param = fragment.inputs[idx];
        if (param.type === "uint256") {
            binary.push(BigInt(args[idx]).toString(16).padStart(64, "0"));
        } else if (param.type === "address") {
            binary.push(BigInt(args[idx]).toString(16).padStart(64, "0"));
        } else {
            throw new Error(`Unsupported type ${param.type}`);
        }
    }
    return binary.join("");
}

const InputParameters = (props: InputParametersProps) => {
    const constructorAbi = useMemo(() => {
        return extractConstructorDefinitionFromAbi(props.abi);
    }, [props.abi]);

    const constrBinary = props.constructorBinary;

    const updateInput = (name: string, value: string) => {
        const newArgs = [];
        const params = [];
        decodedData = decodeConstructorParameters(props.abi, "0x" + constrBinary);
        let idx = 0;
        for (const input of constructorAbi.inputs ?? []) {
            params.push(input);
            if (input.name === name) {
                newArgs.push(value);
            } else {
                newArgs.push(decodedData[idx]);
            }
            idx += 1;
        }

        const binary = [];
        for (let idx = 0; idx < newArgs.length; idx++) {
            const param = params[idx];
            if (param.type === "uint256") {
                binary.push(BigInt(newArgs[idx]).toString(16).padStart(64, "0"));
            }
            if (param.type === "address") {
                binary.push(BigInt(newArgs[idx]).toString(16).padStart(64, "0"));
            }
        }
        try {
            decodeConstructorParameters(props.abi, "0x" + binary.join(""));
            //if decoding succeeded, update the binary
            props.setConstructorBinary(binary.join(""));
        } catch (e) {
            return;
        }
    };

    let decodedData: null | ethers.Result = null;
    try {
        decodedData = decodeConstructorParameters(props.abi, "0x" + constrBinary);
    } catch (e) {
        if (!decodedData) {
            return <div>Failed to decode data</div>;
        }
    }
    if (decodedData.length != constructorAbi.inputs.length) {
        return <div>Invalid number of arguments</div>;
    }

    return (
        <div>
            <h1>Input Parameters</h1>

            <div>
                <h2>Constructor Arguments</h2>
                <div>{constrBinary}</div>
                <table>
                    <thead>
                        <tr>
                            <th>Name</th>
                            <th>Type</th>
                            <th>Value</th>
                        </tr>
                    </thead>
                    <tbody>
                        {constructorAbi.inputs.map((input, idx) => {
                            const val = decodedData ? decodedData[idx] : "";
                            return (
                                <InputParameter
                                    key={input.name}
                                    name={input.name}
                                    type={input.type}
                                    value={val}
                                    setValue={(value) => updateInput(input.name, value)}
                                />
                            );
                        })}
                    </tbody>
                </table>
            </div>
        </div>
    );
};

export default InputParameters;
