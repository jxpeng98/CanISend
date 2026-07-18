param(
    [Parameter(Mandatory = $true)]
    [string]$Binary,

    [Parameter(Mandatory = $true)]
    [ValidateSet("x86_64-pc-windows-msvc")]
    [string]$Target,

    [Parameter(Mandatory = $true)]
    [string]$EvidenceJson
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not $env:CANISEND_WINDOWS_EXPECTED_SIGNER_SUBJECT) {
    throw "Windows signing: CANISEND_WINDOWS_EXPECTED_SIGNER_SUBJECT is required"
}

$binaryPath = (Resolve-Path -LiteralPath $Binary).Path
$evidencePath = [System.IO.Path]::GetFullPath($EvidenceJson)
if ((Get-Item -LiteralPath $binaryPath).LinkType) {
    throw "Windows signing: binary must not be a symlink"
}
if (Test-Path -LiteralPath $evidencePath) {
    throw "Windows signing: evidence destination must not exist: $evidencePath"
}

$signTool = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits\10\bin" `
    -Filter signtool.exe -File -Recurse |
    Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $signTool) {
    throw "Windows signing: x64 signtool.exe was not found"
}

& $signTool.FullName verify /pa /all /v $binaryPath
if ($LASTEXITCODE -ne 0) {
    throw "Windows signing: signtool verification failed with exit code $LASTEXITCODE"
}

$signature = Get-AuthenticodeSignature -LiteralPath $binaryPath
if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
    throw "Windows signing: Authenticode status is $($signature.Status), expected Valid"
}
if (-not $signature.SignerCertificate) {
    throw "Windows signing: signer certificate is missing"
}
if ($signature.SignerCertificate.Subject -ne $env:CANISEND_WINDOWS_EXPECTED_SIGNER_SUBJECT) {
    throw "Windows signing: signer subject does not match the configured identity"
}
if (-not $signature.TimeStamperCertificate) {
    throw "Windows signing: RFC3161 timestamp certificate is missing"
}

$versionOutput = & $binaryPath version --json
if ($LASTEXITCODE -ne 0) {
    throw "Windows signing: signed binary version command failed"
}
$versionEnvelope = $versionOutput | ConvertFrom-Json
$version = $versionEnvelope.data.version
if (-not $version) {
    throw "Windows signing: signed binary did not report a version"
}

$binaryInfo = Get-Item -LiteralPath $binaryPath
$binaryHash = (Get-FileHash -LiteralPath $binaryPath -Algorithm SHA256).Hash.ToLowerInvariant()
$thumbprint = $signature.SignerCertificate.Thumbprint.ToLowerInvariant()
$parent = Split-Path -Parent $evidencePath
New-Item -ItemType Directory -Force -Path $parent | Out-Null

$evidence = [ordered]@{
    schema = "canisend.code-signing-evidence/v1"
    version = $version
    target = $Target
    kind = "windows-authenticode-artifact-signing"
    status = "verified"
    binary = [ordered]@{
        file = $binaryInfo.Name
        sha256 = $binaryHash
        size = $binaryInfo.Length
    }
    archive = $null
    signer = [ordered]@{
        identity = $signature.SignerCertificate.Subject
        thumbprint = $thumbprint
    }
    verification = [ordered]@{
        authenticode_status = "Valid"
        file_digest = "SHA256"
        timestamp_digest = "SHA256"
        timestamp_present = $true
        timestamp_identity = $signature.TimeStamperCertificate.Subject
        service = "azure-artifact-signing"
    }
}

$json = $evidence | ConvertTo-Json -Depth 8
[System.IO.File]::WriteAllText(
    $evidencePath,
    $json + [Environment]::NewLine,
    [System.Text.UTF8Encoding]::new($false)
)

Write-Host "Windows signing: Authenticode signature and timestamp verified for $Target"
