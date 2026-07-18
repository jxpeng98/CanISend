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

$subject = "CN=CanISend Community Build"
$binaryPath = (Resolve-Path -LiteralPath $Binary).Path
$evidencePath = [System.IO.Path]::GetFullPath($EvidenceJson)
$binaryItem = Get-Item -LiteralPath $binaryPath
if ($binaryItem.PSIsContainer) {
    throw "Windows self-signed signing: binary must be a regular file"
}
if (($binaryItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Windows self-signed signing: binary must not be a reparse point"
}
if (Test-Path -LiteralPath $evidencePath) {
    throw "Windows self-signed signing: evidence destination must not exist: $evidencePath"
}
$existing = Get-AuthenticodeSignature -LiteralPath $binaryPath
if ($existing.Status -ne [System.Management.Automation.SignatureStatus]::NotSigned) {
    throw "Windows self-signed signing: input binary already has an Authenticode signature"
}

$certificate = $null
$thumbprint = $null
try {
    $certificate = New-SelfSignedCertificate `
        -Type CodeSigningCert `
        -Subject $subject `
        -CertStoreLocation "Cert:\CurrentUser\My" `
        -HashAlgorithm SHA256 `
        -KeyAlgorithm RSA `
        -KeyLength 3072 `
        -KeyExportPolicy NonExportable `
        -NotAfter (Get-Date).AddDays(3650)
    $thumbprint = $certificate.Thumbprint

    $signingResult = Set-AuthenticodeSignature `
        -LiteralPath $binaryPath `
        -Certificate $certificate `
        -HashAlgorithm SHA256
    if (-not $signingResult.SignerCertificate) {
        throw "Windows self-signed signing: Set-AuthenticodeSignature did not produce a signer certificate"
    }

    Remove-Item -LiteralPath "Cert:\CurrentUser\My\$thumbprint" -Force
    $certificate = $null

    $signature = Get-AuthenticodeSignature -LiteralPath $binaryPath
    $allowedStatus = @(
        [System.Management.Automation.SignatureStatus]::NotTrusted,
        [System.Management.Automation.SignatureStatus]::UnknownError
    )
    if ($signature.Status -notin $allowedStatus) {
        throw "Windows self-signed signing: Authenticode status is $($signature.Status), expected an intact untrusted signature"
    }
    if (-not $signature.SignerCertificate) {
        throw "Windows self-signed signing: embedded signer certificate is missing"
    }
    if ($signature.SignerCertificate.Subject -ne $subject `
        -or $signature.SignerCertificate.Issuer -ne $subject) {
        throw "Windows self-signed signing: embedded certificate is not the expected self-signed identity"
    }
    if ($signature.SignerCertificate.Thumbprint -ne $thumbprint) {
        throw "Windows self-signed signing: embedded certificate thumbprint changed"
    }
    if ($signature.TimeStamperCertificate) {
        throw "Windows self-signed signing: an unexpected timestamp certificate is present"
    }
    $codeSigningOid = "1.3.6.1.5.5.7.3.3"
    $hasCodeSigningEku = $signature.SignerCertificate.Extensions |
        Where-Object { $_.Oid.Value -eq "2.5.29.37" } |
        ForEach-Object { $_.EnhancedKeyUsages } |
        Where-Object { $_.Value -eq $codeSigningOid }
    if (-not $hasCodeSigningEku) {
        throw "Windows self-signed signing: code-signing EKU is missing"
    }

    $versionOutput = & $binaryPath version --json
    if ($LASTEXITCODE -ne 0) {
        throw "Windows self-signed signing: signed binary version command failed"
    }
    $versionEnvelope = $versionOutput | ConvertFrom-Json
    $version = $versionEnvelope.data.version
    if (-not $version) {
        throw "Windows self-signed signing: signed binary did not report a version"
    }

    $binaryInfo = Get-Item -LiteralPath $binaryPath
    $binaryHash = (Get-FileHash -LiteralPath $binaryPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $parent = Split-Path -Parent $evidencePath
    New-Item -ItemType Directory -Force -Path $parent | Out-Null

    $evidence = [ordered]@{
        schema = "canisend.code-signing-evidence/v2"
        version = $version
        target = $Target
        kind = "windows-authenticode-self-signed"
        status = "verified"
        binary = [ordered]@{
            file = $binaryInfo.Name
            sha256 = $binaryHash
            size = $binaryInfo.Length
        }
        archive = $null
        signer = [ordered]@{
            identity = $signature.SignerCertificate.Subject
            thumbprint = $signature.SignerCertificate.Thumbprint.ToLowerInvariant()
        }
        verification = [ordered]@{
            authenticode_status = $signature.Status.ToString()
            signature_present = $true
            self_signed = $true
            certificate_trusted = $false
            file_digest = "SHA256"
            timestamp_present = $false
            service = "powershell-self-signed-authenticode"
        }
    }

    $json = $evidence | ConvertTo-Json -Depth 8
    [System.IO.File]::WriteAllText(
        $evidencePath,
        $json + [Environment]::NewLine,
        [System.Text.UTF8Encoding]::new($false)
    )
} finally {
    if ($certificate -and $certificate.Thumbprint) {
        Remove-Item -LiteralPath "Cert:\CurrentUser\My\$($certificate.Thumbprint)" -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "Windows self-signed signing: integrity signature verified for $Target (not publicly trusted or timestamped)"
