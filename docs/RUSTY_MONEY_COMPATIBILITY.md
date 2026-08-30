# rusty-money compatibility

`pg_money` pins `rusty-money` 0.5.0 with its default ISO catalog plus the
`crypto`, `fast`, and `serde` features. The extension exposes runtime money
behavior through SQL while keeping PostgreSQL storage, indexing, and resource
safety contracts explicit.

| rusty-money capability | Extension status | SQL surface or policy |
| --- | --- | --- |
| ISO currencies and metadata | Exact catalog | `money_currencies()`, `money_currency_info()` |
| Crypto currencies and metadata | Exact 14-entry catalog | `money_crypto_currencies()`, `money_currency_info()` |
| `Money` construction and accessors | Direct | `money_make`, parsing, amount/currency accessors |
| Locale-aware parsing | Direct, explicit | `money_parse_localized`; base-type input remains locale independent |
| Predicates, absolute value, and negation | Direct | Decimal and `money_minor` functions/operators |
| Decimal arithmetic | Compatible and checked | Currency mismatch, division by zero, and overflow are PostgreSQL errors |
| Rounding | Exact | `half_up`, `half_down`, and `half_even` |
| Split | Exact algorithm, bounded | At most 10,000 output values per call |
| Allocation | Exact for positive weights, bounded | Zero/negative weights are rejected; at most 10,000 outputs |
| Formatting and `Params` | Direct | `money_format` and validated `money_format_with` options |
| Exchange conversion | SQL-native superset | Explicit positive rate or historical application-owned table |
| `FastMoney` | Direct | `money_minor`, including major/minor constructors and checked arithmetic |
| Strict/lossy `Money` to `FastMoney` conversion | Direct and safer | Both report overflow; strict conversion also reports precision loss |
| `Money` serde | Exact JSON contract | `money_to_rusty_json`, `money_from_rusty_json` |
| `FastMoney` serde | Exact JSON contract | `money_minor_to_rusty_json`, `money_minor_from_rusty_json` |
| Same-currency comparison | Exact | `money_compare`, `money_minor_compare` return -1, 0, or 1 |
| Mixed-currency comparison | Both contracts available | Strict functions reject it; SQL operators use a stable total order |
| Numeric ISO lookup | SQL-native equivalent | Filter `money_currencies()` by `numeric_code` |
| `to_f64_lossy` | SQL-native equivalent | `money_amount(value)::double precision` |
| In-memory `Exchange` map | SQL-native equivalent | Application-owned exchange-rate table |
| Compile-time `define_currency_set!` | Not exposed | Rust-only compile-time facility; the SQL type uses a stable compiled catalog |

## Intentional production deviations

- SQL equality, ordering, and indexes use currency code followed by amount.
  Changing that ordering would invalidate B-tree and hash contracts. Use
  `money_compare` or `money_minor_compare` when rusty-money's mixed-currency
  rejection is required.
- Allocation accepts positive weights only. Although rusty-money accepts a zero
  weight when the total is nonzero, remainder distribution can make zero-share
  behavior surprising. The SQL API rejects it rather than silently assigning
  an unintended minor unit.
- Split and allocation are capped at 10,000 results to prevent one statement
  from requesting unbounded backend memory.
- Lossy minor conversion truncates excess precision but still reports i64
  overflow. It never turns an overflow into a valid zero amount.
- Exchange rates must be positive, and source and target currencies must differ.

These differences are stable extension contracts, not missing behavior.
