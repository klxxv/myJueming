#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <tag> <assets-directory>" >&2
  exit 64
fi

tag="$1"
assets_directory="$2"
binary="$PWD/target/universal-apple-darwin/release/jueming-mcp"

test -x "$binary"
architectures="$(lipo -archs "$binary")"
if [[ " $architectures " != *" arm64 "* || " $architectures " != *" x86_64 "* ]]; then
  echo "Expected jueming-mcp to contain arm64 and x86_64, got: $architectures" >&2
  exit 1
fi

mkdir -p "$assets_directory"
ditto -c -k --sequesterRsrc --keepParent "$binary" "$assets_directory/Jueming-MCP_${tag}_macos-universal.zip"
