# SQL API

## Type and construction

- `money_with_currency`: normalized decimal amount plus ISO-4217 currency.
- Canonical text: `USD 123.45`; codes are accepted case-insensitively and output uppercase.
- `money_make(numeric, text)`, `money_parse(text)`, `money_try_parse(text)`.
- `money_parse_localized(amount, currency)` and its non-throwing
  `money_try_parse_localized` counterpart parse the currency's rusty-money
  locale without making base-type input locale-dependent.
- `money_from_minor(bigint, currency)`, strict `money_to_minor(value)`, and
  truncating `money_to_minor_lossy(value)` bridge decimal and minor units.
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
money_to_rusty_json(value) -> jsonb
money_from_rusty_json(jsonb) -> money_with_currency
```

`money_to_rusty_json` and `money_from_rusty_json` use rusty-money's exact
`{"amount":"123.45","currency":"USD"}` serde contract. The older
`money_to_json` additionally includes the formatted display string.

`money_format_with(value, options_jsonb)` starts with the currency's locale and
accepts validated overrides for `digit_separator`, `exponent_separator`,
`separator_pattern`, `positions`, `rounding`, `include_symbol`, and
`include_code`. Positions are `sign`, `symbol`, `amount`, `space`, or `code`.

## Minor-unit fast path

`money_minor` stores an `i64` count of currency minor units and delegates
arithmetic to rusty-money's `FastMoney`. Its canonical text remains `USD 12.34`.

```text
money_minor_make(bigint, text) -> money_minor
money_minor_units(value)       -> bigint
money_minor_currency(value)    -> text
money_minor_format(value)      -> text
```

The type supports `+`, `-`, unary `-`, `* bigint`, `/ bigint`, predicates,
absolute value, B-tree/hash indexes, and explicit casts to and from
`money_with_currency`. Conversion to `money_minor` is strict: fractional minor
units and values outside `i64` fail. `money_to_minor_lossy` truncates fractional
minor units but still reports overflow. Division follows `FastMoney` and
truncates toward zero.

## Arithmetic and allocation

`money_add`/`+` and `money_subtract`/`-` require matching currencies.
`money_multiply`/`* numeric` and `money_divide`/`/ numeric` preserve currency.
Unary `-`, `money_negate`, and `money_abs` are also available.

`money_round(value, digits, money_rounding)` supports `half_up`, `half_down`,
and `half_even`. `money_split(value, parts)` and
`money_allocate(value, positive_integer_weights[])` return arrays whose minor
units are distributed using `rusty-money`. Each call is capped at 10,000 output
parts to prevent a single statement from requesting unbounded backend memory.

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

`money_currency_info(code)` returns JSON metadata. `money_currencies()` returns
the complete compiled ISO catalog with `code`, `numeric_code`, `exponent`,
`minor_units`, `name`, `symbol`, `locale`, and `symbol_first` columns.

```text
money_storage_version()   -> integer
money_binary_format()     -> text
money_extension_version() -> text
```
