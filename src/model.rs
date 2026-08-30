use rust_decimal::Decimal;
use rusty_money::{FormattableCurrency, Money, iso};
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

pub(crate) const STORAGE_VERSION: u8 = 1;
pub(crate) const MAX_BINARY_BYTES: usize = 128;
const MAX_INPUT_BYTES: usize = 128;

/// A precise decimal amount paired with an ISO-4217 currency.
#[derive(Debug, Clone)]
#[cfg_attr(
    not(feature = "fuzzing"),
    derive(PostgresType, PostgresEq, PostgresOrd, PostgresHash)
)]
#[cfg_attr(not(feature = "fuzzing"), inoutfuncs)]
#[cfg_attr(not(feature = "fuzzing"), pg_binary_protocol)]
// pgrx uses the Rust identifier as the SQL type name.
#[allow(non_camel_case_types)]
pub struct money_with_currency {
    version: u8,
    pub(crate) currency: String,
    mantissa: i128,
    scale: u32,
}

impl Serialize for money_with_currency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(4)?;
        tuple.serialize_element(&self.version)?;
        tuple.serialize_element(&self.currency)?;
        tuple.serialize_element(&self.mantissa.to_be_bytes())?;
        tuple.serialize_element(&self.scale)?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for money_with_currency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (version, currency, bytes, scale): (u8, String, [u8; 16], u32) =
            Deserialize::deserialize(deserializer)?;
        if version != STORAGE_VERSION {
            return Err(D::Error::custom(format!(
                "unsupported money storage version {version}"
            )));
        }
        let decimal = Decimal::try_from_i128_with_scale(i128::from_be_bytes(bytes), scale)
            .map_err(D::Error::custom)?;
        Self::from_decimal(decimal, &currency).map_err(D::Error::custom)
    }
}

impl PartialEq for money_with_currency {
    fn eq(&self, other: &Self) -> bool {
        self.currency == other.currency && self.decimal() == other.decimal()
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
        self.currency
            .cmp(&other.currency)
            .then_with(|| self.decimal().cmp(&other.decimal()))
    }
}

impl Hash for money_with_currency {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.currency.hash(state);
        self.decimal().hash(state);
    }
}

impl money_with_currency {
    pub(crate) fn from_decimal(decimal: Decimal, currency: &str) -> Result<Self, String> {
        let currency = currency.trim().to_ascii_uppercase();
        if iso::find(&currency).is_none() {
            return Err(format!("unknown ISO-4217 currency: {currency}"));
        }
        let decimal = decimal.normalize();
        Ok(Self {
            version: STORAGE_VERSION,
            currency,
            mantissa: decimal.mantissa(),
            scale: decimal.scale(),
        })
    }

    pub(crate) fn from_money(value: Money<'static, iso::Currency>) -> Self {
        Self::from_decimal(*value.amount(), value.currency().code())
            .expect("rusty-money returned an invalid ISO money value")
    }

    pub(crate) fn decimal(&self) -> Decimal {
        Decimal::try_from_i128_with_scale(self.mantissa, self.scale)
            .expect("validated money_with_currency decimal")
    }

    pub(crate) fn currency_ref(&self) -> &'static iso::Currency {
        iso::find(&self.currency).expect("validated money_with_currency currency")
    }

    pub(crate) fn as_money(&self) -> Money<'static, iso::Currency> {
        Money::from_decimal(self.decimal(), self.currency_ref())
    }

    pub(crate) fn canonical_amount(&self) -> String {
        let mut amount = self.decimal().to_string();
        let exponent = self.currency_ref().exponent() as usize;
        let fraction = amount
            .split_once('.')
            .map(|(_, fraction)| fraction.len())
            .unwrap_or(0);
        if exponent > fraction {
            if fraction == 0 {
                amount.push('.');
            }
            amount.push_str(&"0".repeat(exponent - fraction));
        }
        amount
    }

    pub(crate) fn canonical(&self) -> String {
        format!("{} {}", self.currency, self.canonical_amount())
    }
}

pub(crate) fn parse_decimal(input: &str) -> Result<Decimal, String> {
    if input.is_empty() {
        return Err("amount is empty".to_owned());
    }
    let unsigned = input
        .strip_prefix('+')
        .or_else(|| input.strip_prefix('-'))
        .unwrap_or(input);
    if unsigned.is_empty()
        || unsigned.matches('.').count() > 1
        || !unsigned
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
        || unsigned == "."
    {
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
        .ok_or_else(|| "expected '<ISO currency> <amount>'".to_owned())?;
    let amount = parts
        .next()
        .ok_or_else(|| "expected '<ISO currency> <amount>'".to_owned())?;
    if parts.next().is_some() || currency.len() != 3 {
        return Err("expected '<ISO currency> <amount>'".to_owned());
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
        buffer.push_str(&self.canonical());
    }
}
