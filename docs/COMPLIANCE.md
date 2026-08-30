# Verification map

| Contract | Evidence |
| --- | --- |
| Canonical currency-aware storage | Unit/property round trips and pgrx cast tests |
| Arithmetic and currency safety | Addition, multiplication, mismatch, rounding, and allocation tests |
| Strict rusty-money comparison | Same-currency results, mixed-currency errors, and total-order coexistence tests |
| FastMoney interoperability | Major/minor construction, exact serde, overflow, and i64 property tests |
| rusty-money differential matrix | ISO/crypto exponents, arithmetic, rounding, allocation, and minor conversion properties |
| Index semantics | Unique, B-tree, hash, ordering, and partition tests |
| Aggregation | SUM/AVG NULL, same-currency, and mixed-currency tests |
| Exchange lookup | Direct and historical exact-pair tests |
| Binary safety | Stable encoding fixture, validation, and decoder fuzz target |
| Release upgrade | Package-content check and live 0.2.0-to-0.3.0 extension update test |
