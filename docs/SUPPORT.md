# Support policy

`pg_money` 0.1 supports PostgreSQL 14–18, pgrx 0.19.2, Rust 1.96+, and
`rusty-money` 0.5.0's ISO currency set. Cryptocurrency and custom currency sets
are not enabled.

Dependency updates that change currency metadata, formatting, arithmetic, or
rounding require regression review. Storage, equality, ordering, and hashing
changes require an explicit migration and index-rebuild policy.
