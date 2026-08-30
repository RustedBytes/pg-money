use crate::errors::{fail_binary, fail_parameter};
use crate::model::money_with_currency;
use pgrx::datum::Internal;
use pgrx::prelude::*;
use pgrx::{AllocatedByRust, PgBox, PgMemoryContexts};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub(crate) struct MoneyAggregateState {
    count: i64,
    total: Option<money_with_currency>,
}

// PostgreSQL's `internal` pseudo-type keeps this state private to the aggregate
// executor. It is never a user-visible datum and is serialized only when a
// parallel worker hands partial state to another process.
impl_sql_translatable!(MoneyAggregateState, "internal");

fn aggregate_memory_context(fcinfo: pg_sys::FunctionCallInfo) -> PgMemoryContexts {
    let mut context = std::ptr::null_mut();
    // SAFETY: PostgreSQL supplies `fcinfo`; `AggCheckCallContext` only inspects
    // it and writes the aggregate memory context to the provided out-pointer.
    if unsafe { pg_sys::AggCheckCallContext(fcinfo, &raw mut context) } == 0 || context.is_null() {
        fail_parameter("aggregate support function called outside an aggregate");
    }
    PgMemoryContexts::For(context)
}

fn allocate_state(
    fcinfo: pg_sys::FunctionCallInfo,
    value: MoneyAggregateState,
) -> PgBox<MoneyAggregateState> {
    // SAFETY: the context was returned by `AggCheckCallContext`. The state has
    // no heap-owning fields, so PostgreSQL may reclaim it with the context.
    let mut state = unsafe {
        PgBox::<MoneyAggregateState, AllocatedByRust>::alloc0_in_context(aggregate_memory_context(
            fcinfo,
        ))
    };
    *state = value;
    state.into_pg_boxed()
}

fn merge_aggregate_value(state: &mut MoneyAggregateState, value: money_with_currency, count: i64) {
    state.total = Some(match state.total {
        Some(total) => total
            .checked_add(value)
            .unwrap_or_else(|error| fail_parameter(&error.to_string())),
        None => value,
    });
    state.count = state
        .count
        .checked_add(count)
        .unwrap_or_else(|| fail_parameter("aggregate count overflow"));
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_aggregate_transfn(
    state: Option<PgBox<MoneyAggregateState>>,
    value: Option<money_with_currency>,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<PgBox<MoneyAggregateState>> {
    let value = value?;
    let mut state = state.unwrap_or_else(|| allocate_state(fcinfo, MoneyAggregateState::default()));
    merge_aggregate_value(&mut state, value, 1);
    Some(state)
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_aggregate_combinefn(
    left: Option<PgBox<MoneyAggregateState>>,
    right: Option<PgBox<MoneyAggregateState>>,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<PgBox<MoneyAggregateState>> {
    let Some(right) = right else {
        return left;
    };
    let Some(total) = right.total else {
        return left;
    };
    let mut left = left.unwrap_or_else(|| allocate_state(fcinfo, MoneyAggregateState::default()));
    merge_aggregate_value(&mut left, total, right.count);
    Some(left)
}

#[pg_extern(immutable, strict, parallel_safe)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PostgreSQL aggregate serialization receives an owned internal datum"
)]
pub(crate) fn money_aggregate_serialfn(state: PgBox<MoneyAggregateState>) -> Vec<u8> {
    serde_cbor::to_vec(&*state)
        .unwrap_or_else(|error| fail_binary(&format!("could not serialize aggregate: {error}")))
}

#[pg_extern(immutable, parallel_safe)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PostgreSQL's DESERIALFUNC ABI passes an owned bytea datum"
)]
pub(crate) fn money_aggregate_deserialfn(
    bytes: Vec<u8>,
    _internal: Internal,
    fcinfo: pg_sys::FunctionCallInfo,
) -> PgBox<MoneyAggregateState> {
    let state: MoneyAggregateState = serde_cbor::from_slice(&bytes)
        .unwrap_or_else(|error| fail_binary(&format!("invalid aggregate state: {error}")));
    if state.count <= 0 || state.total.is_none() {
        fail_binary("invalid aggregate state invariants");
    }
    allocate_state(fcinfo, state)
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_sum_finalfn(
    state: Option<PgBox<MoneyAggregateState>>,
) -> Option<money_with_currency> {
    state.and_then(|state| state.total)
}

#[pg_extern(immutable, parallel_safe)]
pub(crate) fn money_avg_finalfn(
    state: Option<PgBox<MoneyAggregateState>>,
) -> Option<money_with_currency> {
    state.and_then(|state| {
        state.total.map(|total| {
            total
                .checked_div(Decimal::from(state.count))
                .unwrap_or_else(|error| fail_parameter(&error.to_string()))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::parse_value;

    #[test]
    fn aggregate_state_cbor_round_trip_uses_validated_money_decoder() {
        let state = MoneyAggregateState {
            count: 1,
            total: Some(parse_value("USD 123.45").unwrap()),
        };
        let encoded = serde_cbor::to_vec(&state).unwrap();
        let decoded: MoneyAggregateState = serde_cbor::from_slice(&encoded).unwrap();
        assert_eq!(decoded.count, 1);
        assert_eq!(decoded.total.unwrap().canonical(), "USD 123.45");
    }
}
