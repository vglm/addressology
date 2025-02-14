use std::env;
use std::str::FromStr;

fn get_env_int(key: &str, default: i64) -> i64 {
    env::var(key)
        .map(|s| i64::from_str(&s).unwrap())
        .unwrap_or(default)
}

fn get_env_float(key: &str, default: f64) -> f64 {
    env::var(key)
        .map(|s| f64::from_str(&s).unwrap())
        .unwrap_or(default)
}

pub fn get_base_difficulty() -> f64 {
    get_env_float("BASE_DIFFICULTY", 16.0f64.powf(9f64))
}

pub fn get_base_difficulty_price() -> i64 {
    get_env_int("BASE_DIFFICULTY_PRICE", 1000)
}
