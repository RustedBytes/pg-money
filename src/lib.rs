#![cfg_attr(feature = "fuzzing", allow(dead_code, unused_imports))]

mod catalog;
mod model;

#[cfg(not(feature = "fuzzing"))]
mod aggregate;
#[cfg(not(feature = "fuzzing"))]
mod api;
#[cfg(not(feature = "fuzzing"))]
mod arithmetic;
#[cfg(not(feature = "fuzzing"))]
mod binary;
#[cfg(not(feature = "fuzzing"))]
mod comparison;
#[cfg(not(feature = "fuzzing"))]
mod currency;
#[cfg(not(feature = "fuzzing"))]
mod errors;
#[cfg(not(feature = "fuzzing"))]
mod exchange;
#[cfg(not(feature = "fuzzing"))]
mod formatting;
#[cfg(feature = "fuzzing")]
pub mod fuzz_api;
#[cfg(not(feature = "fuzzing"))]
mod minor;
#[cfg(not(feature = "fuzzing"))]
mod numeric;
#[cfg(any(test, feature = "pg_test"))]
mod tests;

#[cfg(not(feature = "fuzzing"))]
// These imports are consumed as identifiers by `extension_sql!`, which rustc
// cannot observe as ordinary uses.
#[allow(unused_imports)]
use aggregate::{
    money_aggregate_combinefn, money_aggregate_deserialfn, money_aggregate_serialfn,
    money_aggregate_transfn, money_avg_finalfn, money_sum_finalfn,
};
#[cfg(not(feature = "fuzzing"))]
#[allow(unused_imports)]
use arithmetic::{money_add, money_divide, money_multiply, money_negate, money_subtract};
#[cfg(not(feature = "fuzzing"))]
#[allow(unused_imports)]
use minor::{
    money_minor, money_minor_add, money_minor_divide, money_minor_multiply, money_minor_negate,
    money_minor_subtract,
};
#[cfg(not(feature = "fuzzing"))]
#[allow(unused_imports)]
use model::money_with_currency;
#[cfg(not(feature = "fuzzing"))]
use pgrx::prelude::*;

#[cfg(not(feature = "fuzzing"))]
pgrx::pg_module_magic!();

#[cfg(not(feature = "fuzzing"))]
extension_sql!(
    r"
    CREATE CAST (varchar AS money_with_currency) WITH INOUT;
    CREATE CAST (money_with_currency AS varchar) WITH INOUT AS ASSIGNMENT;

    CREATE OPERATOR + (
        LEFTARG = money_with_currency,
        RIGHTARG = money_with_currency,
        FUNCTION = money_add
    );
    CREATE OPERATOR - (
        LEFTARG = money_with_currency,
        RIGHTARG = money_with_currency,
        FUNCTION = money_subtract
    );
    CREATE OPERATOR * (
        LEFTARG = money_with_currency,
        RIGHTARG = numeric,
        FUNCTION = money_multiply
    );
    CREATE OPERATOR / (
        LEFTARG = money_with_currency,
        RIGHTARG = numeric,
        FUNCTION = money_divide
    );
    CREATE OPERATOR - (
        RIGHTARG = money_with_currency,
        FUNCTION = money_negate
    );

    CREATE AGGREGATE sum(money_with_currency) (
        SFUNC = money_aggregate_transfn,
        STYPE = internal,
        FINALFUNC = money_sum_finalfn,
        COMBINEFUNC = money_aggregate_combinefn,
        SERIALFUNC = money_aggregate_serialfn,
        DESERIALFUNC = money_aggregate_deserialfn,
        PARALLEL = SAFE
    );
    CREATE AGGREGATE avg(money_with_currency) (
        SFUNC = money_aggregate_transfn,
        STYPE = internal,
        FINALFUNC = money_avg_finalfn,
        COMBINEFUNC = money_aggregate_combinefn,
        SERIALFUNC = money_aggregate_serialfn,
        DESERIALFUNC = money_aggregate_deserialfn,
        PARALLEL = SAFE
    );
    ",
    name = "money_sql_surface",
    requires = [
        money_with_currency,
        money_add,
        money_subtract,
        money_multiply,
        money_divide,
        money_negate,
        money_aggregate_transfn,
        money_aggregate_combinefn,
        money_aggregate_serialfn,
        money_aggregate_deserialfn,
        money_sum_finalfn,
        money_avg_finalfn
    ]
);

#[cfg(not(feature = "fuzzing"))]
extension_sql!(
    r"
    CREATE OPERATOR + (
        LEFTARG = money_minor,
        RIGHTARG = money_minor,
        FUNCTION = money_minor_add
    );
    CREATE OPERATOR - (
        LEFTARG = money_minor,
        RIGHTARG = money_minor,
        FUNCTION = money_minor_subtract
    );
    CREATE OPERATOR * (
        LEFTARG = money_minor,
        RIGHTARG = bigint,
        FUNCTION = money_minor_multiply
    );
    CREATE OPERATOR / (
        LEFTARG = money_minor,
        RIGHTARG = bigint,
        FUNCTION = money_minor_divide
    );
    CREATE OPERATOR - (
        RIGHTARG = money_minor,
        FUNCTION = money_minor_negate
    );
    ",
    name = "money_minor_sql_surface",
    requires = [
        money_minor,
        money_minor_add,
        money_minor_subtract,
        money_minor_multiply,
        money_minor_divide,
        money_minor_negate
    ]
);

#[cfg(not(feature = "fuzzing"))]
extension_sql!(
    r"
    ALTER OPERATOR FAMILY money_with_currency_hash_ops USING hash
        ADD FUNCTION 2 (money_with_currency, money_with_currency)
        money_with_currency_hash_extended(money_with_currency, bigint);

    ALTER TYPE money_with_currency SET (
        RECEIVE = money_with_currency_recv_safe,
        SEND = money_with_currency_send_safe
    );

    ALTER OPERATOR FAMILY money_minor_hash_ops USING hash
        ADD FUNCTION 2 (money_minor, money_minor)
        money_minor_hash_extended(money_minor, bigint);

    ALTER TYPE money_minor SET (
        RECEIVE = money_minor_recv_safe,
        SEND = money_minor_send_safe
    );
    ",
    name = "money_binary_and_hash_support",
    finalize
);

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
