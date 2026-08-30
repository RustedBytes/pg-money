use crate::errors::fail_parameter;
use crate::model::find_currency;
use pgrx::JsonB;
use pgrx::prelude::*;
use rusty_money::{FormattableCurrency, Locale, iso};
use serde_json::json;
use std::sync::OnceLock;

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

static ISO_CURRENCIES: OnceLock<Box<[&'static iso::Currency]>> = OnceLock::new();

const fn locale_name(locale: Locale) -> &'static str {
    match locale {
        Locale::EnUs => "en-us",
        Locale::EnIn => "en-in",
        Locale::EnEu => "en-eu",
        Locale::EnBy => "en-by",
    }
}

fn currency_row(currency: &'static iso::Currency) -> CurrencyRow {
    (
        currency.code(),
        currency.iso_numeric_code,
        i32::try_from(currency.exponent).expect("ISO currency exponent fits in integer"),
        i64::try_from(currency.minor_units).expect("ISO minor-unit denominator fits in bigint"),
        currency.name,
        currency.symbol,
        locale_name(currency.locale),
        currency.symbol_first,
    )
}

fn iso_currencies() -> &'static [&'static iso::Currency] {
    ISO_CURRENCIES.get_or_init(|| {
        let mut currencies = Vec::with_capacity(180);
        for first in b'A'..=b'Z' {
            for second in b'A'..=b'Z' {
                for third in b'A'..=b'Z' {
                    let bytes = [first, second, third];
                    // SAFETY: all bytes are uppercase ASCII and therefore valid UTF-8.
                    let code = unsafe { std::str::from_utf8_unchecked(&bytes) };
                    if let Some(currency) = iso::find(code) {
                        currencies.push(currency);
                    }
                }
            }
        }
        currencies.into_boxed_slice()
    })
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_currency_info(currency: &str) -> JsonB {
    let currency = find_currency(currency).unwrap_or_else(|error| fail_parameter(&error));
    JsonB(json!({
        "code": currency.code(),
        "numeric_code": currency.iso_numeric_code,
        "exponent": currency.exponent,
        "minor_units": currency.minor_units,
        "name": currency.name,
        "symbol": currency.symbol,
        "locale": locale_name(currency.locale),
        "symbol_first": currency.symbol_first,
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
    TableIterator::new(iso_currencies().iter().copied().map(currency_row))
}
