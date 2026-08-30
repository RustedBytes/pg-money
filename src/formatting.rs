use crate::errors::fail_parameter;
use crate::model::{MAX_INPUT_BYTES, find_currency, money_with_currency};
use pgrx::JsonB;
use pgrx::prelude::*;
use rusty_money::{FormattableCurrency, Formatter, LocalFormat, Money, Params, Position};
use serde::Deserialize;

const MAX_FORMAT_ITEMS: usize = 16;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FormatPosition {
    Space,
    Amount,
    Code,
    Symbol,
    Sign,
}

impl From<FormatPosition> for Position {
    fn from(value: FormatPosition) -> Self {
        match value {
            FormatPosition::Space => Self::Space,
            FormatPosition::Amount => Self::Amount,
            FormatPosition::Code => Self::Code,
            FormatPosition::Symbol => Self::Symbol,
            FormatPosition::Sign => Self::Sign,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FormatOptions {
    digit_separator: Option<String>,
    exponent_separator: Option<String>,
    separator_pattern: Option<Vec<usize>>,
    positions: Option<Vec<FormatPosition>>,
    rounding: Option<u32>,
    include_symbol: Option<bool>,
    include_code: Option<bool>,
}

fn single_character(value: Option<String>, default: char, field: &str) -> char {
    let Some(value) = value else {
        return default;
    };
    let mut characters = value.chars();
    let character = characters
        .next()
        .unwrap_or_else(|| fail_parameter(&format!("{field} must contain exactly one character")));
    if characters.next().is_some() {
        fail_parameter(&format!("{field} must contain exactly one character"));
    }
    character
}

fn parse_localized(input: &str, currency: &str) -> Result<money_with_currency, String> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(format!("input exceeds {MAX_INPUT_BYTES} bytes"));
    }
    if input.as_bytes().contains(&0) {
        return Err("input contains a NUL byte".to_owned());
    }
    let currency = find_currency(currency)?;
    Money::from_str(input, currency)
        .map(money_with_currency::from_money)
        .map_err(|error| error.to_string())
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_parse_localized(input: &str, currency: &str) -> money_with_currency {
    parse_localized(input, currency).unwrap_or_else(|error| fail_parameter(&error))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_try_parse_localized(
    input: &str,
    currency: &str,
) -> Option<money_with_currency> {
    parse_localized(input, currency).ok()
}

#[pg_extern(immutable, parallel_safe)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "pgrx SQL functions receive an owned jsonb datum"
)]
pub(crate) fn money_format_with(value: money_with_currency, options: JsonB) -> String {
    let options: FormatOptions = serde_json::from_value(options.0)
        .unwrap_or_else(|error| fail_parameter(&format!("invalid formatting options: {error}")));
    let currency = value.currency_ref();
    let locale = LocalFormat::from_locale(currency.locale());
    let digit_separator = single_character(
        options.digit_separator,
        locale.digit_separator,
        "digit_separator",
    );
    let exponent_separator = single_character(
        options.exponent_separator,
        locale.exponent_separator,
        "exponent_separator",
    );
    if digit_separator == exponent_separator {
        fail_parameter("digit_separator and exponent_separator must differ");
    }

    let separator_pattern = options
        .separator_pattern
        .unwrap_or_else(|| locale.digit_separator_pattern.to_vec());
    if separator_pattern.is_empty()
        || separator_pattern.len() > MAX_FORMAT_ITEMS
        || separator_pattern
            .iter()
            .any(|width| !(1..=9).contains(width))
    {
        fail_parameter("separator_pattern must contain between 1 and 16 widths from 1 to 9");
    }

    let positions = options.positions.map_or_else(
        || {
            if currency.symbol_first() {
                vec![Position::Sign, Position::Symbol, Position::Amount]
            } else {
                vec![Position::Sign, Position::Amount, Position::Symbol]
            }
        },
        |positions| positions.into_iter().map(Position::from).collect(),
    );
    if positions.is_empty() || positions.len() > MAX_FORMAT_ITEMS {
        fail_parameter("positions must contain between 1 and 16 entries");
    }

    let rounding = options.rounding.unwrap_or_else(|| currency.exponent());
    if rounding > 28 {
        fail_parameter("rounding must be between 0 and 28");
    }
    let money = value.as_money();
    Formatter::money(
        &money,
        Params {
            digit_separator,
            exponent_separator,
            separator_pattern: &separator_pattern,
            positions: &positions,
            rounding: Some(rounding),
            symbol: options
                .include_symbol
                .unwrap_or(true)
                .then(|| currency.symbol()),
            code: options
                .include_code
                .unwrap_or(false)
                .then(|| currency.code()),
        },
    )
}
