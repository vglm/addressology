import React, { useEffect, useState } from "react";
import { ConstructorFragment, ethers } from "ethers";
import InputParameter from "./InputParameter";

interface InputParametersProps {
    abi: string;
    constructorArgs: string;
    setConstructorArgs: (args: string) => void;
}

function decodeConstructorParameters(abiStr: string) {
    // Create an Interface from the ABI
    const abi = JSON.parse(abiStr);
    const contractInterface = new ethers.Interface(abi);

    contractInterface.forEachFunction((func) => {
        console.log("Function:", func);
    });

    console.log(contractInterface.deploy);

    return contractInterface.deploy;
}

const InputParameters = (props: InputParametersProps) => {
    const [constructorArgs, setConstructorArgs] = useState<ConstructorFragment | null>(null);

    useEffect(() => {
        setConstructorArgs(decodeConstructorParameters(props.abi));
    }, []);

    return (
        <div>
            <h1>Input Parameters</h1>

            <div>
                <h2>Constructor Arguments</h2>
                <table>
                    <tr>
                        <th>Name</th>
                        <th>Type</th>
                        <th>Value</th>
                    </tr>
                    {constructorArgs?.inputs.map((input) => {
                        return (
                            <InputParameter
                                key={input.name}
                                name={input.name}
                                type={input.type}
                                value={props.constructorArgs}
                            />
                        );
                    })}
                </table>
            </div>
        </div>
    );
};

export default InputParameters;
