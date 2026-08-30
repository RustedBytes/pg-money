\set id random(1, :ROWS)
SELECT n.*
FROM money_bench_values AS n
WHERE n.value = (
    SELECT probe.value FROM money_bench_values AS probe WHERE probe.id = :id
);
