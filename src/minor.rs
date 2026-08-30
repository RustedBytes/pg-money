use crate::catalog::Currency;
use crate::errors::{fail_binary, fail_input, fail_parameter};
use crate::model::{
    MAX_BINARY_BYTES, STORAGE_VERSION, find_currency, money_with_currency, parse_value,
};
use pgrx::StringInfo;
use pgrx::datum::Internal;
use pgrx::prelude::*;
use rusty_money::{FastMoney, FormattableCurrency, MoneyError};
use serde::de::Error as _;
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::ffi::CStr;
use std::hash::{Hash, Hasher};

/// An i64 minor-unit amount paired with an ISO or crypto currency.
#[derive(Debug, Clone, Copy, PostgresType, PostgresEq, PostgresOrd, PostgresHash)]
#[inoutfuncs]
#[pg_binary_protocol]
// pgrx uses the Rust identifier as the SQL type name.
#[allow(non_camel_case_types)]
pub struct money_minor {
    currency: &'static Currency,
    minor_units: i64,
}

impl Serialize for money_minor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(3)?;
        tuple.serialize_element(&STORAGE_VERSION)?;
        tuple.serialize_element(self.currency_code())?;
        tuple.serialize_element(&self.minor_units)?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for money_minor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (version, currency, minor_units): (u8, &'de str, i64) =
            Deserialize::deserialize(deserializer)?;
        if version != STORAGE_VERSION {
            return Err(D::Error::custom(format!(
                "unsupported money storage version {version}"
            )));
        }
        if currency.as_bytes().iter().any(u8::is_ascii_lowercase) {
            return Err(D::Error::custom(
                "money currency is not canonically encoded",
            ));
        }
        let currency = find_currency(currency).map_err(D::Error::custom)?;
        Ok(Self::from_known_units(minor_units, currency))
    }
}

impl PartialEq for money_minor {
    fn eq(&self, other: &Self) -> bool {
        self.currency_code() == other.currency_code() && self.minor_units == other.minor_units
    }
}

impl Eq for money_minor {}

impl PartialOrd for money_minor {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for money_minor {
    fn cmp(&self, other: &Self) -> Ordering {
        self.currency_code()
            .cmp(other.currency_code())
            .then_with(|| self.minor_units.cmp(&other.minor_units))
    }
}

impl Hash for money_minor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.currency_code().hash(state);
        self.minor_units.hash(state);
    }
}

impl money_minor {
    fn from_known_units(minor_units: i64, currency: &'static Currency) -> Self {
        Self {
            currency,
            minor_units,
        }
    }

    fn from_units(minor_units: i64, currency: &str) -> Result<Self, String> {
        Ok(Self::from_known_units(
            minor_units,
            find_currency(currency)?,
        ))
    }

    fn from_fast(value: FastMoney<'static, Currency>) -> Self {
        Self::from_known_units(value.minor_units(), value.currency())
    }

    fn from_money(value: money_with_currency) -> Result<Self, MoneyError> {
        FastMoney::from_money(value.as_money()).map(Self::from_fast)
    }

    fn from_money_lossy(value: money_with_currency) -> Result<Self, MoneyError> {
        FastMoney::from_money_lossy(value.as_money()).map(Self::from_fast)
    }

    fn as_fast(&self) -> FastMoney<'static, Currency> {
        FastMoney::from_minor(self.minor_units, self.currency)
    }

    fn as_money(&self) -> money_with_currency {
        money_with_currency::from_money(self.as_fast().to_money())
    }

    fn currency_code(&self) -> &'static str {
        self.currency.code()
    }

    fn canonical(&self) -> String {
        self.as_money().canonical()
    }
}

fn parse_minor(input: &str) -> Result<money_minor, String> {
    let value = parse_value(input)?;
    money_minor::from_money(value).map_err(|error| error.to_string())
}

fn unwrap_minor(result: Result<money_minor, MoneyError>) -> money_minor {
    result.unwrap_or_else(|error| fail_parameter(&error.to_string()))
}

impl InOutFuncs for money_minor {
    fn input(input: &CStr) -> Self {
        let text = input
            .to_str()
            .unwrap_or_else(|_| fail_input("input is not valid UTF-8"));
        parse_minor(text).unwrap_or_else(|error| fail_input(&error))
    }

