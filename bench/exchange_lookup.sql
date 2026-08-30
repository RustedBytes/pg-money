\set id random(1, :ROWS)
SELECT money_exchange_at(
           value,
           'EUR',
           'money_bench_rates'::regclass,
           CURRENT_TIMESTAMP
       )
FROM money_bench_values
WHERE id = :id;
