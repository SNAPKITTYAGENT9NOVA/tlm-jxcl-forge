#!/usr/bin/env bash
# Run every Alloy command in this directory and compare each result with
# its `expect` annotation (0 = UNSAT/no counterexample, 1 = SAT/instance).
# Usage: ALLOY_JAR=/path/to/org.alloytools.alloy.dist.jar ./check.sh
set -euo pipefail
cd "$(dirname "$0")"
: "${ALLOY_JAR:?set ALLOY_JAR to the Alloy 6 dist jar}"
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
fail=0
for model in lemmas decomposition trace dula; do
  mapfile -t expected < <(grep -vE '^\s*(--|\*|/\*)' "$model.als" | grep -oE 'expect [01]\b' | grep -oE '[01]$')
  mapfile -t actual < <(java -jar "$ALLOY_JAR" exec -f -c '*' -o "$out/$model" "$model.als" 2>&1 \
    | grep -E '^[0-9]{2}\. ' | awk '{print (/ UNSAT( |$)/ ? 0 : 1) (/expects=/ ? "!" : "") "\t" $3}')
  if [ "${#expected[@]}" -ne "${#actual[@]}" ] || [ "${#actual[@]}" -eq 0 ]; then
    echo "FAIL $model: ${#expected[@]} expectations, ${#actual[@]} results"; fail=1; continue
  fi
  for i in "${!actual[@]}"; do
    got=${actual[$i]%%$'\t'*}; name=${actual[$i]#*$'\t'}
    if [ "$got" = "${expected[$i]}" ]; then echo "ok   $model.$name (expect ${expected[$i]})"
    else echo "FAIL $model.$name expected ${expected[$i]} got $got"; fail=1; fi
  done
done
exit $fail
