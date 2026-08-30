use crate::api::{money_amount, money_currency, money_parse};
use crate::arithmetic::{money_add, money_round, money_rounding, money_split};
use crate::exchange::money_exchange;
#[cfg(test)]
use crate::model::{money_with_currency, parse_value};
use pgrx::AnyNumeric;
use pgrx::prelude::*;
#[cfg(test)]
use rust_decimal::Decimal;

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
}

#[cfg(test)]
#[test]
fn binary_format_is_stable_and_validated() {
    let value = parse_value("USD 123.45").unwrap();
    let encoded = serde_cbor::to_vec(&value).unwrap();
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
    }
}
