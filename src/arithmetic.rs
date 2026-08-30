use crate::errors::fail_parameter;
use crate::model::money_with_currency;
use crate::numeric::decimal_from_numeric;
use pgrx::AnyNumeric;
use pgrx::prelude::*;
use rust_decimal::{Decimal, RoundingStrategy};
use rusty_money::MoneyError;

fn unwrap_value(result: Result<money_with_currency, MoneyError>) -> money_with_currency {
    result.unwrap_or_else(|error| fail_parameter(&error.to_string()))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_add(
    left: money_with_currency,
    right: money_with_currency,
) -> money_with_currency {
    unwrap_value(left.checked_add(right))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_subtract(
    left: money_with_currency,
    right: money_with_currency,
) -> money_with_currency {
    unwrap_value(left.checked_sub(right))
}

#[pg_extern(immutable, parallel_safe)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "pgrx SQL functions receive owned decoded numeric datums"
)]
pub(crate) fn money_multiply(
    value: money_with_currency,
    multiplier: AnyNumeric,
) -> money_with_currency {
    let multiplier = decimal_from_numeric(&multiplier);
    unwrap_value(value.checked_mul(multiplier))
}

#[pg_extern(immutable, parallel_safe)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "pgrx SQL functions receive owned decoded numeric datums"
)]
pub(crate) fn money_divide(value: money_with_currency, divisor: AnyNumeric) -> money_with_currency {
    let divisor = decimal_from_numeric(&divisor);
    unwrap_value(value.checked_div(divisor))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_abs(value: money_with_currency) -> money_with_currency {
    value.with_decimal(value.decimal().abs())
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_negate(value: money_with_currency) -> money_with_currency {
    value.with_decimal(-value.decimal())
}

#[derive(Debug, Copy, Clone, PostgresEnum)]
// pgrx uses the Rust identifier and variants as their SQL names.
#[allow(non_camel_case_types)]
pub enum money_rounding {
    half_up,
    half_down,
    half_even,
}

impl From<money_rounding> for RoundingStrategy {
    fn from(value: money_rounding) -> Self {
        match value {
            money_rounding::half_up => RoundingStrategy::MidpointAwayFromZero,
            money_rounding::half_down => RoundingStrategy::MidpointTowardZero,
            money_rounding::half_even => RoundingStrategy::MidpointNearestEven,
        }
    }
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_round(
    value: money_with_currency,
    digits: i32,
    strategy: default!(money_rounding, "'half_even'"),
) -> money_with_currency {
    if !(0..=Decimal::MAX_SCALE.cast_signed()).contains(&digits) {
        fail_parameter("digits must be between 0 and 28");
    }
    value.with_decimal(
        value
            .decimal()
            .round_dp_with_strategy(digits.cast_unsigned(), strategy.into()),
    )
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_split(value: money_with_currency, parts: i32) -> Vec<money_with_currency> {
    let parts = u32::try_from(parts)
        .ok()
        .filter(|parts| *parts > 0)
        .unwrap_or_else(|| fail_parameter("parts must be positive"));
    value
        .as_money()
        .split(parts)
        .unwrap_or_else(|error| fail_parameter(&error.to_string()))
        .into_iter()
        .map(money_with_currency::from_money)
        .collect()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_allocate(
    value: money_with_currency,
    weights: Vec<i32>,
) -> Vec<money_with_currency> {
    let weights = weights
        .into_iter()
        .map(|weight| {
            u32::try_from(weight)
                .ok()
                .filter(|weight| *weight > 0)
                .unwrap_or_else(|| fail_parameter("allocation weights must be positive integers"))
        })
        .collect::<Vec<_>>();
    value
        .as_money()
        .allocate(weights)
        .unwrap_or_else(|error| fail_parameter(&error.to_string()))
        .into_iter()
        .map(money_with_currency::from_money)
        .collect()
}
