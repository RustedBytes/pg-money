#!/usr/bin/env bash
set -Eeuo pipefail

postgres_pid=""

stop_postgres() {
    if [[ -n "$postgres_pid" ]] && kill -0 "$postgres_pid" 2>/dev/null; then
        kill -INT "$postgres_pid"
        wait "$postgres_pid" || true
    fi
}

trap stop_postgres EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

is_positive_integer() {
    [[ "$1" =~ ^[1-9][0-9]*$ ]]
}

read -r -a benchmark_rows <<< "${BENCH_ROWS:-10000 100000 1000000}"
read -r -a benchmark_clients <<< "${BENCH_CLIENTS:-4 16 64}"
if [[ ${#benchmark_rows[@]} -eq 0 || ${#benchmark_clients[@]} -eq 0 ]]; then
    echo "BENCH_ROWS and BENCH_CLIENTS must not be empty" >&2
    exit 2
fi
for value in "${benchmark_rows[@]}" "${benchmark_clients[@]}" \
    "${BENCH_JOBS:-4}" "${BENCH_DURATION:-30}" "${BENCH_PROGRESS:-10}" \
    "${PG_MAX_CONNECTIONS:-256}"; do
    if ! is_positive_integer "$value"; then
        echo "benchmark settings must be positive integers; got: $value" >&2
        exit 2
    fi
done

if [[ -z "${POSTGRES_PASSWORD:-}" && -z "${POSTGRES_HOST_AUTH_METHOD:-}" ]]; then
    read -r generated_uuid < /proc/sys/kernel/random/uuid
    export POSTGRES_PASSWORD="benchmark-$generated_uuid"
fi
if [[ -z "${DATABASE_URL:-}" ]]; then
    if [[ -n "${POSTGRES_PASSWORD:-}" ]]; then
        export DATABASE_URL="postgresql://${POSTGRES_USER:-postgres}:${POSTGRES_PASSWORD}@127.0.0.1:5432/${POSTGRES_DB:-pg_money_bench}"
    else
        export DATABASE_URL="postgresql://${POSTGRES_USER:-postgres}@127.0.0.1:5432/${POSTGRES_DB:-pg_money_bench}"
    fi
fi

postgres_args=(
    postgres
    -c "shared_buffers=${PG_SHARED_BUFFERS:-512MB}"
    -c "max_connections=${PG_MAX_CONNECTIONS:-256}"
    -c track_io_timing=on
)
if [[ $# -gt 0 ]]; then
    postgres_args+=("$@")
fi

docker-entrypoint.sh "${postgres_args[@]}" &
postgres_pid="$!"

ready=false
for _ in {1..120}; do
    if pg_isready \
        --host 127.0.0.1 \
        --port 5432 \
        --username "${POSTGRES_USER:-postgres}" \
        --dbname "${POSTGRES_DB:-pg_money_bench}" >/dev/null 2>&1; then
        ready=true
        break
    fi
    if ! kill -0 "$postgres_pid" 2>/dev/null; then
        wait "$postgres_pid"
        exit 1
    fi
    sleep 1
done

if [[ "$ready" != true ]]; then
    echo "PostgreSQL did not become ready within 120 seconds" >&2
    exit 1
fi

/bench/run.sh "${benchmark_rows[@]}"
