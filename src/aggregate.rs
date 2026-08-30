use crate::arithmetic::money_add;
use crate::errors::{fail_input, fail_parameter};
use crate::model::money_with_currency;
use crate::numeric::unwrap_money;
use pgrx::StringInfo;
use pgrx::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PostgresType)]
#[inoutfuncs]
#[pg_binary_protocol]
// pgrx uses the Rust identifier as the SQL transition-state type name.
#[allow(non_camel_case_types)]
pub struct money_aggregate_state {
    count: i64,
    total: Option<money_with_currency>,
}

impl InOutFuncs for money_aggregate_state {
    fn input(input: &CStr) -> Self {
        serde_json::from_slice(input.to_bytes())
            .unwrap_or_else(|error| fail_input(&format!("invalid aggregate state: {error}")))
    }

    fn output(&self, buffer: &mut StringInfo) {
        buffer.push_str(
            &serde_json::to_string(self)
                .unwrap_or_else(|error| fail_input(&format!("invalid aggregate state: {error}"))),
        );
    }
}

fn merge_aggregate_value(
    mut state: money_aggregate_state,
    value: money_with_currency,
    count: i64,
) -> money_aggregate_state {
    state.total = Some(match state.total {
        Some(total) => money_add(total, value),
        None => value,
    });
    state.count = state
        .count
        .checked_add(count)
        .unwrap_or_else(|| fail_parameter("aggregate count overflow"));
    state
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_aggregate_transfn(
    state: Option<money_aggregate_state>,
    value: Option<money_with_currency>,
) -> money_aggregate_state {
    let state = state.unwrap_or_default();
    match value {
        Some(value) => merge_aggregate_value(state, value, 1),
        None => state,
    }
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_aggregate_combinefn(
    left: Option<money_aggregate_state>,
    right: Option<money_aggregate_state>,
) -> money_aggregate_state {
    let left = left.unwrap_or_default();
    if let Some(state) = right
        && let Some(total) = state.total
    {
        return merge_aggregate_value(left, total, state.count);
    }
    left
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_sum_finalfn(
    state: Option<money_aggregate_state>,
) -> Option<money_with_currency> {
    state.and_then(|state| state.total)
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_avg_finalfn(
    state: Option<money_aggregate_state>,
) -> Option<money_with_currency> {
    state.and_then(|state| {
        state
            .total
            .map(|total| unwrap_money(total.as_money().div(Decimal::from(state.count))))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::parse_value;

    #[test]
    fn aggregate_state_json_round_trip_uses_validated_money_decoder() {
        let state = money_aggregate_state {
            count: 1,
            total: Some(parse_value("USD 123.45").unwrap()),
        };
        let encoded = serde_json::to_vec(&state).unwrap();
        let decoded: money_aggregate_state = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.count, 1);
        assert_eq!(decoded.total.unwrap().canonical(), "USD 123.45");
    }
}
