\set id random(1, :ROWS)
SELECT money_amount(value),
       money_currency(value),
       money_is_positive((value + 'USD 1.00'::money_with_currency) * 1.075)
FROM money_bench_values
WHERE id = :id;
