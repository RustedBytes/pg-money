# pg_money

`pg_money` is a PostgreSQL 14–18 extension implemented with pgrx and
[`rusty-money`](https://crates.io/crates/rusty-money). It provides a precise,
currency-aware `money_with_currency` base type, safe arithmetic, B-tree/hash
indexes, fair allocation, aggregation, formatting, and time-aware exchange-rate
lookups. It is deliberately separate from PostgreSQL's locale-sensitive
`pg_catalog.money` type.

## Build and install

Prerequisites are Rust 1.96 or newer, PostgreSQL development headers, and
`cargo-pgrx` 0.19.2.

```bash
cargo install cargo-pgrx --version 0.19.2 --locked
cargo pgrx init --pg18=/path/to/pg_config
./install.sh --pg-config /path/to/pg_config
```

Then install it in a database:

```sql
CREATE EXTENSION pg_money;
```

## Quick start

```sql
CREATE TABLE invoices (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    total money_with_currency NOT NULL
);

INSERT INTO invoices(total) VALUES
    ('USD 19.95'),
    (money_make(5.25, 'USD'));

SELECT sum(total), avg(total), money_format(sum(total)) FROM invoices;
-- USD 25.20 | USD 12.60 | $25.20

SELECT 'USD 10.00'::money_with_currency + 'USD 2.50'; -- USD 12.50
SELECT 'USD 10.00'::money_with_currency * 1.075;       -- USD 10.75
SELECT money_round('USD 10.005', 2, 'half_up');        -- USD 10.01
SELECT money_split('USD 10.00', 3);
-- {"USD 3.34","USD 3.33","USD 3.33"}
```

Amounts preserve decimal precision until explicitly rounded. Addition,
subtraction, `sum`, and `avg` reject mixed currencies. Sorting uses a stable
total order of currency code followed by amount.

## Exchange rates

Direct conversion accepts an explicit rate:

```sql
SELECT money_exchange('USD 100', 'EUR', 0.85); -- EUR 85.00
```

Historical lookup uses an application-owned table:

```sql
CREATE TABLE exchange_rates (
    from_currency text NOT NULL,
    to_currency text NOT NULL,
    rate numeric NOT NULL CHECK (rate > 0),
    valid_at timestamptz NOT NULL,
    PRIMARY KEY (from_currency, to_currency, valid_at)
);

SELECT money_exchange_at(
    'USD 100', 'EUR', 'exchange_rates'::regclass, CURRENT_TIMESTAMP
);
```

The lookup selects the newest exact currency pair at or before the requested
time. It never downloads rates or infers inverse rates.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings -W clippy::pedantic
cargo pgrx test pg18
cargo clippy --manifest-path fuzz/Cargo.toml --all-targets -- \
  -D warnings -W clippy::pedantic
```

For release-performance checks, install the release build into a disposable
PostgreSQL instance, set `DATABASE_URL`, and run:

```bash
BENCH_CLIENTS="4 16 64" BENCH_DURATION=30 ./bench/run.sh \
  10000 100000 1000000
```

The benchmark covers parsing, on-disk size, predicates, arithmetic, formatting,
SUM/AVG, equality, hashing, ordering, binary COPY, prepared indexed lookup, and
historical exchange lookup. It reports per-command latency and throughput at
each requested concurrency. Set `BENCH_JOBS` and `BENCH_PROGRESS` to tune the
pgbench worker count and progress interval.

For a completely isolated run with PostgreSQL 18 and pgbench included, build
and run the disposable benchmark image:

```bash
docker build -f dockerfile.bench -t pg-money-bench .
docker run --rm --shm-size=1g \
  -e BENCH_ROWS="100000 1000000" \
  -e BENCH_CLIENTS="16 64 128" \
  -e BENCH_JOBS=8 \
  -e BENCH_DURATION=120 \
  pg-money-bench
```

The container initializes a fresh database, installs the release extension,
runs every benchmark workload, prints pgbench latency and throughput, and then
shuts PostgreSQL down. Use a bind-mounted PostgreSQL 18 data directory only
when repeatable warm-data runs are intentional; the default ephemeral database
avoids stale benchmark state.

See [the SQL API](docs/API.md), [binary format](docs/BINARY_FORMAT.md), and
[exchange-rate contract](docs/EXCHANGE_RATES.md) for details.
