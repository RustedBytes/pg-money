use rust_decimal::Decimal;
use rusty_money::{FormattableCurrency, Money, MoneyError};
use serde::de::Error as _;
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

#[cfg(not(feature = "fuzzing"))]
use crate::errors::fail_input;
#[cfg(not(feature = "fuzzing"))]
use pgrx::StringInfo;
#[cfg(not(feature = "fuzzing"))]
use pgrx::prelude::*;
#[cfg(not(feature = "fuzzing"))]
use std::ffi::CStr;

use crate::catalog::{self, Currency};

pub(crate) const STORAGE_VERSION: u8 = 1;
pub(crate) const MAX_BINARY_BYTES: usize = 128;
pub(crate) const MAX_INPUT_BYTES: usize = 128;

/// A precise decimal amount paired with an ISO or crypto currency.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(
    not(feature = "fuzzing"),
    derive(PostgresType, PostgresEq, PostgresOrd, PostgresHash)
)]
#[cfg_attr(not(feature = "fuzzing"), inoutfuncs)]
#[cfg_attr(not(feature = "fuzzing"), pg_binary_protocol)]
// pgrx uses the Rust identifier as the SQL type name.
#[allow(non_camel_case_types)]
pub struct money_with_currency {
    currency: &'static Currency,
    amount: Decimal,
}

impl Serialize for money_with_currency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(4)?;
        tuple.serialize_element(&STORAGE_VERSION)?;
        tuple.serialize_element(self.currency_code())?;
        tuple.serialize_element(&self.amount.mantissa().to_be_bytes())?;
        tuple.serialize_element(&self.amount.scale())?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for money_with_currency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (version, currency, bytes, scale): (u8, &'de str, [u8; 16], u32) =
            Deserialize::deserialize(deserializer)?;
        if version != STORAGE_VERSION {
            return Err(D::Error::custom(format!(
                "unsupported money storage version {version}"
            )));
        }
        let amount = Decimal::try_from_i128_with_scale(i128::from_be_bytes(bytes), scale)
            .map_err(D::Error::custom)?;
        let normalized = amount.normalize();
        if normalized.mantissa() != amount.mantissa() || normalized.scale() != amount.scale() {
            return Err(D::Error::custom("money amount is not canonically encoded"));
        }
        if currency.as_bytes().iter().any(u8::is_ascii_lowercase) {
            return Err(D::Error::custom(
                "money currency is not canonically encoded",
            ));
        }
        Self::from_decimal(amount, currency).map_err(D::Error::custom)
    }
}

impl PartialEq for money_with_currency {
    fn eq(&self, other: &Self) -> bool {
        self.currency_code() == other.currency_code() && self.amount == other.amount
    }
}

impl Eq for money_with_currency {}

impl PartialOrd for money_with_currency {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for money_with_currency {
    fn cmp(&self, other: &Self) -> Ordering {
        self.currency_code()
            .cmp(other.currency_code())
            .then_with(|| self.amount.cmp(&other.amount))
    }
}

impl Hash for money_with_currency {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.currency_code().hash(state);
        self.amount.hash(state);
    }
}

impl money_with_currency {
    pub(crate) fn from_decimal(decimal: Decimal, currency: &str) -> Result<Self, String> {
        Ok(Self::from_known_currency(decimal, find_currency(currency)?))
    }

    pub(crate) fn from_money(value: Money<'static, Currency>) -> Self {
        Self::from_known_currency(*value.amount(), value.currency())
    }

    pub(crate) fn from_known_currency(amount: Decimal, currency: &'static Currency) -> Self {
        Self {
            currency,
            amount: amount.normalize(),
        }
    }

    pub(crate) fn with_decimal(&self, amount: Decimal) -> Self {
        Self::from_known_currency(amount, self.currency)
    }

    pub(crate) fn checked_add(self, other: Self) -> Result<Self, MoneyError> {
        self.require_same_currency(&other)?;
        self.amount
            .checked_add(other.amount)
            .map(|amount| self.with_decimal(amount))
            .ok_or(MoneyError::Overflow)
    }

    pub(crate) fn checked_sub(self, other: Self) -> Result<Self, MoneyError> {
        self.require_same_currency(&other)?;
        self.amount
            .checked_sub(other.amount)
            .map(|amount| self.with_decimal(amount))
            .ok_or(MoneyError::Overflow)
    }

    pub(crate) fn checked_mul(self, multiplier: Decimal) -> Result<Self, MoneyError> {
        self.amount
            .checked_mul(multiplier)
            .map(|amount| self.with_decimal(amount))
            .ok_or(MoneyError::Overflow)
    }

    pub(crate) fn checked_div(self, divisor: Decimal) -> Result<Self, MoneyError> {
        if divisor.is_zero() {
            return Err(MoneyError::DivisionByZero);
        }
        self.amount
            .checked_div(divisor)
            .map(|amount| self.with_decimal(amount))
            .ok_or(MoneyError::Overflow)
    }

