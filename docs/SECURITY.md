# Security model

The extension performs no network or filesystem access. Currency metadata is
compiled into `rusty-money`; exchange rates come only from a caller-supplied
`regclass`. Relation names are schema-qualified and identifier-quoted before
SPI queries, while currencies and timestamps are passed as bound parameters.

Text and binary inputs are bounded and validated. Arithmetic errors, division
by zero, mixed currencies, invalid rates, and decimal overflow are reported as
PostgreSQL errors rather than silently converting to floating point.
