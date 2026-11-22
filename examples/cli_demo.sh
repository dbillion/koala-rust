#!/bin/sh
# Try the koala-rust CLI. Uses a throwaway state dir, your real ~/.koala is not touched.
# Usage: sh examples/cli_demo.sh ./koala-rust
K=${1:-koala-rust}
KOALA_STATE_DIR=$(mktemp -d)
export KOALA_STATE_DIR

echo "1) write a fact twice (second write makes version 2)"
$K fact-write --name patient-note --text "Allergic to penicillin"
$K fact-write --name patient-note --text "Allergic to penicillin and sulfa drugs"

echo; echo "2) version history"
$K -f json fact-versions --name patient-note

echo; echo "3) read the current fact"
$K fact-read --name patient-note

echo; echo "4) raw SQL: look at every stored row"
$K -f json sql "select rowid, text, is_current from facts"

echo; echo "5) health check"
$K health

echo; echo "state dir: $KOALA_STATE_DIR  (delete it when done)"
echo "Known issue: fact-delete leaves old versions on disk, see issue #11"
