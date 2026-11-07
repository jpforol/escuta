#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd -- "$project_dir"
if [[ ! -x target/release/escuta ]]; then
    echo 'Compile primeiro com: cargo build --release --locked -j 6' >&2
    exit 1
fi
exec "$project_dir/target/release/escuta" "$@"
