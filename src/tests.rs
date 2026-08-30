use crate::api::{
    money_amount, money_currency, money_from_rusty_json, money_parse, money_to_rusty_json,
};
use crate::arithmetic::{money_add, money_round, money_rounding, money_split};
use crate::currency::money_currency_info;
use crate::exchange::money_exchange;
use crate::formatting::{money_format_with, money_parse_localized, money_try_parse_localized};
use crate::minor::{money_from_minor, money_to_minor, money_to_minor_lossy};
#[cfg(test)]
use crate::model::{money_with_currency, parse_value};
use pgrx::prelude::*;
use pgrx::{AnyNumeric, JsonB};
#[cfg(test)]
use rust_decimal::Decimal;
#[cfg(test)]
use std::collections::hash_map::DefaultHasher;
#[cfg(test)]
use std::hash::{Hash, Hasher};

#[pg_schema]
mod tests {
    use super::*;

    #[pg_test]
    fn canonical_storage_and_accessors_work() {
        assert_eq!(money_parse("usd 10").canonical(), "USD 10.00");
        assert_eq!(money_parse("JPY -10.50").canonical(), "JPY -10.5");
        assert_eq!(money_currency(money_parse("EUR 1.25")), "EUR");
        assert_eq!(money_amount(money_parse("USD 1.2300")).to_string(), "1.23");
    }

    #[pg_test]
    fn currency_catalog_and_metadata_work() {
        let usd = money_currency_info("usd").0;
        assert_eq!(usd["code"], "USD");
        assert_eq!(usd["numeric_code"], "840");
        assert_eq!(usd["exponent"], 2);
        assert_eq!(usd["symbol"], "$");
        assert_eq!(usd["locale"], "en-us");
        assert_eq!(
            Spi::get_one::<bool>(
                "SELECT count(*) > 150 AND count(*) = count(DISTINCT code) \
                 FROM money_currencies()"
            )
            .unwrap(),
            Some(true)
        );
        assert_eq!(
            Spi::get_one::<String>(
                "SELECT name FROM money_currencies() WHERE numeric_code = '392'"
            )
            .unwrap(),
            Some("Japanese Yen".to_owned())
        );
    }

    #[pg_test]
    fn localized_parsing_and_custom_formatting_work() {
        assert_eq!(
            money_parse_localized("1.000,99", "EUR").canonical(),
            "EUR 1000.99"
        );
        assert_eq!(
            money_parse_localized("1,00,000.50", "INR").canonical(),
            "INR 100000.50"
        );
        assert!(money_try_parse_localized("1,00", "USD").is_none());

        let options = JsonB(serde_json::json!({
            "digit_separator": "/",
            "exponent_separator": ",",
            "positions": ["sign", "space", "symbol", "amount", "space", "code"],
            "rounding": 2,
            "include_code": true
        }));
        assert_eq!(
            money_format_with(money_parse("USD -1234.5"), options),
            "- $1/234,50 USD"
        );
    }

    #[pg_test]
    fn rusty_money_json_contract_round_trips() {
        let value = money_parse("USD 123.45");
        let encoded = money_to_rusty_json(value);
        assert_eq!(
            encoded.0,
            serde_json::json!({"amount": "123.45", "currency": "USD"})
        );
        assert_eq!(money_from_rusty_json(encoded).canonical(), "USD 123.45");
    }

