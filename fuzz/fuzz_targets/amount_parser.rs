#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| pg_money::fuzz_api::amount(data));
