use crate::catalog::Currency;
use crate::errors::fail_parameter;
use crate::model::{find_currency, money_with_currency};
use crate::numeric::{decimal_from_numeric, unwrap_money};
use pgrx::datetime::TimestampWithTimeZone;
use pgrx::prelude::*;
use pgrx::{AnyNumeric, PgRelation};
use rust_decimal::Decimal;
use rusty_money::{ExchangeRate, FormattableCurrency};

#[pg_extern(immutable, parallel_safe)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "pgrx SQL functions receive owned decoded numeric datums"
)]
pub(crate) fn money_exchange(
    value: money_with_currency,
    target_currency: &str,
    rate: AnyNumeric,
) -> money_with_currency {
    let target = find_currency(target_currency).unwrap_or_else(|error| fail_parameter(&error));
    exchange_with_rate(value, target, decimal_from_numeric(&rate))
}

fn exchange_with_rate(
    value: money_with_currency,
    target: &'static Currency,
    rate: Decimal,
) -> money_with_currency {
    if target == value.currency_ref() {
        fail_parameter("source and target currencies must differ");
    }
    if rate <= Decimal::ZERO {
        fail_parameter("exchange rate must be positive");
    }
    let exchange_rate = ExchangeRate::new(value.currency_ref(), target, rate)
        .unwrap_or_else(|error| fail_parameter(&error.to_string()));
    unwrap_money(exchange_rate.convert(&value.as_money()))
}

fn qualified_relation(relation: &PgRelation) -> String {
    pgrx::spi::quote_qualified_identifier(relation.namespace(), relation.name())
}

#[pg_extern(stable)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "pgrx SQL functions receive an owned relation handle"
)]
pub(crate) fn money_exchange_at(
    value: money_with_currency,
    target_currency: &str,
    rate_table: PgRelation,
    as_of: default!(TimestampWithTimeZone, "CURRENT_TIMESTAMP"),
) -> money_with_currency {
    let target = find_currency(target_currency).unwrap_or_else(|error| fail_parameter(&error));
    let table = qualified_relation(&rate_table);
    let query = format!(
        "SELECT rate::numeric FROM {table} \
         WHERE from_currency = $1 AND to_currency = $2 AND valid_at <= $3 \
         ORDER BY valid_at DESC LIMIT 1"
    );
    let rate = Spi::get_one_with_args::<AnyNumeric>(
        &query,
        &[
            value.currency_code().into(),
            target.code().into(),
            as_of.into(),
        ],
    )
    .unwrap_or_else(|error| {
        fail_parameter(&format!(
            "invalid exchange-rate table (expected from_currency text, to_currency text, rate numeric, valid_at timestamptz): {error}"
        ))
    })
    .unwrap_or_else(|| {
        fail_parameter(&format!(
            "no exchange rate from {} to {} at or before {as_of}",
            value.currency_code(),
            target.code()
        ))
    });
    exchange_with_rate(value, target, decimal_from_numeric(&rate))
}
