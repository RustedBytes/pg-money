# Exchange-rate tables

`pg_money` never fetches or owns rates. `money_exchange_at` accepts a user-owned
relation with these columns:

```sql
from_currency text NOT NULL,
to_currency   text NOT NULL,
rate          numeric NOT NULL,
valid_at      timestamptz NOT NULL
```

Use uppercase ISO codes and positive rates where one source unit equals `rate`
target units. A suitable key is:

```sql
PRIMARY KEY (from_currency, to_currency, valid_at)
```

The function safely schema-qualifies the supplied `regclass`, selects the
newest exact pair with `valid_at <= as_of`, and errors when none exists. It does
not infer reverse rates. Restrict table access with ordinary PostgreSQL grants.
