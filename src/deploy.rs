use crate::db::model::ContractDbObj;
use crate::db::ops::{fancy_get_by_address, update_contract_data};
use crate::error::AddressologyError;
use crate::types::DbAddress;
use crate::{err_custom_create, DeployData};
use sqlx::PgPool;

pub async fn handle_fancy_deploy(
    conn: &PgPool,
    contract: ContractDbObj,
) -> Result<(), AddressologyError> {
    let address = contract
        .address
        .clone()
        .ok_or_else(|| err_custom_create!("Address not found on db obj"))?;

    let address = DbAddress::from_str(&address)
        .map_err(|e| err_custom_create!("Failed to parse address: {}", e))?;

    let fancy = fancy_get_by_address(conn, address)
        .await
        .map_err(|_| err_custom_create!("Failed to get fancy address"))?
        .ok_or_else(|| err_custom_create!("Fancy address not found"))?;

    let deploy_data = serde_json::from_str::<DeployData>(&contract.data)
        .map_err(|e| err_custom_create!("Failed to parse deploy data: {}", e))?;

    let command = format!(
        "npx hardhat run deploy3Universal.ts --network {}",
        contract.network
    );
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

    let bytes =
        hex::decode(deploy_data.constructor_args.replace("0x", "").trim_ascii()).map_err(|e| {
            err_custom_create!(
                "Failed to decode constructor args: {}. Args provided: {}",
                e,
                deploy_data.constructor_args
            )
        })?;

    let total_bytes =
        "0x".to_string() + &deploy_data.contract.evm.bytecode.object + &hex::encode(bytes);

    let env_vars = vec![
        ("ADDRESS", format!("{:#x}", fancy.address.addr())),
        (
            "FACTORY",
            format!(
                "{:#x}",
                fancy
                    .factory
                    .ok_or_else(|| err_custom_create!(
                        "Factory not found on fancy address, it has to be there!"
                    ))?
                    .addr()
            ),
        ),
        ("SALT", fancy.salt.clone()),
        ("BYTECODE", total_bytes.clone()),
    ];

    log::info!("{:?}", env_vars);

    {
        let mut new_contract = contract.clone();
        new_contract.deploy_sent = Some(chrono::Utc::now().naive_utc());
        new_contract.deploy_status = crate::db::model::DeployStatus::TxSent;
        update_contract_data(conn, new_contract)
            .await
            .map_err(|e| err_custom_create!("Failed to update contract: {}", e))?;
    }

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
        let mut new_contract = contract.clone();
        new_contract.deployed = Some(chrono::Utc::now().naive_utc());
        new_contract.deploy_status = crate::db::model::DeployStatus::Succeeded;
        update_contract_data(conn, new_contract)
            .await
            .map_err(|e| err_custom_create!("Failed to update contract: {}", e))?;

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
