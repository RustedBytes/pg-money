\set ON_ERROR_STOP on
\timing on

CREATE EXTENSION IF NOT EXISTS pg_money;
DROP TABLE IF EXISTS money_bench_raw, money_bench_values, money_bench_copy,
                     money_bench_rates;

CREATE UNLOGGED TABLE money_bench_raw AS
SELECT g AS id,
       'USD ' || ((g % 100000000)::numeric / 100)::text AS raw
FROM generate_series(1, :ROWS) AS g;

\echo canonical parsing
SELECT count(money_parse(raw)) FROM money_bench_raw;

\echo invalid non-throwing parsing
SELECT count(money_try_parse('invalid-' || id::text)) FROM money_bench_raw;

\echo bulk insert and indexes
CREATE UNLOGGED TABLE money_bench_values(
    id bigint PRIMARY KEY,
    value money_with_currency NOT NULL
);
INSERT INTO money_bench_values SELECT id, money_parse(raw) FROM money_bench_raw;
CREATE UNIQUE INDEX money_bench_value_idx ON money_bench_values(value);
CREATE INDEX money_bench_value_hash_idx ON money_bench_values USING hash(value);
ANALYZE money_bench_values;

\echo storage footprint
SELECT avg(pg_column_size(value)) AS average_value_bytes,
       pg_size_pretty(pg_total_relation_size('money_bench_values')) AS table_and_indexes;

\echo predicates and accessors
SELECT count(*) FILTER (WHERE money_is_positive(value)),
       count(*) FILTER (WHERE money_is_zero(value)),
       count(DISTINCT money_currency(value))
FROM money_bench_values;

\echo arithmetic and formatting hot path
SELECT count(money_format(value * 1.075)) FROM money_bench_values;
SELECT count((value + money_parse('USD 1.00')) - money_parse('USD 1.00'))
FROM money_bench_values;

\echo parallel-capable aggregate
SELECT sum(value), avg(value) FROM money_bench_values;

\echo equality hashing and ordering
SELECT count(*) FROM money_bench_values WHERE value = value;
SET enable_sort = off;
SET enable_indexscan = off;
SET enable_indexonlyscan = off;
SELECT count(*) FROM (
    SELECT value FROM money_bench_values GROUP BY value
) AS grouped_values;
RESET enable_sort;
RESET enable_indexscan;
RESET enable_indexonlyscan;
SELECT value FROM money_bench_values ORDER BY value LIMIT 1000;

\echo btree indexed lookup
EXPLAIN (ANALYZE, BUFFERS, COSTS OFF)
SELECT * FROM money_bench_values WHERE value = money_parse('USD 555.17');

\echo prepare COPY source
CREATE UNLOGGED TABLE money_bench_copy AS TABLE money_bench_values WITH NO DATA;

\echo prepare indexed historical rates
CREATE UNLOGGED TABLE money_bench_rates(
    from_currency text NOT NULL,
    to_currency text NOT NULL,
    rate numeric NOT NULL CHECK (rate > 0),
    valid_at timestamptz NOT NULL,
    PRIMARY KEY (from_currency, to_currency, valid_at)
);
INSERT INTO money_bench_rates
SELECT 'USD', 'EUR', 0.80 + (g % 100)::numeric / 10000,
       CURRENT_TIMESTAMP - (g || ' hours')::interval
FROM generate_series(1, 10000) AS g;
ANALYZE money_bench_rates;
