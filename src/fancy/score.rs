use crate::config::get_base_difficulty;
use crate::db::model::{FancyScore, FancyScoreEntry};
use crate::fancy::address_to_mixed_case;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use web3::types::{Address, U256};

#[derive(Serialize, Deserialize, EnumIter, PartialEq, Eq, Debug, Clone, Default)]
pub enum FancyScoreCategory {
    LeadingZeroes,
    LeadingAny,
    LettersCount,
    LettersHeavy,
    NumbersOnly,
    ShortLeadingZeroes,
    ShortLeadingAny,
    SnakeScore,
    LeadingLetters,
    #[default]
    Random,
}

impl Display for FancyScoreCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FancyScoreCategory::LeadingZeroes => write!(f, "leading_zeroes"),
            FancyScoreCategory::LeadingAny => write!(f, "leading_any"),
            FancyScoreCategory::LettersCount => write!(f, "letters_count"),
            FancyScoreCategory::LettersHeavy => write!(f, "letters_heavy"),
            FancyScoreCategory::NumbersOnly => write!(f, "numbers_only"),
            FancyScoreCategory::ShortLeadingZeroes => write!(f, "short_leading_zeroes"),
            FancyScoreCategory::ShortLeadingAny => write!(f, "short_leading_any"),
            FancyScoreCategory::SnakeScore => write!(f, "snake_score"),
            FancyScoreCategory::LeadingLetters => write!(f, "leading_letters"),
            FancyScoreCategory::Random => write!(f, "random"),
        }
    }
}

