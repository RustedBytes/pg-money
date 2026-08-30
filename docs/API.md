# SQL API

## Type and construction

- `money_with_currency`: normalized decimal amount plus ISO-4217 currency.
- Canonical text: `USD 123.45`; codes are accepted case-insensitively and output uppercase.
- `money_make(numeric, text)`, `money_parse(text)`, `money_try_parse(text)`.
- Explicit casts from `text`/`varchar`; assignment casts back to text/varchar.
- No cast to or from PostgreSQL's built-in `money` type.

## Inspection and display

```text
money_amount(value)       -> numeric
money_currency(value)     -> text
money_is_zero(value)      -> boolean
money_is_positive(value)  -> boolean
money_is_negative(value)  -> boolean
money_format(value)       -> localized text
money_to_json(value)      -> jsonb
```

## Arithmetic and allocation

`money_add`/`+` and `money_subtract`/`-` require matching currencies.
`money_multiply`/`* numeric` and `money_divide`/`/ numeric` preserve currency.
Unary `-`, `money_negate`, and `money_abs` are also available.

`money_round(value, digits, money_rounding)` supports `half_up`, `half_down`,
and `half_even`. `money_split(value, parts)` and
`money_allocate(value, positive_integer_weights[])` return arrays whose minor
units are distributed using `rusty-money`.

`sum(money_with_currency)` and `avg(money_with_currency)` ignore NULLs, return
NULL for empty groups, and reject mixed currencies.

## Comparison and indexes

Equality is normalized currency plus decimal amount. Ordering is ISO code then
amount. Default B-tree and hash operator classes support unique constraints,
joins, sorting, hash indexes, and hash partitioning.

## Exchange

```text
money_exchange(value, target_currency, rate)
money_exchange_at(value, target_currency, rate_table,
                  as_of = CURRENT_TIMESTAMP)
```

Rates must be positive. Source and target must differ. Table lookup uses only
the latest exact pair at or before `as_of`; see [EXCHANGE_RATES.md](EXCHANGE_RATES.md).

## Introspection

```text
money_storage_version()   -> integer
money_binary_format()     -> text
money_extension_version() -> text
```
