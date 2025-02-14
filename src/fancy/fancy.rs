use crate::config::get_base_difficulty_price;
use crate::db::model::FancyDbObj;
use crate::err_custom_create;
use crate::error::AddressologyError;
use crate::fancy::score_fancy;
use crate::hash::compute_create3_command;
use crate::types::DbAddress;
use web3::types::Address;

pub fn parse_fancy(salt: String, factory: Address) -> Result<FancyDbObj, AddressologyError> {
    /*let censor = censor::Standard + censor::Zealous + censor::Sex;

    //get rid of any weird characters from miner string
    let provider_name_censored = miner_unfiltered.provider_name.map(|s| {
            let provider_name_filtered = s.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-' || *c == '_' || *c == ' ')
                .take(20)
                .collect::<String>();

            censor.censor(&provider_name_filtered)
        });
    */
    let address = compute_create3_command(&format!("{:#x}", factory), &salt)?;

    let address =
        DbAddress::from_str(&address).map_err(|_| err_custom_create!("Failed to parse address"))?;

    let score = score_fancy(address.addr());

    Ok(FancyDbObj {
        address,
        salt,
        factory: DbAddress::wrap(factory),
        created: chrono::Utc::now().naive_utc(),
        score: score.total_score,

        owner: None,
        price: (score.price_multiplier * get_base_difficulty_price() as f64) as i64,
        category: score.category,
        job: None,
    })
}

//test fancy

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_parse_fancy() {
        let salt = "0x9a07547b2ac4220006e585000000000000000000000000000000000000000000";
        let factory = Address::from_str("0x9E3F8eaE49E442A323EF2094f277Bf62752E6995").unwrap();
        //let miner = "shitty-miner v1.0.2";

        let result = parse_fancy(salt.to_string(), factory);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        //assert_eq!(parsed.miner, "****ty-miner v1.0.2");

        assert_eq!(
            format!("{:#x}", parsed.address.addr()),
            "0x31585b5cd5557777376822555552bb555ee18882"
        );
        println!("{:?}", parsed);
    }
}