    #[pg_test]
    fn minor_unit_conversions_and_fast_type_work() {
        assert_eq!(money_from_minor(12_345, "USD").canonical(), "USD 123.45");
        assert_eq!(money_to_minor(money_parse("JPY 123")), 123);
        assert_eq!(money_to_minor_lossy(money_parse("USD 1.239")), 123);
        assert_eq!(
            Spi::get_one::<String>(
                "SELECT (money_minor_make(1000, 'USD') \
                        + money_minor_make(500, 'USD'))::text"
            )
            .unwrap(),
            Some("USD 15.00".to_owned())
        );
        assert_eq!(
            Spi::get_one::<String>(
                "SELECT ((money_minor_make(1001, 'USD') / 3::bigint) \
                         * 3::bigint)::text"
            )
            .unwrap(),
            Some("USD 9.99".to_owned())
        );
        assert_eq!(
            Spi::get_one::<i64>(
                "SELECT money_minor_units( \
                    ('USD 123.45'::money_with_currency)::money_minor)"
            )
            .unwrap(),
            Some(12_345)
        );
        Spi::run(
            "CREATE TEMP TABLE minor_index_test(value money_minor UNIQUE); \
             INSERT INTO minor_index_test VALUES ('USD 1.00'), ('EUR 1.00'); \
             CREATE INDEX minor_index_test_hash ON minor_index_test USING hash(value);",
        )
        .unwrap();
        assert_eq!(
            Spi::get_one::<bool>(
                "SELECT typreceive::regproc::text = 'money_minor_recv_safe' \
                     AND typsend::regproc::text = 'money_minor_send_safe' \
                 FROM pg_type WHERE typname = 'money_minor'"
            )
            .unwrap(),
            Some(true)
        );
    }

    #[pg_test(error = "Conversion would lose precision")]
    fn strict_minor_conversion_rejects_fractional_minor_units() {
        Spi::run("SELECT money_to_minor('USD 1.001'::money_with_currency)").unwrap();
    }

    #[pg_test]
    fn arithmetic_rounding_and_allocation_work() {
        assert_eq!(
            money_add(money_parse("USD 10"), money_parse("USD 2.50")).canonical(),
            "USD 12.50"
        );
        assert_eq!(
            money_round(money_parse("USD 10.005"), 2, money_rounding::half_up).canonical(),
            "USD 10.01"
        );
        assert_eq!(
            money_split(money_parse("USD 10"), 3)
                .into_iter()
                .map(|value| value.canonical())
                .collect::<Vec<_>>(),
            ["USD 3.34", "USD 3.33", "USD 3.33"]
        );
    }

    #[pg_test]
    fn sql_operators_aggregates_and_indexes_work() {
        Spi::run(
            "CREATE TABLE money_test(value money_with_currency UNIQUE); \
             INSERT INTO money_test VALUES ('USD 10.00'), ('USD 2.50'); \
             INSERT INTO money_test VALUES ('USD 10.0000') ON CONFLICT DO NOTHING; \
             CREATE INDEX money_test_hash ON money_test USING hash(value);",
        )
        .unwrap();
        assert_eq!(
            Spi::get_one::<String>(
                "SELECT (sum(value))::text FROM money_test WHERE money_currency(value) = 'USD'"
            )
            .unwrap(),
            Some("USD 12.50".to_owned())
        );
        assert_eq!(
            Spi::get_one::<String>("SELECT avg(value)::text FROM money_test").unwrap(),
            Some("USD 6.25".to_owned())
        );
        assert_eq!(
            Spi::get_one::<bool>(
                "SELECT sum(value) IS NULL AND avg(value) IS NULL \
                 FROM (VALUES (NULL::money_with_currency)) AS empty(value)"
            )
            .unwrap(),
            Some(true)
        );
        assert_eq!(
            Spi::get_one::<String>("SELECT ('USD 10'::money_with_currency * 2.5)::text").unwrap(),
            Some("USD 25.00".to_owned())
        );
        assert_eq!(
            Spi::get_one::<bool>(
                "SELECT 'EUR 999'::money_with_currency < 'USD -999'::money_with_currency"
            )
            .unwrap(),
            Some(true)
        );
        Spi::run(
            "CREATE TABLE money_partition_test(value money_with_currency NOT NULL) \
                PARTITION BY HASH(value); \
             CREATE TABLE money_partition_test_0 PARTITION OF money_partition_test \
                FOR VALUES WITH (modulus 2, remainder 0); \
             CREATE TABLE money_partition_test_1 PARTITION OF money_partition_test \
                FOR VALUES WITH (modulus 2, remainder 1); \
             INSERT INTO money_partition_test VALUES ('USD 1'), ('EUR 1');",
        )
        .unwrap();
        assert_eq!(
            Spi::get_one::<i64>("SELECT count(*) FROM money_partition_test").unwrap(),
            Some(2)
        );
        assert_eq!(
            Spi::get_one::<bool>(
                "SELECT typreceive::regproc::text = 'money_with_currency_recv_safe' \
                     AND typsend::regproc::text = 'money_with_currency_send_safe' \
                 FROM pg_type WHERE typname = 'money_with_currency'"
            )
            .unwrap(),
            Some(true)
        );
    }

