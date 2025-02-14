use crate::err_custom_create;
use crate::error::AddressologyError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolidityBytecode {
    pub object: String,
    pub opcodes: String,
    pub source_map: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SolidityEvm {
    #[serde(rename = "bytecode")]
    pub bytecode: SolidityBytecode,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SolidityContract {
    pub metadata: String,
    pub evm: SolidityEvm,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoliditySourceFile(BTreeMap<String, SolidityContract>);

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolidityError {
    pub message: String,
    pub severity: String,
    pub formatted_message: String,
    #[serde(rename = "type")]
    pub typ: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolidityJsonResponse {
    pub errors: Option<Vec<SolidityError>>,
    pub contracts: Option<BTreeMap<String, SoliditySourceFile>>,
}

pub async fn compile_solc(
    sources: BTreeMap<String, String>,
    solidity_version: &str,
) -> Result<SolidityJsonResponse, AddressologyError> {
    let bin = if solidity_version == "0.8.28" {
        if cfg!(target_os = "linux") {
            "/addressology/bin/solc_0.8.28"
        } else if cfg!(target_os = "windows") {
            ".\\solc-windows.exe"
        } else {
            return Err(err_custom_create!(
                "Unsupported solidity version: {}",
                solidity_version
            ));
        }
    } else {
        return Err(err_custom_create!(
            "Unsupported solidity version: {}",
            solidity_version
        ));
    };

    let mut cmd = match tokio::process::Command::new(bin)
        .arg("--standard-json")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(cmd) => cmd,
        Err(err) => return Err(err_custom_create!("Error starting solc: {} {}", bin, err)),
    };
    let sol_input_json = r#"
{
    "language": "Solidity",
    "sources": {
    },
    "settings": {
        "outputSelection": {
            "*": {
                "*": [
                    "metadata",
                    "evm.bytecode"
                ]
            }
        }
    }
}
"#;

    let mut sol_input_json = serde_json::from_str::<serde_json::Value>(sol_input_json)
        .map_err(|err| err_custom_create!("Error parsing solc input json: {}", err))?;

    for (source_name, source_code) in sources {
        log::info!("Compiling source: {}", source_name);
        sol_input_json["sources"][source_name]["content"] = serde_json::Value::String(source_code);
    }

    {
        let stdin = cmd
            .stdin
            .as_mut()
            .ok_or_else(|| err_custom_create!("Error getting stdin"))?;

        stdin
            .write_all(sol_input_json.to_string().as_bytes())
            .await
            .map_err(|err| err_custom_create!("Error writing to stdin: {}", err))?;
    }
    let output = cmd
        .wait_with_output()
        .await
        .map_err(|err| err_custom_create!("Error waiting for solc: {}", err))?;

    #[allow(clippy::match_single_binding)]
    match output {
        std::process::Output {
            status,
            stdout,
            stderr,
        } => {
            if !status.success() {
                return Err(err_custom_create!(
                    "Error compiling solidity code: {}",
                    String::from_utf8_lossy(&stderr)
                ));
            }

            log::info!("{}", String::from_utf8_lossy(&stdout));
            match serde_json::from_slice::<SolidityJsonResponse>(stdout.as_slice()) {
                Ok(json) => {
                    if let Some(_errors) = &json.errors {
                        log::info!("Solidity compilation failed");
                    } else if let Some(contracts_map) = &json.contracts {
                        for contract_name in contracts_map.keys() {
                            log::info!("Successfully compiled contract: {}", contract_name);
                        }
                    } else {
                        log::info!("No contracts found in solc output");
                    }
                    Ok(json)
                }
                Err(err) => Err(err_custom_create!("Error parsing solc output: {}", err)),
            }
        }
    }
}

//experimental - not working
/*pub async fn compile_solc_in_docker_not_working() -> Result<(), AddressologyError> {
    // Initialize the Docker client
    let docker = Docker::connect_with_local_defaults().map_err(
        |err| err_custom_create!("Error connecting to Docker: {}", err)
    )?;

    let image_base_name = "ethereum/solc";
    let image_tag = "0.8.28";
    let image_full_name = format!("{}:{}", image_base_name, image_tag);

    //download image
    docker.create_image(
        Some(CreateImageOptions {
            from_image: image_full_name.clone(),
            ..Default::default()
        }),
        None,
        None,
    ).try_for_each(|ev| async {
        match ev { CreateImageInfo { .. } => {} }
        Ok(())
    }).await.map_err(
        |err| err_custom_create!("Error creating image: {}", err)
    )?;


    // Define the container configuration

    //gen random container name

    let config = Config {
        image: Some(image_full_name.clone()), // Use a lightweight base image
        cmd: Some(vec!["--standard-json".to_string()]), // Run a command that will keep the container running
        attach_stdin: Some(true),
        attach_stdout: Some(true),
        host_config: Some(HostConfig {
            auto_remove: Some(true), // Automatically remove the container when stopped
            ..Default::default()
        }),
        ..Default::default()
    };

    let container_name = uuid::Uuid::new_v4().to_string();
    // Create the container
    docker
        .create_container(Some(CreateContainerOptions {
            name: container_name.clone(),
            platform: None,
        }), config)
        .await.map_err(
            |err| err_custom_create!("Error creating container: {}", err)
        )?;

    // Start the container
    docker
        .start_container(&container_name, None::<StartContainerOptions<String>>)
        .await.map_err(
            |err| err_custom_create!("Error starting container: {}", err)
        )?;

    // Attach to the container's stdin
    let attach_options: AttachContainerOptions<String> = AttachContainerOptions {
        stream: Some(true),
        stdin: Some(true),
        stdout: Some(true),
        stderr: Some(true),
        ..Default::default()
    };

    let mut attach_stream = docker.attach_container(&container_name, Some(attach_options)).await.map_err(
        |err| err_custom_create!("Error attaching to container: {}", err)
    )?;

    log::info!("Attached to container, writing to input");
    attach_stream.input.write_all(b"// SPDX-License-Identifier: UNLICENSED\npragma solidity ^0.8.28;\n\n// Uncomment this line to use console.log\n// import \"hardhat/console.sol\";\n\ncontract Lock {\n    uint public unlockTime;\n    address payable public owner;\n\n    event Withdrawal(uint amount, uint when);\n\n    constructor(uint _unlockTime) payable {\n        require(\n            block.timestamp < _unlockTime,\n            \"Unlock time should be in the future\"\n        );\n\n        unlockTime = _unlockTime;\n        owner = payable(msg.sender);\n    }\n\n    function withdraw() public {\n        // Uncomment this line, and the import of \"hardhat/console.sol\", to print a log in your terminal\n        // console.log(\"Unlock time is %o and block timestamp is %o\", unlockTime, block.timestamp);\n\n        require(block.timestamp >= unlockTime, \"You can't withdraw yet\");\n        require(msg.sender == owner, \"You aren't the owner\");\n\n        emit Withdrawal(address(this).balance, block.timestamp);\n\n        owner.transfer(address(this).balance);\n    }\n}\n\n").await.map_err(
        |err| err_custom_create!("Error writing to container stdin: {}", err)
    )?;
    log::info!("Wrote to input");

    attach_stream.output.try_for_each(
        |output| async {
            match output {
                LogOutput::StdErr { message } => {
                    log::error!("Container error: {}", String::from_utf8_lossy(&message));
                }
                LogOutput::StdOut { message } => {
                    println!("Container output: {}", String::from_utf8_lossy(&message));
                }
                LogOutput::StdIn { .. } => {
                }
                LogOutput::Console { .. } => {
                }
            }
            Ok(())
        }
    ).await.map_err(
        |err| err_custom_create!("Error reading from container stdout: {}", err)
    )?;

    Ok(())
}*/