    fn output(&self, buffer: &mut StringInfo) {
        buffer.push_str(&self.canonical());
    }
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_from_minor(minor_units: i64, currency: &str) -> money_with_currency {
    money_minor::from_units(minor_units, currency)
        .unwrap_or_else(|error| fail_parameter(&error))
        .as_money()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_to_minor(value: money_with_currency) -> i64 {
    money_minor::from_money(value)
        .unwrap_or_else(|error| fail_parameter(&error.to_string()))
        .minor_units
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_to_minor_lossy(value: money_with_currency) -> i64 {
    money_minor::from_money_lossy(value)
        .unwrap_or_else(|error| fail_parameter(&error.to_string()))
        .minor_units
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_make(minor_units: i64, currency: &str) -> money_minor {
    money_minor::from_units(minor_units, currency).unwrap_or_else(|error| fail_parameter(&error))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_units(value: money_minor) -> i64 {
    value.minor_units
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_currency(value: money_minor) -> &'static str {
    value.currency_code()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_is_zero(value: money_minor) -> bool {
    value.as_fast().is_zero()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_is_positive(value: money_minor) -> bool {
    value.as_fast().is_positive()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_is_negative(value: money_minor) -> bool {
    value.as_fast().is_negative()
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_add(left: money_minor, right: money_minor) -> money_minor {
    unwrap_minor(
        left.as_fast()
            .add(right.as_fast())
            .map(money_minor::from_fast),
    )
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_subtract(left: money_minor, right: money_minor) -> money_minor {
    unwrap_minor(
        left.as_fast()
            .sub(right.as_fast())
            .map(money_minor::from_fast),
    )
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_multiply(value: money_minor, multiplier: i64) -> money_minor {
    unwrap_minor(value.as_fast().mul(multiplier).map(money_minor::from_fast))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_divide(value: money_minor, divisor: i64) -> money_minor {
    unwrap_minor(value.as_fast().div(divisor).map(money_minor::from_fast))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_abs(value: money_minor) -> money_minor {
    unwrap_minor(value.as_fast().abs().map(money_minor::from_fast))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_negate(value: money_minor) -> money_minor {
    unwrap_minor(value.as_fast().neg().map(money_minor::from_fast))
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_minor_format(value: money_minor) -> String {
    value.as_fast().to_string()
}

#[pg_cast(immutable, parallel_safe)]
pub(crate) fn money_minor_to_money_with_currency(value: money_minor) -> money_with_currency {
    value.as_money()
}

#[pg_cast(immutable, parallel_safe)]
pub(crate) fn money_with_currency_to_money_minor(value: money_with_currency) -> money_minor {
    money_minor::from_money(value).unwrap_or_else(|error| fail_parameter(&error.to_string()))
}

#[pg_extern(immutable, strict, parallel_safe)]
pub(crate) fn money_minor_send_safe(value: money_minor) -> Vec<u8> {
    serde_cbor::to_vec(&value)
        .unwrap_or_else(|error| fail_binary(&format!("could not encode value: {error}")))
}

#[pg_extern(immutable, strict, parallel_safe)]
pub(crate) fn money_minor_recv_safe(mut internal: Internal) -> money_minor {
    // SAFETY: PostgreSQL invokes a base-type receive function with a valid,
    // backend-owned StringInfoData pointer. Bounds are checked before slicing.
    let buffer = unsafe { internal.get_mut::<pg_sys::StringInfoData>() }
        .unwrap_or_else(|| fail_binary("missing protocol buffer"));
    if buffer.cursor < 0 || buffer.len < buffer.cursor {
        fail_binary("invalid protocol cursor");
    }
    let remaining = usize::try_from(buffer.len - buffer.cursor)
        .unwrap_or_else(|_| fail_binary("invalid protocol length"));
    if remaining == 0 || remaining > MAX_BINARY_BYTES {
        fail_binary(&format!(
            "payload length must be between 1 and {MAX_BINARY_BYTES} bytes"
        ));
    }
    let cursor =
        usize::try_from(buffer.cursor).unwrap_or_else(|_| fail_binary("invalid protocol cursor"));
    let bytes = {
        // SAFETY: StringInfoData guarantees `data` is valid for `len` bytes, and
        // the validated nonnegative cursor plus `remaining` stays within `len`.
        unsafe { std::slice::from_raw_parts(buffer.data.add(cursor).cast::<u8>(), remaining) }
    };
    let decoded =
        serde_cbor::from_slice(bytes).unwrap_or_else(|error| fail_binary(&error.to_string()));
    buffer.cursor = buffer.len;
    decoded
}

#[pg_extern(immutable, strict, parallel_safe)]
pub(crate) fn money_minor_hash_extended(value: money_minor, seed: i64) -> i64 {
    let mut hash = pgrx::misc::pgrx_seahash(&value) ^ seed.cast_unsigned();
    hash = hash.wrapping_add(0x9e37_79b9_7f4a_7c15);
    hash = (hash ^ (hash >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash = (hash ^ (hash >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (hash ^ (hash >> 31)).cast_signed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cbor_round_trip_preserves_minor_units_and_currency() {
        let value = money_minor::from_units(12_345, "USD").expect("USD must be available");
        let encoded = serde_cbor::to_vec(&value).expect("money_minor must serialize");
        let decoded: money_minor =
            serde_cbor::from_slice(&encoded).expect("money_minor must deserialize");

        assert_eq!(decoded, value);
        assert_eq!(decoded.currency_code(), "USD");
        assert_eq!(decoded.minor_units, 12_345);
        assert!(encoded.len() <= MAX_BINARY_BYTES);
    }

    #[test]
    fn cbor_rejects_noncanonical_currency_and_unknown_version() {
        let lowercase = serde_cbor::to_vec(&(STORAGE_VERSION, "usd", 100_i64))
            .expect("test tuple must serialize");
        let future = serde_cbor::to_vec(&(STORAGE_VERSION + 1, "USD", 100_i64))
            .expect("test tuple must serialize");

        assert!(serde_cbor::from_slice::<money_minor>(&lowercase).is_err());
        assert!(serde_cbor::from_slice::<money_minor>(&future).is_err());
    }
}
