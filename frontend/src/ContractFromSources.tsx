import React, { useState } from "react";

import Editor from "react-simple-code-editor";
import { highlight, languages } from "prismjs";
import "prismjs/components/prism-clike";
import "prismjs/components/prism-solidity";
import "prismjs/themes/prism.css";
import { backendFetch } from "./common/BackendCall";
import { Button } from "@mui/material";
import "./ContractFromSources.css";
import { CompileErrors, CompileResponse, ContractCompiled, ContractCompiledInt } from "./model/Contract";
import { useNavigate } from "react-router-dom";

const ContractFromSources = () => {
    //const loginInformation = useLoginOrNull();
    const navigate = useNavigate();
    const [errors, setErrors] = useState<CompileErrors[]>([]);
    const [contracts, setContracts] = useState<{ [key: string]: { [key: string]: ContractCompiledInt } }>({});
    const [code, setCode] = useState(
        "// SPDX-License-Identifier: UNLICENSED\n" +
            "pragma solidity ^0.8.28;\n" +
            "\n" +
            "// Uncomment this line to use console.log\n" +
            '// import "hardhat/console.sol";\n' +
            "\n" +
            "contract Lock {\n" +
            "    uint public unlockTime;\n" +
            "    address payable public owner;\n" +
            "\n" +
            "    event Withdrawal(uint amount, uint when);\n" +
            "\n" +
            "    constructor(uint _unlockTime) payable {\n" +
            "        require(\n" +
            "            block.timestamp < _unlockTime,\n" +
            '            "Unlock time should be in the future"\n' +
            "        );\n" +
            "\n" +
            "        unlockTime = _unlockTime;\n" +
            "        owner = payable(msg.sender);\n" +
            "    }\n" +
            "\n" +
            "    function withdraw() public {\n" +
            '        // Uncomment this line, and the import of "hardhat/console.sol", to print a log in your terminal\n' +
            '        // console.log("Unlock time is %o and block timestamp is %o", unlockTime, block.timestamp);\n' +
            "\n" +
            '        require(block.timestamp >= unlockTime, "You can\'t withdraw yet");\n' +
            '        require(msg.sender == owner, "You aren\'t the owner");\n' +
            "\n" +
            "        emit Withdrawal(address(this).balance, block.timestamp);\n" +
            "\n" +
            "        owner.transfer(address(this).balance);\n" +
            "    }\n" +
            "}\n" +
            "\n",
    );

    const compileSourceCode = async (sourceCode: string) => {
        const response = await backendFetch("/api/contract/compile", {
            method: "Post",
            body: JSON.stringify({
                sources: {
                    main: sourceCode,
                },
            }),
        });
        const compile: CompileResponse = await response.json();
        console.log(compile);
        if (compile.errors) {
            setErrors(compile.errors);
            console.log(compile.errors);
        } else {
            console.log(compile);
        }
        if (compile.contracts) {
            console.log(compile.contracts);
            setContracts(compile.contracts);
        }
    };

    function getMarginLeft() {
        return Math.max((window.innerWidth - 1500) / 2, 15);
    }
    window.onresize = () => {
        const marginLeft = getMarginLeft();
        document.getElementsByClassName("main-page")[0].setAttribute("style", `margin-left: ${marginLeft}px`);
    };

    if (!contracts) {
        return <div>No contract</div>;
    }
    const compiledContracts: { [key: string]: ContractCompiledInt } = {};
    for (const value of Object.values(contracts)) {
        for (const [key2, value2] of Object.entries(value)) {
            compiledContracts[key2] = value2;
        }
    }

    function selectContract(key: string, objWithSource: ContractCompiledInt) {
        const newObj: ContractCompiled = {
            name: key,
            constructorArgs: "0x",
            contract: objWithSource,
        };
        localStorage.setItem("currentContract", JSON.stringify(newObj));
        navigate("/template");
    }

    return (
        <div className="main-page">
            <div
                style={{
                    fontFamily: '"Fira code", "Fira Mono", monospace',
                }}
            >
                <div>
                    <h2>Contract from source code</h2>
                    <div className={"source-code"}>
                        <Editor
                            value={code}
                            onValueChange={(newCode) => setCode(newCode)}
                            padding={10}
                            tabSize={4}
                            highlight={(code) => highlight(code, languages.solidity, "Solidity")}
                            style={{
                                backgroundColor: "#f5f5f5",
                                border: "1px solid #ddd",
                                borderRadius: "5px",
                                boxShadow: "0 2px 4px rgba(0,0,0,0.1)",
                                fontSize: "14px",
                                lineHeight: "20px",
                            }}
                        />
                    </div>
                </div>
                <Button onClick={(_e) => compileSourceCode(code)}>Compile</Button>

                {errors.map((error, index) => (
                    <div key={index}>
                        {error.severity} {error.type} {error.message} {error.formattedMessage}
                    </div>
                ))}
                {Object.keys(compiledContracts).map((key, index) => {
                    const objWithSource = compiledContracts[key];
                    objWithSource.singleFileCode = code;
                    return (
                        <div key={key}>
                            {index + 1} : Successfully compiled contract {key}
                            <Button onClick={(_e) => selectContract(key, objWithSource)}>Select {key}</Button>
                        </div>
                    );
                })}
            </div>
        </div>
    );
};

export default ContractFromSources;
