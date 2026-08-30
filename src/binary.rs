use crate::errors::fail_binary;
use crate::model::{MAX_BINARY_BYTES, money_with_currency};
use pgrx::datum::Internal;
use pgrx::prelude::*;

#[pg_extern(immutable, strict, parallel_safe)]
pub(crate) fn money_with_currency_send_safe(value: money_with_currency) -> Vec<u8> {
    serde_cbor::to_vec(&value)
        .unwrap_or_else(|error| fail_binary(&format!("could not encode value: {error}")))
}

#[pg_extern(immutable, strict, parallel_safe)]
pub(crate) fn money_with_currency_recv_safe(mut internal: Internal) -> money_with_currency {
    // SAFETY: PostgreSQL invokes a base-type receive function with a valid,
    // backend-owned StringInfoData pointer. Bounds are checked before slicing.
    let buffer = unsafe { internal.get_mut::<pg_sys::StringInfoData>() }
        .unwrap_or_else(|| fail_binary("missing protocol buffer"));
    if buffer.cursor < 0 || buffer.len < buffer.cursor {
        fail_binary("invalid protocol cursor");
    }
    let remaining = (buffer.len - buffer.cursor) as usize;
    if remaining == 0 || remaining > MAX_BINARY_BYTES {
        fail_binary(&format!(
            "payload length must be between 1 and {MAX_BINARY_BYTES} bytes"
        ));
    }
    // SAFETY: StringInfoData guarantees `data` is valid for `len` bytes.
    let bytes = unsafe {
        std::slice::from_raw_parts(
            buffer.data.add(buffer.cursor as usize).cast::<u8>(),
            remaining,
        )
    };
    let decoded =
        serde_cbor::from_slice(bytes).unwrap_or_else(|error| fail_binary(&error.to_string()));
    buffer.cursor = buffer.len;
    decoded
}

#[pg_extern(immutable, strict, parallel_safe)]
pub(crate) fn money_with_currency_hash_extended(value: money_with_currency, seed: i64) -> i64 {
    let mut hash = pgrx::misc::pgrx_seahash(&value) ^ seed as u64;
    hash = hash.wrapping_add(0x9e37_79b9_7f4a_7c15);
    hash = (hash ^ (hash >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash = (hash ^ (hash >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (hash ^ (hash >> 31)) as i64
}
