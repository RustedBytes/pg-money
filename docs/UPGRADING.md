# Upgrade policy

Version 0.1.0 is the first `pg_money` release and intentionally has no upgrade
path from the repository's unrelated predecessor.

Version 0.3.0 adds currency-strict comparison and direct `money_minor`
construction/serde helpers. It does not change storage, equality, ordering, or
hashing. Upgrade an installed 0.2.0 extension with:

```sql
ALTER EXTENSION pg_money UPDATE TO '0.3.0';
```

Future releases follow semantic versioning for SQL and storage contracts.
Additive APIs require an extension update script. Storage changes must retain a
decoder for earlier versions or document an explicit table rewrite. Equality,
ordering, or hashing changes require a major release and mandatory index rebuild.
