[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$Tag,
  [Parameter(Mandatory = $true)]
  [string]$AssetsDirectory
)

$ErrorActionPreference = "Stop"
$binary = Join-Path $PWD "target/release/jueming-mcp.exe"

if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
  throw "Expected MCP server binary was not produced: $binary"
}

New-Item -ItemType Directory -Path $AssetsDirectory -Force | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $AssetsDirectory "Jueming-MCP_${Tag}_windows-x64.exe")
