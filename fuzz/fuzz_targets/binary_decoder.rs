#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| pg_money::fuzz_api::binary(data));
