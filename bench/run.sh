#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
database_url="${DATABASE_URL:-postgres:///postgres}"
duration="${BENCH_DURATION:-30}"
client_list="${BENCH_CLIENTS:-4 16 64}"
jobs="${BENCH_JOBS:-4}"
progress="${BENCH_PROGRESS:-10}"
copy_file="$(mktemp "${TMPDIR:-/tmp}/pg-money-copy.XXXXXX")"
trap 'rm -f "$copy_file"' EXIT

read -r -a clients <<< "$client_list"

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

    for client_count in "${clients[@]}"; do
        run_jobs="$jobs"
        if (( run_jobs > client_count )); then
            run_jobs="$client_count"
        fi

        for workload in lookup hot_path exchange_lookup; do
            echo "pgbench workload=$workload clients=$client_count jobs=$run_jobs"
            pgbench "$database_url" -n -M prepared -T "$duration" \
                -c "$client_count" -j "$run_jobs" -P "$progress" -r \
                -D ROWS="$rows" -f "$project_dir/bench/$workload.sql"
        done
    done
done
