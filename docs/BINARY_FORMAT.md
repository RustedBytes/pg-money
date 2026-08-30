# Stable storage and binary protocol

## Logical identity

Values are normalized before storage. Equality and hashing use `(currency,
decimal amount)`. Ordering uses currency code first and amount second. Decimal
input scale and storage-version bytes do not affect logical identity.

## CBOR payload version 1

`money_with_currency` on-disk pgrx storage and PostgreSQL binary send/receive
encode a four-element CBOR array:

```text
[
  version:       unsigned integer (currently 1),
  currency:      uppercase ISO-4217 text,
  mantissa:      array of 16 unsigned bytes in signed i128 network byte order,
  scale:         unsigned integer (0..28)
]
```

`money_minor` uses the same version and currency rules with a three-element
array:

```text
[
  version:       unsigned integer (currently 1),
  currency:      uppercase ISO-4217 text,
  minor_units:   signed 64-bit integer
]
```

The decoder limits payloads to 128 bytes and rejects unknown versions,
currencies, invalid decimals, and noncanonical components before values enter
PostgreSQL. Both types install bounded validated receive functions.
`money_binary_format()` returns `cbor-array-v1`.

Drivers without this binary codec should request text format, whose canonical
representation is `<ISO code> <decimal amount>`.
