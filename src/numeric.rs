use crate::errors::fail_parameter;
use crate::model::{money_with_currency, parse_decimal};
use pgrx::AnyNumeric;
use rust_decimal::Decimal;
use rusty_money::{Money, MoneyError, iso};

pub(crate) fn decimal_from_numeric(input: &AnyNumeric) -> Decimal {
    // PostgreSQL already owns the normalized output buffer. Parsing it directly
    // avoids allocating an intermediate Rust `String` on every numeric operation.
    parse_decimal(input.normalize()).unwrap_or_else(|error| fail_parameter(&error))
}

pub(crate) fn numeric_from_decimal(input: Decimal) -> AnyNumeric {
    AnyNumeric::try_from(input.normalize().to_string().as_str())
        .unwrap_or_else(|error| fail_parameter(&format!("could not produce numeric: {error}")))
}

pub(crate) fn unwrap_money(
    result: Result<Money<'static, iso::Currency>, MoneyError>,
) -> money_with_currency {
    result.map_or_else(
        |error| fail_parameter(&error.to_string()),
        money_with_currency::from_money,
    )
}
