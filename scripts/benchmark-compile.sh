#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
benchmark_root=$(mktemp -d "${TMPDIR:-/tmp}/askama-minify-compile.XXXXXX")

cleanup() {
    rm -rf -- "$benchmark_root"
}
trap cleanup EXIT

case "$(uname -s)" in
    Darwin) time_args=(-l) ;;
    *) time_args=(-v) ;;
esac

cd "$project_root"
cargo fetch --locked

measure() {
    local label=$1
    shift

    echo
    echo "== $label =="
    env CARGO_TARGET_DIR="$benchmark_root/$label" \
        /usr/bin/time "${time_args[@]}" cargo check --locked --offline "$@"
}

measure minimal --no-default-features
measure default
measure all-features --all-features
