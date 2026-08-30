use crate::model::{money_with_currency, parse_decimal, parse_value};

pub fn parse(input: &str) {
    let _ = parse_value(input);
}

pub fn binary(input: &[u8]) {
    if let Ok(value) = serde_cbor::from_slice::<money_with_currency>(input) {
        let _ = value.canonical();
        let _ = value.as_money().to_string();
    }
}

pub fn amount(input: &str) {
    let _ = parse_decimal(input);
}
