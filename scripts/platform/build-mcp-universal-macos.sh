#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script must run on macOS because it uses lipo." >&2
  exit 1
fi

cargo build --locked --package jueming-mcp --bin jueming-mcp --release --target aarch64-apple-darwin
cargo build --locked --package jueming-mcp --bin jueming-mcp --release --target x86_64-apple-darwin

arm_binary="$PWD/target/aarch64-apple-darwin/release/jueming-mcp"
intel_binary="$PWD/target/x86_64-apple-darwin/release/jueming-mcp"
universal_directory="$PWD/target/universal-apple-darwin/release"
universal_binary="$universal_directory/jueming-mcp"

test -x "$arm_binary"
test -x "$intel_binary"
mkdir -p "$universal_directory"
lipo -create "$arm_binary" "$intel_binary" -output "$universal_binary"

architectures="$(lipo -archs "$universal_binary")"
if [[ " $architectures " != *" arm64 "* || " $architectures " != *" x86_64 "* ]]; then
  echo "Expected jueming-mcp to contain arm64 and x86_64, got: $architectures" >&2
  exit 1
fi
