#!/usr/bin/env bash
# Usage: shoot.sh <out.png> [riff-story args…]
# Starts riff-story with the given arguments, screenshots its window via
# niri, then stops it. Only this script's own story process is stopped.
set -euo pipefail
out=$1; shift
cd "$(dirname "$0")/.."
./target/debug/riff-story "$@" >/tmp/riff-story-shot.log 2>&1 &
pid=$!
trap 'kill $pid 2>/dev/null || true' EXIT
for _ in $(seq 1 40); do
  id=$(niri msg --json windows | python3 -c "import json,sys; print(next((w['id'] for w in json.load(sys.stdin) if w.get('pid')==$pid), ''))")
  [[ -n $id ]] && break
  sleep 0.25
done
[[ -n $id ]] || { echo "no window for pid $pid"; cat /tmp/riff-story-shot.log; exit 1; }
sleep 2
niri msg action screenshot-window --id "$id" --path "$out" >/dev/null
sleep 0.5
echo "$out"
