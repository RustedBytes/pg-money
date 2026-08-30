# Exchange-rate tables

`pg_money` never fetches or owns rates. `money_exchange_at` accepts a user-owned
relation with these columns:

```sql
from_currency text NOT NULL,
to_currency   text NOT NULL,
rate          numeric NOT NULL,
valid_at      timestamptz NOT NULL
```

Use uppercase codes from `money_currencies()` or `money_crypto_currencies()`
and positive rates where one source unit equals `rate` target units. A suitable
key is:

```sql
PRIMARY KEY (from_currency, to_currency, valid_at)
```

The function safely schema-qualifies the supplied `regclass`, selects the
newest exact pair with `valid_at <= as_of`, and errors when none exists. It does
not infer reverse rates. Restrict table access with ordinary PostgreSQL grants.

`money_exchange_at` performs one indexed SPI lookup per call. The key above is
therefore required for predictable latency. For bulk conversion, join the rate
table once and call `money_exchange(value, to_currency, rate)` instead of
calling `money_exchange_at` for every row; this lets PostgreSQL plan the lookup
as a set operation and avoids one SPI invocation per input row.
