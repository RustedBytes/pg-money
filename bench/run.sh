#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
database_url="${DATABASE_URL:-postgres:///postgres}"
duration="${BENCH_DURATION:-30}"
copy_file="$(mktemp "${TMPDIR:-/tmp}/pg-money-copy.XXXXXX")"
trap 'rm -f "$copy_file"' EXIT

if [[ $# -gt 0 ]]; then
    sizes=("$@")
else
    sizes=(10000 100000 1000000)
fi

for rows in "${sizes[@]}"; do
    echo "benchmark rows=$rows"
    psql "$database_url" -X -v ON_ERROR_STOP=1 -v ROWS="$rows" \
        -f "$project_dir/bench/benchmark.sql"

    echo "COPY BINARY export"
    time psql "$database_url" -X -qAt \
        -c "COPY money_bench_values TO STDOUT WITH (FORMAT binary)" > "$copy_file"

    echo "COPY BINARY import"
    time psql "$database_url" -X -q \
        -c "COPY money_bench_copy FROM STDIN WITH (FORMAT binary)" < "$copy_file"

    echo "indexed lookup via pgbench"
    pgbench "$database_url" -n -M prepared -T "$duration" -c 4 -j 2 \
        -D ROWS="$rows" -f "$project_dir/bench/lookup.sql"
done
