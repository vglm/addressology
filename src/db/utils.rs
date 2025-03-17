use chrono::{NaiveDateTime, Timelike};

// Postgres saves time with 6 digits of precision,
// to avoid wrong time comparison we need to round it to 3 digits
pub fn get_current_utc_time() -> NaiveDateTime {
    let current_time = chrono::Utc::now().naive_utc();
    let current_time_ns = current_time.nanosecond();
    //get rid of last 3 digits

    let current_time_ns = current_time_ns - current_time_ns % 1000;
    current_time.with_nanosecond(current_time_ns).unwrap()
}