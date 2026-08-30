# Upgrade policy

Version 0.1.0 is the first `pg_money` release and intentionally has no upgrade
path from the repository's unrelated predecessor.

Future releases follow semantic versioning for SQL and storage contracts.
Additive APIs require an extension update script. Storage changes must retain a
decoder for earlier versions or document an explicit table rewrite. Equality,
ordering, or hashing changes require a major release and mandatory index rebuild.
