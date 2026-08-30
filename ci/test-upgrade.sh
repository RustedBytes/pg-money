#!/usr/bin/env bash
set -euo pipefail

if [[ "${CI:-}" != "true" && "${PG_MONEY_ALLOW_UPGRADE_TEST:-}" != "1" ]]; then
    echo "refusing to modify a PostgreSQL instance outside CI; set PG_MONEY_ALLOW_UPGRADE_TEST=1" >&2
    exit 1
fi

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"

pg_label="${1:-pg18}"
if [[ ! "$pg_label" =~ ^pg(1[4-8])$ ]]; then
    echo "expected a PostgreSQL label from pg14 through pg18" >&2
    exit 1
fi
pg_major="${BASH_REMATCH[1]}"
pg_port="$((28800 + pg_major))"
pg_config_path="$(cargo pgrx info pg-config "$pg_major")"
pg_bin="$(dirname "$pg_config_path")"
extension_dir="$($pg_config_path --sharedir)/extension"
old_schema="$extension_dir/pg_money--0.2.0.sql"
database="pg_money_upgrade_test"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/pg-money-upgrade.XXXXXX")"
current_schema="$temp_dir/pg_money--0.3.0.sql"
database_created=false
schema_created=false

cleanup() {
    if [[ "$database_created" == true ]]; then
        "$pg_bin/dropdb" -h localhost -p "$pg_port" "$database" || true
    fi
    if [[ "$schema_created" == true ]]; then
        rm -f -- "$old_schema"
    fi
    rm -rf -- "$temp_dir"
}
trap cleanup EXIT

if [[ -e "$old_schema" ]]; then
    echo "refusing to overwrite existing $old_schema" >&2
    exit 1
fi

cargo pgrx install --pg-config "$pg_config_path"
cargo pgrx schema "$pg_label" --out "$current_schema"

schema_created=true
awk '
BEGIN { capture = 0; block = "" }
$0 == "/* <begin connected objects> */" { capture = 1; block = $0 ORS; next }
capture {
    block = block $0 ORS
    if ($0 == "/* </end connected objects> */") {
        if (block !~ /comparison::money_compare|minor::money_minor_compare|minor::money_minor_from_major|minor::money_minor_to_rusty_json|minor::money_minor_from_rusty_json/) {
            printf "%s", block
        }
        capture = 0
        block = ""
    }
    next
}
{ print }
END { if (capture) exit 2 }
' "$current_schema" > "$old_schema"

if grep -Eq 'CREATE  FUNCTION "money_compare"|CREATE  FUNCTION "money_minor_compare"|CREATE  FUNCTION "money_minor_from_major"|CREATE  FUNCTION "money_minor_to_rusty_json"|CREATE  FUNCTION "money_minor_from_rusty_json"' "$old_schema"; then
    echo "temporary 0.2.0 schema still contains 0.3.0 functions" >&2
    exit 1
fi

cargo pgrx start "$pg_label"
"$pg_bin/createdb" -h localhost -p "$pg_port" "$database"
database_created=true

"$pg_bin/psql" -X -v ON_ERROR_STOP=1 -h localhost -p "$pg_port" -d "$database" <<'SQL'
CREATE EXTENSION pg_money VERSION '0.2.0';
DO $$
BEGIN
    IF (SELECT extversion <> '0.2.0' FROM pg_extension WHERE extname = 'pg_money') THEN
        RAISE EXCEPTION 'expected extension version 0.2.0';
    END IF;
    IF to_regprocedure('money_compare(money_with_currency,money_with_currency)') IS NOT NULL THEN
        RAISE EXCEPTION '0.3.0 function exists before upgrade';
    END IF;
END
$$;

CREATE TABLE upgrade_values(value money_with_currency PRIMARY KEY);
INSERT INTO upgrade_values VALUES ('USD 1.25'), ('BTC 1.00000001');
ALTER EXTENSION pg_money UPDATE TO '0.3.0';

DO $$
BEGIN
    IF (SELECT extversion <> '0.3.0' FROM pg_extension WHERE extname = 'pg_money') THEN
        RAISE EXCEPTION 'expected extension version 0.3.0';
    END IF;
    IF money_compare('USD 1', 'USD 2') <> -1 THEN
        RAISE EXCEPTION 'strict comparison failed after upgrade';
    END IF;
    IF money_minor_units(money_minor_from_major(10, 'USD')) <> 1000 THEN
        RAISE EXCEPTION 'FastMoney constructor failed after upgrade';
    END IF;
    IF money_minor_to_rusty_json(money_minor_from_major(10, 'USD')) <>
       '{"amount":"10.00","currency":"USD"}'::jsonb THEN
        RAISE EXCEPTION 'FastMoney serde failed after upgrade';
    END IF;
    IF (SELECT count(*) FROM upgrade_values) <> 2 THEN
        RAISE EXCEPTION 'stored values were not preserved by upgrade';
    END IF;
END
$$;

SET enable_seqscan = off;
DO $$
BEGIN
    IF (SELECT count(*) FROM upgrade_values WHERE value = 'BTC 1.00000001') <> 1 THEN
        RAISE EXCEPTION 'pre-upgrade index lookup failed after upgrade';
    END IF;
END
$$;
RESET enable_seqscan;
SQL