    #[pg_test]
    fn large_aggregate_group_uses_bounded_internal_state() {
        assert_eq!(
            Spi::get_one::<String>(
                "SELECT sum(money_make(g::numeric, 'USD'))::text \
                 FROM generate_series(1, 100000) AS series(g)"
            )
            .unwrap(),
            Some("USD 5000050000.00".to_owned())
        );
        assert_eq!(
            Spi::get_one::<String>(
                "SELECT avg(money_make(g::numeric, 'USD'))::text \
                 FROM generate_series(1, 100000) AS series(g)"
            )
            .unwrap(),
            Some("USD 50000.50".to_owned())
        );
    }

    #[pg_test]
    fn exchange_direct_and_historical_lookup_work() {
        assert_eq!(
            money_exchange(
                money_parse("USD 100"),
                "EUR",
                AnyNumeric::try_from("0.85").unwrap()
            )
            .canonical(),
            "EUR 85.00"
        );
        Spi::run(
            "CREATE TEMP TABLE rates( \
                from_currency text, to_currency text, rate numeric, valid_at timestamptz \
             ); \
             INSERT INTO rates VALUES \
                ('USD', 'EUR', 0.80, '2025-01-01'), \
                ('USD', 'EUR', 0.85, '2026-01-01');",
        )
        .unwrap();
        assert_eq!(
            Spi::get_one::<String>(
                "SELECT money_exchange_at( \
                    'USD 100', 'EUR', 'rates'::regclass, '2025-06-01'::timestamptz \
                 )::text"
            )
            .unwrap(),
            Some("EUR 80.00".to_owned())
        );
    }

    #[pg_test(error = "Currency mismatch: expected USD, got EUR")]
    fn mixed_currency_arithmetic_fails() {
        Spi::run("SELECT 'USD 1'::money_with_currency + 'EUR 1'::money_with_currency").unwrap();
    }

    #[pg_test(error = "Currency mismatch: expected USD, got EUR")]
    fn mixed_currency_aggregate_fails() {
        Spi::run(
            "SELECT sum(value) FROM (VALUES \
                ('USD 1'::money_with_currency), \
                ('EUR 1'::money_with_currency)) AS mixed(value)",
        )
        .unwrap();
    }

    #[pg_test(
        error = "invalid input syntax for type money_with_currency: amount must be a plain signed decimal"
    )]
    fn locale_dependent_type_input_is_rejected() {
        Spi::run("SELECT 'USD 1,000.00'::money_with_currency").unwrap();
    }

    #[pg_test(error = "Arithmetic overflow")]
    fn arithmetic_overflow_is_reported() {
        Spi::run(
            "SELECT 'USD 79228162514264337593543950335'::money_with_currency \
                    + 'USD 1'::money_with_currency",
        )
        .unwrap();
    }

    #[pg_test(error = "Division by zero")]
    fn division_by_zero_is_reported() {
        Spi::run("SELECT 'USD 1'::money_with_currency / 0").unwrap();
    }

    #[pg_test(error = "parts must be between 1 and 10000")]
    fn excessive_split_is_rejected_before_allocation() {
        Spi::run("SELECT money_split('USD 1', 10001)").unwrap();
    }

    #[pg_test(error = "allocation accepts at most 10000 weights")]
    fn excessive_allocation_is_rejected() {
        Spi::run(
            "SELECT money_allocate(\
                'USD 1', ARRAY(SELECT 1 FROM generate_series(1, 10001))\
             )",
        )
        .unwrap();
    }
}

