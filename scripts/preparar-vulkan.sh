#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
headers_dir="$project_dir/.deps/vulkan-headers"
revision='88487f687f3100ebe1d3559a8a34d971ee85b977'
if [[ -f "$headers_dir/include/vulkan/vulkan.h" ]]; then
    echo "Cabeçalhos Vulkan disponíveis em $headers_dir"
    exit 0
fi
mkdir -p "$headers_dir"
git -C "$headers_dir" init -q
git -C "$headers_dir" fetch --depth 1 https://github.com/KhronosGroup/Vulkan-Headers.git "$revision"
git -C "$headers_dir" checkout --detach FETCH_HEAD
