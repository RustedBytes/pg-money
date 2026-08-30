use crate::errors::{fail_input, fail_parameter};
use crate::model::{STORAGE_VERSION, money_with_currency, parse_value};
use crate::numeric::{decimal_from_numeric, numeric_from_decimal};
use pgrx::prelude::*;
use pgrx::{AnyNumeric, JsonB};
use serde_json::json;

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_make(amount: AnyNumeric, currency: &str) -> money_with_currency {
    money_with_currency::from_decimal(decimal_from_numeric(&amount), currency)
        .unwrap_or_else(|error| fail_parameter(&error))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_parse(input: &str) -> money_with_currency {
    parse_value(input).unwrap_or_else(|error| fail_input(&error))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_try_parse(input: &str) -> Option<money_with_currency> {
    parse_value(input).ok()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_amount(value: money_with_currency) -> AnyNumeric {
    numeric_from_decimal(value.decimal())
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_currency(value: money_with_currency) -> String {
    value.currency
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_is_zero(value: money_with_currency) -> bool {
    value.as_money().is_zero()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_is_positive(value: money_with_currency) -> bool {
    value.as_money().is_positive()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_is_negative(value: money_with_currency) -> bool {
    value.as_money().is_negative()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_format(value: money_with_currency) -> String {
    value.as_money().to_string()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_to_json(value: money_with_currency) -> JsonB {
    let amount = value.canonical_amount();
    let formatted = value.as_money().to_string();
    JsonB(json!({
        "amount": amount,
        "currency": value.currency,
        "formatted": formatted,
    }))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_storage_version() -> i32 {
    STORAGE_VERSION.into()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_binary_format() -> &'static str {
    "cbor-array-v1"
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_extension_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pg_cast(immutable, parallel_safe)]
pub(crate) fn text_to_money_with_currency(input: &str) -> money_with_currency {
    money_parse(input)
}

#[pg_cast(assignment, immutable, parallel_safe)]
pub(crate) fn money_with_currency_to_text(value: money_with_currency) -> String {
    value.canonical()
}
