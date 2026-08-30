# Support policy

`pg_money` 0.2 supports PostgreSQL 14–18, pgrx 0.19.2, Rust 1.96+, and
`rusty-money` 0.5.0's ISO and crypto currency sets. Custom application-defined
Rust currency sets are not exposed through the PostgreSQL extension.

Dependency updates that change currency metadata, formatting, arithmetic, or
rounding require regression review. Storage, equality, ordering, and hashing
changes require an explicit migration and index-rebuild policy.
