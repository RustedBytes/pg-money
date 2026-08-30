# Verification map

| Contract | Evidence |
| --- | --- |
| Canonical currency-aware storage | Unit/property round trips and pgrx cast tests |
| Arithmetic and currency safety | Addition, multiplication, mismatch, rounding, and allocation tests |
| Index semantics | Unique, B-tree, hash, ordering, and partition tests |
| Aggregation | SUM/AVG NULL, same-currency, and mixed-currency tests |
| Exchange lookup | Direct and historical exact-pair tests |
| Binary safety | Stable encoding fixture, validation, and decoder fuzz target |
