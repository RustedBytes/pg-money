use crate::errors::fail_parameter;
use crate::model::money_with_currency;
use crate::numeric::{decimal_from_numeric, unwrap_money};
use pgrx::datetime::TimestampWithTimeZone;
use pgrx::prelude::*;
use pgrx::{AnyNumeric, PgRelation};
use rust_decimal::Decimal;
use rusty_money::{ExchangeRate, iso};

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_exchange(
    value: money_with_currency,
    target_currency: &str,
    rate: AnyNumeric,
) -> money_with_currency {
    let target_code = target_currency.trim().to_ascii_uppercase();
    let target = iso::find(&target_code)
        .unwrap_or_else(|| fail_parameter(&format!("unknown ISO-4217 currency: {target_code}")));
    if target == value.currency_ref() {
        fail_parameter("source and target currencies must differ");
    }
    let rate = decimal_from_numeric(&rate);
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
pub(crate) fn money_exchange_at(
    value: money_with_currency,
    target_currency: &str,
    rate_table: PgRelation,
    as_of: default!(TimestampWithTimeZone, "CURRENT_TIMESTAMP"),
) -> money_with_currency {
    let target = target_currency.trim().to_ascii_uppercase();
    if iso::find(&target).is_none() {
        fail_parameter(&format!("unknown ISO-4217 currency: {target}"));
    }
    let table = qualified_relation(&rate_table);
    let query = format!(
        "SELECT rate::numeric FROM {table} \
         WHERE from_currency = $1 AND to_currency = $2 AND valid_at <= $3 \
         ORDER BY valid_at DESC LIMIT 1"
    );
    let rate = Spi::get_one_with_args::<AnyNumeric>(
        &query,
        &[
            value.currency.clone().into(),
            target.clone().into(),
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
            "no exchange rate from {} to {target} at or before {as_of}",
            value.currency
        ))
    });
    money_exchange(value, &target, rate)
}
