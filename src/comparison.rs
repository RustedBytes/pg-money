use crate::errors::fail_parameter;
use crate::model::money_with_currency;
use pgrx::prelude::*;
use rusty_money::MoneyError;
use std::cmp::Ordering;

pub(crate) fn unwrap_ordering(result: Result<Ordering, MoneyError>) -> i32 {
    match result.unwrap_or_else(|error| fail_parameter(&error.to_string())) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

/// Compare amounts using rusty-money semantics, rejecting mixed currencies.
#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_compare(left: money_with_currency, right: money_with_currency) -> i32 {
    unwrap_ordering(left.as_money().compare(&right.as_money()))
}
