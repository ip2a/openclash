#!/usr/bin/env bash
set -euo pipefail

target="${1:?target triple is required}"
platform_id="${2:?platform id is required}"
artifact_binary="${3:?artifact binary name is required}"
build_tool="${OPENCLASH_BUILD_TOOL:-cargo}"

"${build_tool}" build --release --locked --target "${target}"

mkdir -p "dist/${platform_id}"
cp "target/${target}/release/${artifact_binary}" "dist/${platform_id}/${artifact_binary}"
chmod +x "dist/${platform_id}/${artifact_binary}"
