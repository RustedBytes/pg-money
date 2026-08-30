# Extension SQL

The current base schema is generated from the pgrx entity graph during packaging.
Every additive release must add an explicit `pg_money--old--new.sql` upgrade
script here.

- `pg_money--0.2.0--0.3.0.sql` adds the strict comparison and direct
  `money_minor` construction/serde functions without changing stored values or
  operator classes.
