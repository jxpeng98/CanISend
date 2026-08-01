param(
  [Parameter(Mandatory = $true)]
  [string]$Target,
  [Parameter(Mandatory = $true)]
  [string]$EvidenceDirectory,
  [string]$BuildRoot
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if ([string]::IsNullOrWhiteSpace($BuildRoot)) {
  $BuildRoot = Join-Path ([System.IO.Path]::GetTempPath()) (
    'canisend-profile-matrix-' + [System.Guid]::NewGuid().ToString('N')
  )
}
New-Item -ItemType Directory -Force -Path $EvidenceDirectory, $BuildRoot | Out-Null
$EvidenceDirectory = (Resolve-Path -LiteralPath $EvidenceDirectory).Path
$BuildRoot = (Resolve-Path -LiteralPath $BuildRoot).Path
$hostName = if ($Target.Contains('windows')) { 'canisend-gui.exe' } else { 'canisend-gui' }

Push-Location $repositoryRoot
try {
  cargo run -p xtask --locked -- desktop template-audit
  if ($LASTEXITCODE -ne 0) { throw 'Typst template audit failed' }
  pnpm --dir apps/canisend-desktop build
  if ($LASTEXITCODE -ne 0) { throw 'Desktop frontend build failed' }

  $candidates = @(
    @{ Name = 'release'; OptLevel = '3'; Lto = 'thin' },
    @{ Name = 'size-s-thin'; OptLevel = 's'; Lto = 'thin' },
    @{ Name = 'size-z-thin'; OptLevel = 'z'; Lto = 'thin' },
    @{ Name = 'size-z-fat'; OptLevel = 'z'; Lto = 'fat' }
  )
  foreach ($candidate in $candidates) {
    $candidateRoot = Join-Path $BuildRoot $candidate.Name
    $env:CARGO_TARGET_DIR = $candidateRoot
    $env:CARGO_PROFILE_RELEASE_OPT_LEVEL = $candidate.OptLevel
    $env:CARGO_PROFILE_RELEASE_LTO = $candidate.Lto
    cargo build --locked -p canisend-gui --release `
      --target $Target --features custom-protocol
    if ($LASTEXITCODE -ne 0) {
      throw "Desktop profile build failed for $($candidate.Name)"
    }
    Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    Remove-Item Env:CARGO_PROFILE_RELEASE_OPT_LEVEL -ErrorAction SilentlyContinue
    Remove-Item Env:CARGO_PROFILE_RELEASE_LTO -ErrorAction SilentlyContinue

    $host = Join-Path $candidateRoot "$Target/release/$hostName"
    if (-not (Test-Path -LiteralPath $host -PathType Leaf)) {
      throw "Desktop profile host is missing for $($candidate.Name): $host"
    }
    $record = Join-Path $EvidenceDirectory ($candidate.Name + '.json')
    cargo run -p xtask --locked -- desktop profile-record `
      $Target $candidate.Name $candidate.OptLevel $candidate.Lto $host $record
    if ($LASTEXITCODE -ne 0) {
      throw "Desktop profile record failed for $($candidate.Name)"
    }
  }

  cargo run -p xtask --locked -- desktop profile-summary `
    (Join-Path $EvidenceDirectory 'release.json') `
    (Join-Path $EvidenceDirectory 'size-s-thin.json') `
    (Join-Path $EvidenceDirectory 'size-z-thin.json') `
    (Join-Path $EvidenceDirectory 'size-z-fat.json') `
    (Join-Path $EvidenceDirectory 'summary.json')
  if ($LASTEXITCODE -ne 0) { throw 'Desktop profile summary failed' }
} finally {
  Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
  Remove-Item Env:CARGO_PROFILE_RELEASE_OPT_LEVEL -ErrorAction SilentlyContinue
  Remove-Item Env:CARGO_PROFILE_RELEASE_LTO -ErrorAction SilentlyContinue
  Pop-Location
}

Write-Output "desktop profile evidence: $EvidenceDirectory"
Write-Output "desktop profile build root: $BuildRoot"
