#!/usr/bin/env sh
# Checks a deployed single-container Facile app from the browser's side as well as the API's.
#
#   sh scripts/verify-deploy.sh feuilles [boot-path] [domain]
#
# A green /api/health says nothing about what the browser calls: that blind spot once let a
# broken front end reach production.

set -eu

app="${1:?usage: sh scripts/verify-deploy.sh <app> [boot-path] [domain]}"
boot="${2:-/api/auth/config}"
base="https://${app}.${3:-facile.studio}"
fails=0

# check <label> <expected> <actual>
check() {
  if [ "$2" = "$3" ]; then
    printf '  ok    %-46s %s\n' "$1" "$3"
  else
    printf '  FAIL  %-46s %s (expected %s)\n' "$1" "$3" "$2"
    fails=$((fails + 1))
  fi
}

code() { curl -s --max-time 15 -o /dev/null -w '%{http_code}' "$1"; }
ctype() { curl -s --max-time 15 -o /dev/null -w '%{content_type}' "$1" | cut -d';' -f1; }

printf '== %s ==\n' "$base"

# 1. The front end loads, and it is HTML.
check "GET /"                        200 "$(code "$base/")"
check "GET / content-type"           "text/html" "$(ctype "$base/")"

# 2. A client route unknown to the server falls back to the index (SPA routing).
check "GET /a-client-route"          200 "$(code "$base/a-client-route-nobody-made")"

# 3. A hashed asset the page references is served.
asset=$(curl -s --max-time 15 "$base/" | grep -o '/_app/immutable/[A-Za-z0-9/_.-]*\.js' | head -1 || true)
if [ -n "$asset" ]; then
  check "GET $asset"                 200 "$(code "$base$asset")"
else
  printf '  WARN  no _app/immutable asset found in the page\n'
fi

# 4. A missing bundle must 404, never receive the index, or the browser reports a MIME error
#    and hides the real cause.
check "GET missing bundle"           404 "$(code "$base/_app/immutable/absent.OLDHASH.js")"

# 5. The call the front end makes at startup.
check "GET $boot"                    200 "$(code "$base$boot")"
check "GET $boot content-type"       "application/json" "$(ctype "$base$boot")"

# 6. The prefix must not be doubled.
check "GET /api$boot (must 404)"     404 "$(code "$base/api$boot")"

# 7. Health.
check "GET /api/health"              200 "$(code "$base/api/health")"
check "GET /api/ready"               200 "$(code "$base/api/ready")"

# 8. A write to an unknown route is refused, never absorbed as a 2xx: a machine client would
#    take a 200 for a delivery.
check "POST unknown route"           405 "$(curl -s --max-time 15 -o /dev/null -w '%{http_code}' -X POST "$base/no-such-route")"

# 9. No /api/api in what the page loads.
doubled=0
for f in $(curl -s --max-time 15 "$base/" | grep -o '/_app/immutable/[A-Za-z0-9/_.-]*\.js' | sort -u); do
  n=$(curl -s --max-time 10 "$base$f" | grep -c '/api/api' || true)
  doubled=$((doubled + n))
done
check "occurrences of /api/api"      0 "$doubled"

if [ "$fails" -eq 0 ]; then
  printf '  => %s OK\n' "$app"
else
  printf '  => %s: %d FAILURE(S)\n' "$app" "$fails"
fi
exit "$fails"
