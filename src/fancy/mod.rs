use web3::signing::keccak256;
use web3::types::H160;
#[allow(clippy::module_inception)]
mod fancy;
mod score;
pub use fancy::*;
pub use score::*;

fn address_to_mixed_case(address: &H160) -> String {
    let address_str = format!("{:x}", address);
    let hash = keccak256(address_str.as_bytes());
    let mut result = "0x".to_string();

    for (i, char) in address_str.chars().enumerate() {
        if char.is_ascii_hexdigit() {
            let hash_byte = hash[i / 2];
            let is_uppercase = if i % 2 == 0 {
                hash_byte >> 4 > 7
            } else {
                (hash_byte & 0x0f) > 7
            };
            if is_uppercase {
                result.push(char.to_ascii_uppercase());
            } else {
                result.push(char);
            }
        } else {
            result.push(char);
        }
    }

    result
}