impl FromStr for FancyScoreCategory {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "leading_zeroes" => Ok(FancyScoreCategory::LeadingZeroes),
            "leading_any" => Ok(FancyScoreCategory::LeadingAny),
            "letters_count" => Ok(FancyScoreCategory::LettersCount),
            "letters_heavy" => Ok(FancyScoreCategory::LettersHeavy),
            "numbers_only" => Ok(FancyScoreCategory::NumbersOnly),
            "short_leading_zeroes" => Ok(FancyScoreCategory::ShortLeadingZeroes),
            "short_leading_any" => Ok(FancyScoreCategory::ShortLeadingAny),
            "snake_score" => Ok(FancyScoreCategory::SnakeScore),
            "leading_letters" => Ok(FancyScoreCategory::LeadingLetters),
            "random" => Ok(FancyScoreCategory::Random),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct FancyCategoryInfo {
    pub key: String,
    pub name: String,
    pub description: String,
}

pub fn list_score_categories() -> Vec<FancyCategoryInfo> {
    let mut categories = Vec::new();

    for category in FancyScoreCategory::iter() {
        match category {
            FancyScoreCategory::LeadingZeroes => categories.push(FancyCategoryInfo {
                key: category.to_string(),
                name: "Leading Zeroes".to_string(),
                description: "The number of leading zeroes in the address.".to_string(),
            }),
            FancyScoreCategory::LeadingAny => categories.push(FancyCategoryInfo {
                key: category.to_string(),
                name: "Leading Any".to_string(),
                description: "The number of leading characters that are the same.".to_string(),
            }),
            FancyScoreCategory::ShortLeadingZeroes => categories.push(FancyCategoryInfo {
                key: category.to_string(),
                name: "Short Leading Zeroes".to_string(),
                description: "The number of leading zeroes in the address.".to_string(),
            }),
            FancyScoreCategory::ShortLeadingAny => categories.push(FancyCategoryInfo {
                key: category.to_string(),
                name: "Short Leading Any".to_string(),
                description: "The number of leading characters that are the same.".to_string(),
            }),
            FancyScoreCategory::LettersCount => categories.push(FancyCategoryInfo {
                key: category.to_string(),
                name: "Letters Count".to_string(),
                description:
                    "The number of letters in the address (only one type of cipher allowed)."
                        .to_string(),
            }),
            FancyScoreCategory::LettersHeavy => categories.push(FancyCategoryInfo {
                key: category.to_string(),
                name: "Letters Heavy".to_string(),
                description: "The number of letters in the address (ciphers can be different)."
                    .to_string(),
            }),
            FancyScoreCategory::NumbersOnly => categories.push(FancyCategoryInfo {
                key: category.to_string(),
                name: "Smallest decimal".to_string(),
                description: "Only cyphers, score determine by smallest decimal".to_string(),
            }),
            FancyScoreCategory::Random => categories.push(FancyCategoryInfo {
                key: category.to_string(),
                name: "Random".to_string(),
                description: "Randomness of the address.".to_string(),
            }),
            FancyScoreCategory::SnakeScore => categories.push(FancyCategoryInfo {
                key: category.to_string(),
                name: "Snake Score".to_string(),
                description: "The number of repeating characters in the address.".to_string(),
            }),
            FancyScoreCategory::LeadingLetters => categories.push(FancyCategoryInfo {
                key: category.to_string(),
                name: "Leading Letters".to_string(),
                description: "The number of leading letters case sensitive in the address."
                    .to_string(),
            }),
        }
    }
    categories
}

pub fn total_combinations(n: f64) -> f64 {
    16.0f64.powf(n)
}

// n choose k symbol combinations
pub fn combinations(n: i64, k: i64) -> f64 {
    let mut result = 1.0;
    for i in 0..k {
        result *= (n - i) as f64 / (i + 1) as f64;
    }
    result
}

//one number is accepted
pub fn exactly_letters_combinations(letters: u64, total: u64) -> f64 {
    if letters == total {
        return 6.0f64.powf(letters as f64);
    }
    6.0f64.powf(letters as f64) * combinations(total as i64, (total as i64 - letters as i64) * 10)
}

pub fn exactly_letters_combinations_difficulty(letters: u64, total: u64) -> f64 {
    if letters < 30 {
        return 1.0f64;
    }
    let mut combinations_total = 0.0f64;
    for i in letters..=total {
        combinations_total += exactly_letters_combinations(i, total);
    }
    total_combinations(total as f64) / combinations_total
}

pub fn exactly_letters_combinations_multiple_ciphers(letters: u64, total: u64) -> f64 {
    if letters == total {
        return 6.0f64.powf(letters as f64);
    }
    6.0f64.powf(letters as f64)
        * combinations(total as i64, total as i64 - letters as i64)
        * 10f64.powf((total - letters) as f64)
}

pub fn exactly_letters_combinations_multiple_ciphers_difficulty(letters: u64, total: u64) -> f64 {
    if letters < 30 {
        return 1.0f64;
    }
    let mut combinations_total = 0.0f64;
    for i in letters..=total {
        combinations_total += crate::fancy::exactly_letters_combinations_multiple_ciphers(i, total);
    }
    total_combinations(total as f64) / combinations_total
}

pub fn snake_combinations(snake: i64, total: u64) -> f64 {
    if snake < 10 {
        return 1.0f64;
    }
    let mut combinations_total = 0.0f64;
    for i in ((snake as u64)..=total).rev() {
        let curr_comb =
            16.0 * combinations(total as i64, i as i64) * 15.0f64.powf(total as f64 - i as f64);
        combinations_total += curr_comb;
        /*println!(
            "combinations_total: {} {} {} {}",
            i,
            curr_comb,
            combinations_total,
            total_combinations(total as f64) / combinations_total
        );*/
    }

    total_combinations(total as f64) / combinations_total
}

#[tokio::test]
async fn tx_test() {
    assert_eq!(combinations(40, 1), 40.0);
    assert_eq!(combinations(40, 2), 780.0);
    //all letters probability

    let all_combinations = 16.0f64.powf(40.0);
    assert_eq!(all_combinations, 1.461501637330903e48);

    let only_letters_combinations = 6.0f64.powf(40.0);
    assert_eq!(only_letters_combinations, 1.3367494538843734e31);

    let one_number_combinations = 6.0f64.powf(39.0) * combinations(40, 1) * 10f64.powf(1.0);
    assert_eq!(one_number_combinations, 8.911663025895824e32);

    assert_eq!((6.0f64 / 16.0).powf(40.0), 9.14641092243755e-18);
    //39 letters probability
}

#[allow(clippy::vec_init_then_push)]
pub fn score_fancy(address: Address) -> FancyScore {
    let mut score = FancyScore::default();

    score.address_lower_case = format!("{:#x}", address).to_lowercase();
    score.address_mixed_case = address_to_mixed_case(&address);
    score.address_short_etherscan =
        score.address_mixed_case[0..10].to_string() + "..." + &score.address_mixed_case[33..42];

    let mixed_address_str = score.address_mixed_case.trim_start_matches("0x");
    let address_str = format!("{:#x}", address);
    let address_str = address_str.trim_start_matches("0x");
    let short_address_str = score
        .address_short_etherscan
        .trim_start_matches("0x")
        .replace("...", "");
    let mut leading_zeroes = 0;
    for c in address_str.chars() {
        if c == '0' {
            leading_zeroes += 1;
        } else {
            break;
        }
    }

    let char_start = address_str.chars().next().unwrap();
    let mut leading_any = 0;
    for c in address_str.chars() {
        if c == char_start {
            leading_any += 1;
        } else {
            break;
        }
    }

    let mut leading_letters = 0;
    let mixed_char_start = mixed_address_str.chars().next().unwrap();
    if mixed_char_start.is_alphabetic() {
        for c in mixed_address_str.chars() {
            if c == mixed_char_start {
                leading_letters += 1;
            } else {
                break;
            }
        }
    }

    let mut allowed_cipher = 'a';
    let mut letters_only = 0;
    for c in address_str.chars() {
        if c.is_alphabetic() {
            letters_only += 1;
        } else if allowed_cipher == 'a' {
            allowed_cipher = c;
        } else {
            //cipher have to be the same
            if c != allowed_cipher {
                letters_only = 0;
                break;
            }
        }
    }
    let mut letters_heavy = 0;
    for c in address_str.chars() {
        if c.is_alphabetic() {
            letters_heavy += 1;
        }
    }

    let mut numbers_only = 0;
    for c in address_str.chars() {
        if c.is_numeric() {
            numbers_only += 1;
        }
    }

    let mut short_leading_zeroes = 0;
    for c in short_address_str.chars() {
        if c == '0' {
            short_leading_zeroes += 1;
        } else {
            break;
        }
    }

    let mut short_leading_any = 0;
    let char_start = short_address_str.chars().next().unwrap();
    for c in short_address_str.chars() {
        if c == char_start {
            short_leading_any += 1;
        } else {
            break;
        }
    }

    let mut snake_score: i64 = 0;
    let mut prev_char = address_str.chars().next().unwrap();
    for c in address_str.chars() {
        if c == prev_char {
            snake_score += 1;
        } else {
            prev_char = c;
        }
    }

    let mut score_entries = Vec::new();

    score_entries.push(FancyScoreEntry {
        category: FancyScoreCategory::Random,
        score: 1.0f64,
        difficulty: 1000.0f64,
    });

    //for leading zeroes difficulty is a chance to get the smallest number interpreted as hex number
    let difficulty_leading_zeroes = {
        let number = U256::from_str_radix(address_str, 16).unwrap() + U256::from(1);
        let max_number =
            U256::from_str_radix("0xffffffffffffffffffffffffffffffffffffffff", 16).unwrap();

        let u256_to_float = |u256: U256| -> f64 {
            let u256_str = u256.to_string();
            u256_str.parse::<f64>().unwrap()
        };
        let float_number = u256_to_float(number);
        let float_max_number = u256_to_float(max_number);
        float_max_number / float_number
    };
    let u256_to_float = |u256: U256| -> f64 {
        let u256_str = u256.to_string();
        u256_str.parse::<f64>().unwrap()
    };
    let difficulty_leading_any = {
        let max_number = U256::from_str_radix("0x1111111111111111111111111111111111111111", 16)
            .unwrap()
            / U256::from(2);
        let mut min_difference = max_number;
        for i in [
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
        ]
        .iter()
        {
            let full_str = i.to_string().repeat(40);
            let ideal_number = U256::from_str_radix(&full_str, 16).unwrap();

            let current_number = U256::from_str_radix(address_str, 16).unwrap();

            let difference = if ideal_number >= current_number {
                ideal_number - current_number
            } else {
                current_number - ideal_number
            };
            if difference < min_difference {
                min_difference = difference;
            }
        }
        u256_to_float(max_number) / u256_to_float(min_difference + U256::from(1)) / 15.0
    };

    score_entries.push(FancyScoreEntry {
        category: FancyScoreCategory::LeadingZeroes,
        score: leading_zeroes as f64,
        difficulty: difficulty_leading_zeroes,
    });

    score_entries.push(FancyScoreEntry {
        category: FancyScoreCategory::LeadingAny,
        score: leading_any as f64 - 1.0_f64,
        difficulty: difficulty_leading_any,
    });

    score_entries.push(FancyScoreEntry {
        category: FancyScoreCategory::LettersCount,
        score: letters_only as f64,
        difficulty: exactly_letters_combinations_difficulty(letters_only, 40),
    });

    score_entries.push(FancyScoreEntry {
        category: FancyScoreCategory::LettersHeavy,
        score: letters_heavy as f64,
        difficulty: exactly_letters_combinations_multiple_ciphers_difficulty(letters_heavy, 40),
    });

    if numbers_only == 40 {
        let number = address_str.parse::<f64>().unwrap();
        let max_number = 9999999999999999999999999999999999999999f64;
        let difficulty1 =
            total_combinations(40.0) / 10.0f64.powf(numbers_only as f64) / (number / max_number);
        let difficulty2 = total_combinations(40.0)
            / 10.0f64.powf(numbers_only as f64)
            / ((max_number - number) / max_number);
        score_entries.push(FancyScoreEntry {
            category: FancyScoreCategory::NumbersOnly,
            score: numbers_only as f64,
            difficulty: difficulty1.max(difficulty2),
        });
    } else {
        score_entries.push(FancyScoreEntry {
            category: FancyScoreCategory::NumbersOnly,
            score: numbers_only as f64,
            difficulty: 1.0f64,
        });
    }
    let difficulty_short_leading_zeroes = {
        //important to add 1 to avoid division by zero and get proper result when address is exactly zero
        let number = U256::from_str_radix(&short_address_str, 16).unwrap() + U256::from(1);

        let max_number = U256::from_str_radix("0xfffffffffffffffff", 16).unwrap();

        let float_number = u256_to_float(number);
        let float_max_number = u256_to_float(max_number);
        float_max_number / float_number
    };

    score_entries.push(FancyScoreEntry {
        category: FancyScoreCategory::ShortLeadingZeroes,
        score: short_leading_zeroes as f64,
        difficulty: difficulty_short_leading_zeroes,
    });
    let short_difficulty_leading_any = {
        let max_number = U256::from_str_radix("0x11111111111111111", 16).unwrap() / U256::from(2);
        let mut min_difference = max_number;
        let current_number = U256::from_str_radix(&short_address_str, 16).unwrap() + U256::from(1);
        for i in [
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
        ]
        .iter()
        {
            let full_str = i.to_string().repeat(17);
            let ideal_number = U256::from_str_radix(&full_str, 16).unwrap();

            let difference = if ideal_number >= current_number {
                ideal_number - current_number
            } else {
                current_number - ideal_number
            };
            if difference < min_difference {
                min_difference = difference;
            }
        }
        u256_to_float(max_number) / u256_to_float(min_difference + U256::from(1)) / 15.0
    };
    score_entries.push(FancyScoreEntry {
        category: FancyScoreCategory::ShortLeadingAny,
        score: short_leading_any as f64,
        difficulty: short_difficulty_leading_any,
    });

    score_entries.push(FancyScoreEntry {
        category: FancyScoreCategory::SnakeScore,
        score: (snake_score - 1) as f64,
        difficulty: snake_combinations(snake_score - 1, 39),
    });
    score_entries.push(FancyScoreEntry {
        category: FancyScoreCategory::LeadingLetters,
        score: leading_letters as f64,
        difficulty: 32.0f64.powf(leading_letters as f64 - (15. / 16.)),
    });

    score.scores = score_entries
        .iter()
        .map(|entry| (entry.category.to_string(), entry.clone()))
        .collect();

    let neutral_price_point = get_base_difficulty();

    // This simple method is better than iterator, because of float NaN issues
    let mut biggest_score = score_entries[0].clone();
    for entry in score_entries.iter() {
        if entry.difficulty > biggest_score.difficulty {
            biggest_score = entry.clone();
        }
    }

    let biggest_score_difficulty = biggest_score.difficulty;

    let price_multiplier = if biggest_score_difficulty <= neutral_price_point {
        1.0
    } else {
        biggest_score_difficulty / neutral_price_point
    };

    score.total_score = biggest_score_difficulty;
    score.price_multiplier = price_multiplier;
    score.category = biggest_score.category.to_string();
    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use web3::types::Address;

    #[test]
    fn test_score_fancy() {
        let address = Address::from_str("0x99927777d11dDdFfFfF79b93bB00BBbB5fff5553").unwrap();
        let score = score_fancy(address);
        println!("{:?}", score);
    }
}