#[cfg(test)]
#[test]
fn binary_format_is_stable_and_validated() {
    let value = parse_value("USD 123.45").unwrap();
    let encoded = serde_cbor::to_vec(&value).unwrap();
    assert_eq!(
        encoded,
        [
            0x84, 0x01, 0x63, b'U', b'S', b'D', 0x90, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x30, 0x18, 0x39, 0x02,
        ]
    );
    assert_eq!(
        serde_cbor::from_slice::<money_with_currency>(&encoded).unwrap(),
        value
    );
    assert_eq!(value.canonical(), "USD 123.45");
    assert_eq!(parse_value("JPY -10.50").unwrap().canonical(), "JPY -10.5");
    assert_eq!(
        parse_value("USD 1,000.00").unwrap_err(),
        "amount must be a plain signed decimal"
    );

    let mut value_hash = DefaultHasher::new();
    value.hash(&mut value_hash);
    let mut logical_hash = DefaultHasher::new();
    "USD".hash(&mut logical_hash);
    Decimal::new(12345, 2).hash(&mut logical_hash);
    assert_eq!(value_hash.finish(), logical_hash.finish());

    assert!(std::mem::size_of::<money_with_currency>() <= 32);

    let noncanonical = serde_cbor::to_vec(&(
        1_u8,
        "USD",
        Decimal::new(12300, 4).mantissa().to_be_bytes(),
        4_u32,
    ))
    .unwrap();
    assert!(serde_cbor::from_slice::<money_with_currency>(&noncanonical).is_err());

    let maximum = money_with_currency::from_decimal(Decimal::MAX, "USD").unwrap();
    let one = money_with_currency::from_decimal(Decimal::ONE, "USD").unwrap();
    assert_eq!(
        maximum.checked_add(one),
        Err(rusty_money::MoneyError::Overflow)
    );
    assert_eq!(
        maximum.checked_div(Decimal::ZERO),
        Err(rusty_money::MoneyError::DivisionByZero)
    );
}

#[cfg(test)]
mod properties {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn canonical_round_trip(mantissa in -1_000_000_000_i64..1_000_000_000, scale in 0_u32..=6) {
            let value = money_with_currency::from_decimal(Decimal::new(mantissa, scale), "USD").unwrap();
            prop_assert_eq!(parse_value(&value.canonical()).unwrap(), value);
        }

        #[test]
        fn binary_decoder_never_panics(data in prop::collection::vec(any::<u8>(), 0..256)) {
            let _ = serde_cbor::from_slice::<money_with_currency>(&data);
        }

        #[test]
        fn optimized_add_and_subtract_match_rusty_money(
            left in -1_000_000_000_i64..1_000_000_000,
            right in -1_000_000_000_i64..1_000_000_000,
            scale in 0_u32..=6,
        ) {
            let left = money_with_currency::from_decimal(Decimal::new(left, scale), "USD").unwrap();
            let right = money_with_currency::from_decimal(Decimal::new(right, scale), "USD").unwrap();
            let expected_sum = money_with_currency::from_money(
                left.as_money().add(right.as_money()).unwrap()
            );
            let expected_difference = money_with_currency::from_money(
                left.as_money().sub(right.as_money()).unwrap()
            );

            prop_assert_eq!(left.checked_add(right).unwrap(), expected_sum);
            prop_assert_eq!(left.checked_sub(right).unwrap(), expected_difference);
        }

        #[test]
        fn optimized_multiply_and_divide_match_rusty_money(
            amount in -1_000_000_i64..1_000_000,
            scalar in -10_000_i64..10_000,
            scale in 0_u32..=4,
        ) {
            let value = money_with_currency::from_decimal(Decimal::new(amount, scale), "USD").unwrap();
            let scalar = Decimal::new(scalar, scale);
            let expected_product = value
                .as_money()
                .mul(scalar)
                .map(money_with_currency::from_money);
            let expected_quotient = value
                .as_money()
                .div(scalar)
                .map(money_with_currency::from_money);

            prop_assert_eq!(value.checked_mul(scalar), expected_product);
            prop_assert_eq!(value.checked_div(scalar), expected_quotient);
        }
    }
}
