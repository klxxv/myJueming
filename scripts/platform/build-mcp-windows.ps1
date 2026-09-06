[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

cargo build --locked --package jueming-mcp --bin jueming-mcp --release
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

$binary = Join-Path $PWD "target/release/jueming-mcp.exe"
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
  throw "Expected MCP server binary was not produced: $binary"
}
