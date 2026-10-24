#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd -- "$repo_root"

run_dir="target/stress-scaling/$(date -u +%Y%m%dT%H%M%SZ)-$$"
mkdir -p -- "$run_dir"

git rev-parse HEAD > "$run_dir/revision.txt"
git status --short > "$run_dir/worktree-status.txt"
rustc --version > "$run_dir/rustc-version.txt"
uname -a > "$run_dir/system.txt"

cargo build --release --locked --features stress-benchmark --bin bfge-stress

for run in 1 2 3; do
    output="$run_dir/run-$run.json"
    printf 'Run %s/3: %s\n' "$run" "$output"
    timeout 180s target/release/bfge-stress window benchmark/configs/stress-scaling.json "$output"
done

python3 scripts/summarize-stress-scaling.py "$run_dir" "${BFGE_TARGET_FPS:-60}"
printf 'Reports: %s\n' "$run_dir"
