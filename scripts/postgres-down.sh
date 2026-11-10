#!/usr/bin/env sh
set -eu

database="${POSTGRES_DB:-feuilles}"
provider="${POSTGRES_PROVIDER:-auto}"

data_dir=".local/postgres/data"
container="${database}_dev_postgres"

docker_running() {
  command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1
}

stop_docker() {
  if [ "$(docker inspect -f '{{.State.Running}}' "$container" 2>/dev/null || echo missing)" = "true" ]; then
    docker stop "$container" >/dev/null
    echo "stopped repo-local postgres"
    return 0
  fi
  return 1
}

stop_local() {
  if command -v pg_ctl >/dev/null 2>&1 &&
    [ -d "$data_dir" ] &&
    pg_ctl -D "$data_dir" status >/dev/null 2>&1; then
    pg_ctl -D "$data_dir" stop -m fast >/dev/null
    echo "stopped repo-local postgres"
    return 0
  fi
  return 1
}

case "$provider" in
docker)
  docker_running || {
    echo "POSTGRES_PROVIDER=docker but docker is not available or not running" >&2
    exit 1
  }
  stop_docker || echo "repo-local postgres is not running"
  ;;
local)
  stop_local || echo "repo-local postgres is not running"
  ;;
auto)
  stopped=1
  if docker_running && stop_docker; then stopped=0; fi
  if [ "$stopped" -ne 0 ] && stop_local; then stopped=0; fi
  [ "$stopped" -eq 0 ] || echo "repo-local postgres is not running"
  ;;
*)
  echo "unknown POSTGRES_PROVIDER '$provider' (expected auto, docker or local)" >&2
  exit 1
  ;;
esac
