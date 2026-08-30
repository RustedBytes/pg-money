\set ON_ERROR_STOP on
\timing on

CREATE EXTENSION IF NOT EXISTS pg_money;
DROP TABLE IF EXISTS money_bench_raw, money_bench_values, money_bench_copy;

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

\echo arithmetic and formatting
SELECT count(money_format(value * 1.075)) FROM money_bench_values;

\echo aggregate
SELECT sum(value), avg(value) FROM money_bench_values;

\echo equality and ordering
SELECT count(*) FROM money_bench_values WHERE value = value;
SELECT value FROM money_bench_values ORDER BY value LIMIT 1000;

\echo btree indexed lookup
EXPLAIN (ANALYZE, BUFFERS, COSTS OFF)
SELECT * FROM money_bench_values WHERE value = money_parse('USD 555.17');

\echo prepare COPY source
CREATE UNLOGGED TABLE money_bench_copy AS TABLE money_bench_values WITH NO DATA;
