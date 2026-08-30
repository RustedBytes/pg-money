use crate::catalog::{self, Currency, CurrencyKind};
use crate::errors::fail_parameter;
use crate::model::find_currency;
use pgrx::JsonB;
use pgrx::prelude::*;
use rusty_money::{FormattableCurrency, Locale};
use serde_json::json;

type CurrencyRow = (
    name!(code, &'static str),
    name!(numeric_code, &'static str),
    name!(exponent, i32),
    name!(minor_units, i64),
    name!(name, &'static str),
    name!(symbol, &'static str),
    name!(locale, &'static str),
    name!(symbol_first, bool),
);

const fn locale_name(locale: Locale) -> &'static str {
    match locale {
        Locale::EnUs => "en-us",
        Locale::EnIn => "en-in",
        Locale::EnEu => "en-eu",
        Locale::EnBy => "en-by",
    }
}

fn currency_row(currency: &'static Currency) -> CurrencyRow {
    (
        currency.code(),
        currency
            .numeric_code()
            .expect("ISO currency must have a numeric code"),
        i32::try_from(currency.exponent()).expect("currency exponent fits in integer"),
        i64::try_from(currency.minor_units()).expect("minor-unit denominator fits in bigint"),
        currency.name(),
        currency.symbol(),
        locale_name(currency.locale()),
        currency.symbol_first(),
    )
}

type CryptoCurrencyRow = (
    name!(code, &'static str),
    name!(exponent, i32),
    name!(minor_units, i64),
    name!(name, &'static str),
    name!(symbol, &'static str),
    name!(locale, &'static str),
    name!(symbol_first, bool),
);

fn crypto_currency_row(currency: &'static Currency) -> CryptoCurrencyRow {
    (
        currency.code(),
        i32::try_from(currency.exponent()).expect("currency exponent fits in integer"),
        i64::try_from(currency.minor_units()).expect("minor-unit denominator fits in bigint"),
        currency.name(),
        currency.symbol(),
        locale_name(currency.locale()),
        currency.symbol_first(),
    )
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_currency_info(currency: &str) -> JsonB {
    let currency = find_currency(currency).unwrap_or_else(|error| fail_parameter(&error));
    JsonB(json!({
        "code": currency.code(),
        "kind": currency.kind().as_str(),
        "numeric_code": currency.numeric_code(),
        "exponent": currency.exponent(),
        "minor_units": currency.minor_units(),
        "name": currency.name(),
        "symbol": currency.symbol(),
        "locale": locale_name(currency.locale()),
        "symbol_first": currency.symbol_first(),
    }))
}

#[pg_extern(immutable, parallel_safe)]
#[allow(
    clippy::type_complexity,
    reason = "pgrx requires RETURNS TABLE column names inline in the Rust return type"
)]
pub(crate) fn money_currencies() -> TableIterator<
    'static,
    (
        name!(code, &'static str),
        name!(numeric_code, &'static str),
        name!(exponent, i32),
        name!(minor_units, i64),
        name!(name, &'static str),
        name!(symbol, &'static str),
        name!(locale, &'static str),
        name!(symbol_first, bool),
    ),
> {
    TableIterator::new(
        catalog::currencies()
            .iter()
            .filter(|currency| currency.kind() == CurrencyKind::Iso)
            .map(currency_row),
    )
}

#[pg_extern(immutable, parallel_safe)]
#[allow(
    clippy::type_complexity,
    reason = "pgrx requires RETURNS TABLE column names inline in the Rust return type"
)]
pub(crate) fn money_crypto_currencies() -> TableIterator<
    'static,
    (
        name!(code, &'static str),
        name!(exponent, i32),
        name!(minor_units, i64),
        name!(name, &'static str),
        name!(symbol, &'static str),
        name!(locale, &'static str),
        name!(symbol_first, bool),
    ),
> {
    TableIterator::new(
        catalog::currencies()
            .iter()
            .filter(|currency| currency.kind() == CurrencyKind::Crypto)
            .map(crypto_currency_row),
    )
}
