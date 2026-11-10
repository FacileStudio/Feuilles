#!/usr/bin/env sh
set -eu

host="127.0.0.1"
port="${POSTGRES_PORT:-5653}"
user="postgres"
password="postgres"
database="${POSTGRES_DB:-feuilles}"
image="${POSTGRES_IMAGE:-postgres:16-alpine}"
provider="${POSTGRES_PROVIDER:-auto}"

data_root=".local/postgres"
data_dir="$data_root/data"
log_file="$data_root/postgres.log"
container="${database}_dev_postgres"
volume="${database}_dev_pgdata"

has_docker() {
  command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1
}

has_local_postgres() {
  command -v initdb >/dev/null 2>&1 &&
    command -v pg_ctl >/dev/null 2>&1 &&
    command -v pg_isready >/dev/null 2>&1 &&
    command -v psql >/dev/null 2>&1 &&
    command -v createdb >/dev/null 2>&1
}

if [ "$provider" = "auto" ]; then
  if has_docker; then
    provider="docker"
  elif has_local_postgres; then
    provider="local"
  else
    cat >&2 <<EOF
no way to start a dev database.

Install either:
  - Docker (preferred, matches how this app is deployed), or
  - Postgres 16 client+server binaries on PATH, e.g. 'brew install postgresql@16'

Then re-run 'mise run db-up'. Force a provider with POSTGRES_PROVIDER=docker|local.
EOF
    exit 1
  fi
fi

start_docker() {
  has_docker || {
    echo "POSTGRES_PROVIDER=docker but docker is not available or not running" >&2
    exit 1
  }

  case "$(docker inspect -f '{{.State.Running}}' "$container" 2>/dev/null || echo missing)" in
  true)
    echo "postgres is already running on ${host}:${port}"
    ;;
  false)
    echo "starting existing postgres container on ${host}:${port}"
    docker start "$container" >/dev/null
    ;;
  *)
    echo "creating postgres container on ${host}:${port}"
    docker run -d \
      --name "$container" \
      -e POSTGRES_USER="$user" \
      -e POSTGRES_PASSWORD="$password" \
      -e POSTGRES_DB="$database" \
      -p "${host}:${port}:5432" \
      -v "${volume}:/var/lib/postgresql/data" \
      "$image" >/dev/null
    ;;
  esac

  attempt=0
  until docker exec "$container" pg_isready -U "$user" -d "$database" >/dev/null 2>&1; do
    attempt=$((attempt + 1))
    if [ "$attempt" -ge 60 ]; then
      echo "postgres did not become ready in 60s - check 'docker logs ${container}'" >&2
      exit 1
    fi
    sleep 1
  done
}

start_local() {
  has_local_postgres || {
    echo "POSTGRES_PROVIDER=local but postgres binaries are not on PATH" >&2
    exit 1
  }

  mkdir -p "$data_root"

  if [ ! -s "$data_dir/PG_VERSION" ]; then
    echo "initializing postgres data directory at $data_dir"
    initdb -D "$data_dir" -U "$user" -A trust >/dev/null
  fi

  if pg_ctl -D "$data_dir" status >/dev/null 2>&1; then
    echo "postgres is already running on ${host}:${port}"
  else
    echo "starting postgres on ${host}:${port}"
    pg_ctl -D "$data_dir" -l "$log_file" -o "-h ${host} -p ${port}" start >/dev/null
  fi

  attempt=0
  until pg_isready -h "$host" -p "$port" -U "$user" -d postgres >/dev/null 2>&1; do
    attempt=$((attempt + 1))
    if [ "$attempt" -ge 60 ]; then
      echo "postgres did not become ready in 60s - check $log_file" >&2
      exit 1
    fi
    sleep 1
  done

  if ! psql -h "$host" -p "$port" -U "$user" -d postgres -tAc \
    "SELECT 1 FROM pg_database WHERE datname = '${database}'" | grep -qx "1"; then
    createdb -h "$host" -p "$port" -U "$user" "$database"
  fi
}

case "$provider" in
docker) start_docker ;;
local) start_local ;;
*)
  echo "unknown POSTGRES_PROVIDER '$provider' (expected auto, docker or local)" >&2
  exit 1
  ;;
esac

echo "postgres is ready: postgres://${user}:${password}@${host}:${port}/${database}?sslmode=disable"
