use crate::db::model::ContractDbObj;
use crate::db::ops::fancy_get_by_address;
use crate::error::AddressologyError;
use crate::types::DbAddress;
use crate::{err_custom_create, DeployData};
use sqlx::SqlitePool;

pub async fn handle_fancy_deploy(
    conn: &SqlitePool,
    contract: ContractDbObj,
) -> Result<(), AddressologyError> {
    let address = contract
        .address
        .ok_or_else(|| err_custom_create!("Address not found on db obj"))?;

    let address = DbAddress::from_str(&address)
        .map_err(|e| err_custom_create!("Failed to parse address: {}", e))?;

    let fancy = fancy_get_by_address(conn, address)
        .await
        .map_err(|_| err_custom_create!("Failed to get fancy address"))?
        .ok_or_else(|| err_custom_create!("Fancy address not found"))?;

    let deploy_data = serde_json::from_str::<DeployData>(&contract.data)
        .map_err(|e| err_custom_create!("Failed to parse deploy data: {}", e))?;

    let command = "npx hardhat run deploy3Universal.ts --network holesky";
    let command = if cfg!(windows) {
        format!("cmd /C {}", command)
    } else {
        command.to_string()
    };
    let current_dir = if cfg!(windows) {
        "C:/vglm/pretzel/locker"
    } else {
        "/addressology/pretzel/locker"
    };

    let args = if cfg!(windows) {
        command.split_whitespace().collect::<Vec<&str>>()
    } else {
        vec!["/bin/bash", "-c", &command]
    };

    let bytes = hex::decode(deploy_data.constructor_args.replace("0x", "").clone())
        .map_err(|e| err_custom_create!("Failed to decode constructor args: {}", e))?;

    let total_bytes = deploy_data.bytecode.clone() + &hex::encode(bytes);

    let env_vars = vec![
        ("ADDRESS", format!("{:#x}", fancy.address.addr())),
        ("FACTORY", format!("{:#x}", fancy.factory.addr())),
        ("SALT", fancy.salt.clone()),
        ("BYTECODE", total_bytes.clone()),
    ];

    log::info!("Running command: {:#?}", args);
    let cmd = tokio::process::Command::new(args[0])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .envs(env_vars)
        .current_dir(current_dir)
        .args(&args[1..])
        .spawn()
        .map_err(|e| err_custom_create!("Failed to spawn command: {}", e))?;

    let output = cmd
        .wait_with_output()
        .await
        .map_err(|e| err_custom_create!("Failed to wait for command: {}", e))?;
    let output_str = String::from_utf8_lossy(&output.stdout);
    log::info!("Command output: {}", output_str.to_string());
    if output.status.success() {
        Ok(())
    } else {
        log::error!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Err(err_custom_create!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }

    //run command
}