    fn require_same_currency(&self, other: &Self) -> Result<(), MoneyError> {
        // All currencies enter through the static catalog, so equal currencies share
        // the same process-static descriptor.
        if std::ptr::eq(self.currency, other.currency) {
            Ok(())
        } else {
            Err(MoneyError::CurrencyMismatch {
                expected: self.currency_code(),
                actual: other.currency_code(),
            })
        }
    }

    pub(crate) fn decimal(&self) -> Decimal {
        self.amount
    }

    pub(crate) fn currency_ref(&self) -> &'static Currency {
        self.currency
    }

    pub(crate) fn currency_code(&self) -> &'static str {
        self.currency.code()
    }

    pub(crate) fn as_money(&self) -> Money<'static, Currency> {
        Money::from_decimal(self.decimal(), self.currency_ref())
    }

    pub(crate) fn canonical_amount(&self) -> String {
        let mut amount = String::with_capacity(32);
        self.write_canonical_amount(&mut amount);
        amount
    }

    pub(crate) fn canonical(&self) -> String {
        let mut output = String::with_capacity(36);
        self.write_canonical(&mut output);
        output
    }

    fn write_canonical(&self, output: &mut impl std::fmt::Write) {
        output
            .write_str(self.currency_code())
            .expect("writing canonical currency cannot fail");
        output
            .write_char(' ')
            .expect("writing canonical separator cannot fail");
        self.write_canonical_amount(output);
    }

    fn write_canonical_amount(&self, output: &mut impl std::fmt::Write) {
        write!(output, "{}", self.amount).expect("writing canonical amount cannot fail");
        let exponent = self.currency.exponent() as usize;
        let scale = self.amount.scale() as usize;
        if exponent > scale {
            if scale == 0 {
                output
                    .write_char('.')
                    .expect("writing canonical decimal point cannot fail");
            }
            for _ in 0..exponent - scale {
                output
                    .write_char('0')
                    .expect("writing canonical zero cannot fail");
            }
        }
    }
}

pub(crate) fn find_currency(input: &str) -> Result<&'static Currency, String> {
    let input = input.trim();
    if (3..=4).contains(&input.len()) && input.is_ascii() {
        let mut code = [0_u8; 4];
        code[..input.len()].copy_from_slice(input.as_bytes());
        code[..input.len()].make_ascii_uppercase();
        // SAFETY: `code` was copied from ASCII input and ASCII uppercasing
        // preserves UTF-8 validity.
        let code = unsafe { std::str::from_utf8_unchecked(&code[..input.len()]) };
        if let Some(currency) = catalog::find(code) {
            return Ok(currency);
        }
    }
    Err(format!("unknown currency: {}", input.to_ascii_uppercase()))
}

pub(crate) fn parse_decimal(input: &str) -> Result<Decimal, String> {
    if input.is_empty() {
        return Err("amount is empty".to_owned());
    }
    let unsigned = input
        .strip_prefix('+')
        .or_else(|| input.strip_prefix('-'))
        .unwrap_or(input);
    let mut has_digit = false;
    let mut has_decimal_point = false;
    for byte in unsigned.bytes() {
        match byte {
            b'0'..=b'9' => has_digit = true,
            b'.' if !has_decimal_point => has_decimal_point = true,
            _ => return Err("amount must be a plain signed decimal".to_owned()),
        }
    }
    if !has_digit {
        return Err("amount must be a plain signed decimal".to_owned());
    }
    Decimal::from_str(input).map_err(|_| "amount exceeds decimal precision".to_owned())
}

pub(crate) fn parse_value(input: &str) -> Result<money_with_currency, String> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(format!("input exceeds {MAX_INPUT_BYTES} bytes"));
    }
    if input.as_bytes().contains(&0) {
        return Err("input contains a NUL byte".to_owned());
    }
    let mut parts = input.split_ascii_whitespace();
    let currency = parts
        .next()
        .ok_or_else(|| "expected '<currency> <amount>'".to_owned())?;
    let amount = parts
        .next()
        .ok_or_else(|| "expected '<currency> <amount>'".to_owned())?;
    if parts.next().is_some() || !(3..=4).contains(&currency.len()) {
        return Err("expected '<currency> <amount>'".to_owned());
    }
    money_with_currency::from_decimal(parse_decimal(amount)?, currency)
}

#[cfg(not(feature = "fuzzing"))]
impl InOutFuncs for money_with_currency {
    fn input(input: &CStr) -> Self {
        let text = input
            .to_str()
            .unwrap_or_else(|_| fail_input("input is not valid UTF-8"));
        parse_value(text).unwrap_or_else(|error| fail_input(&error))
    }

    fn output(&self, buffer: &mut StringInfo) {
        self.write_canonical(buffer);
    }
}
