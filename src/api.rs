use crate::errors::{fail_input, fail_parameter};
use crate::model::{STORAGE_VERSION, money_with_currency, parse_value};
use crate::numeric::{decimal_from_numeric, numeric_from_decimal};
use pgrx::prelude::*;
use pgrx::{AnyNumeric, JsonB};
use rusty_money::{Money, iso};
use serde_json::json;

#[pg_extern(immutable, parallel_safe)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "pgrx SQL functions receive owned decoded numeric datums"
)]
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
pub(crate) fn money_currency(value: money_with_currency) -> &'static str {
    value.currency_code()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_is_zero(value: money_with_currency) -> bool {
    value.decimal().is_zero()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_is_positive(value: money_with_currency) -> bool {
    let amount = value.decimal();
    amount.is_sign_positive() && !amount.is_zero()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_is_negative(value: money_with_currency) -> bool {
    let amount = value.decimal();
    amount.is_sign_negative() && !amount.is_zero()
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
        "currency": value.currency_code(),
        "formatted": formatted,
    }))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_to_rusty_json(value: money_with_currency) -> JsonB {
    JsonB(
        serde_json::to_value(value.as_money())
            .unwrap_or_else(|error| fail_parameter(&format!("could not serialize money: {error}"))),
    )
}

#[pg_extern(immutable, parallel_safe)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "pgrx SQL functions receive an owned jsonb datum"
)]
pub(crate) fn money_from_rusty_json(input: JsonB) -> money_with_currency {
    let value: Money<'static, iso::Currency> = serde_json::from_value(input.0)
        .unwrap_or_else(|error| fail_parameter(&format!("invalid rusty-money JSON: {error}")));
    money_with_currency::from_money(value)
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
