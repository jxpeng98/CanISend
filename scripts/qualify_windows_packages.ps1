[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("scoop", "winget")]
    [string]$Channel,

    [Parameter(Mandatory = $true)]
    [string]$FromCandidate,

    [Parameter(Mandatory = $true)]
    [string]$ToCandidate,

    [Parameter(Mandatory = $true)]
    [string]$FromTag,

    [Parameter(Mandatory = $true)]
    [string]$ToTag,

    [Parameter(Mandatory = $true)]
    [ValidateSet("windows-2025", "windows-sandbox")]
    [string]$Environment,

    [Parameter(Mandatory = $true)]
    [ValidateRange(1, 9223372036854775807)]
    [long]$GitHubRunId,

    [Parameter(Mandatory = $true)]
    [string]$Output
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Command,

        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]]$Arguments
    )
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code ${LASTEXITCODE}: $Command $($Arguments -join ' ')"
    }
}

function Read-CandidateSource {
    param(
        [string]$Root,
        [string]$Tag,
        [string]$Stage
    )
    $path = Join-Path $Root "candidate-source.json"
    $source = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
    if (-not $source.candidate_only -or
        $source.publication_authorized -or
        $source.release.tag -ne $Tag -or
        $source.release.version -ne $Tag.Substring(1) -or
        $source.release.stage -ne $Stage) {
        throw "Candidate source does not match $Tag/$Stage or crosses the publication boundary: $path"
    }
    return $path
}

function Read-CanISendVersion {
    $value = & canisend version --json | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or -not $value.ok) {
        throw "CanISend version probe failed"
    }
    return [string]$value.data.version
}

function Assert-CanISendDoctor {
    $value = & canisend doctor --json | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or
        -not $value.ok -or
        $value.data.python_required -ne $false -or
        $value.data.embedded_typst -ne "verified" -or
        $value.data.resource_manifest -ne "verified") {
        throw "CanISend doctor did not prove the standalone runtime contract"
    }
}

$target = "x86_64-pc-windows-msvc"
$fromVersion = $FromTag.Substring(1)
$toVersion = $ToTag.Substring(1)
$fromSource = Read-CandidateSource -Root $FromCandidate -Tag $FromTag -Stage "beta"
$toSource = Read-CandidateSource -Root $ToCandidate -Tag $ToTag -Stage "rc"
$fromDigest = (Get-FileHash -LiteralPath $fromSource -Algorithm SHA256).Hash.ToLowerInvariant()
$toDigest = (Get-FileHash -LiteralPath $toSource -Algorithm SHA256).Hash.ToLowerInvariant()
$workspace = Join-Path ([System.IO.Path]::GetTempPath()) "canisend-package-workspace-$([guid]::NewGuid())"
$installed = $false
$bucketName = "canisend-qualification"
$bucketRoot = Join-Path ([System.IO.Path]::GetTempPath()) "canisend-scoop-bucket-$([guid]::NewGuid())"
$wingetLocalEnabled = $false

try {
    if ($Channel -eq "scoop") {
        if ($Environment -ne "windows-2025") {
            throw "Scoop qualification requires the windows-2025 environment"
        }
        $record = "scoop-x86_64-pc-windows-msvc"
        $fromManifest = Join-Path $FromCandidate "scoop/bucket/canisend.json"
        $toManifest = Join-Path $ToCandidate "scoop/bucket/canisend.json"
        if (Get-Command canisend -ErrorAction SilentlyContinue) {
            throw "CanISend is already installed on this Windows runner"
        }
        New-Item -ItemType Directory -Force -Path (Join-Path $bucketRoot "bucket") | Out-Null
        Copy-Item -LiteralPath $fromManifest -Destination (Join-Path $bucketRoot "bucket/canisend.json")
        Invoke-Checked git -C $bucketRoot init --initial-branch=main
        Invoke-Checked git -C $bucketRoot config user.name "CanISend qualification"
        Invoke-Checked git -C $bucketRoot config user.email "qualification@canisend.invalid"
        Invoke-Checked git -C $bucketRoot add bucket/canisend.json
        Invoke-Checked git -C $bucketRoot commit -m "add beta candidate"
        Invoke-Checked scoop bucket add $bucketName $bucketRoot
        Invoke-Checked scoop install "$bucketName/canisend"
        $installed = $true
        $fromObserved = Read-CanISendVersion
        if ($fromObserved -ne $fromVersion) { throw "Scoop installed $fromObserved instead of $fromVersion" }
        Assert-CanISendDoctor
        Invoke-Checked canisend --workspace $workspace workspace init --json
        Invoke-Checked canisend --workspace $workspace workspace check --json

        Copy-Item -LiteralPath $toManifest -Destination (Join-Path $bucketRoot "bucket/canisend.json") -Force
        Invoke-Checked git -C $bucketRoot add bucket/canisend.json
        Invoke-Checked git -C $bucketRoot commit -m "upgrade to rc candidate"
        Invoke-Checked scoop update
        Invoke-Checked scoop update canisend
        $toObserved = Read-CanISendVersion
        if ($toObserved -ne $toVersion) { throw "Scoop upgraded to $toObserved instead of $toVersion" }
        Assert-CanISendDoctor
        Invoke-Checked scoop uninstall canisend
        $installed = $false
        Invoke-Checked scoop bucket rm $bucketName
        $toolVersion = (& scoop --version | Select-Object -First 1).Trim()
    }
    else {
        if ($Environment -ne "windows-sandbox") {
            throw "WinGet qualification must run inside the prepared Windows Sandbox"
        }
        $record = "winget-x86_64-pc-windows-msvc"
        $fromManifest = Join-Path $FromCandidate "winget/manifests/p/PengJiaxin/CanISend/$fromVersion"
        $toManifest = Join-Path $ToCandidate "winget/manifests/p/PengJiaxin/CanISend/$toVersion"
        Invoke-Checked winget validate --manifest $fromManifest --disable-interactivity
        Invoke-Checked winget validate --manifest $toManifest --disable-interactivity
        Invoke-Checked winget settings --enable LocalManifestFiles
        $wingetLocalEnabled = $true
        Invoke-Checked winget install --manifest $fromManifest --accept-package-agreements --accept-source-agreements --disable-interactivity
        $installed = $true
        $env:PATH = "$env:LOCALAPPDATA\Microsoft\WinGet\Links;$env:PATH"
        $fromObserved = Read-CanISendVersion
        if ($fromObserved -ne $fromVersion) { throw "WinGet installed $fromObserved instead of $fromVersion" }
        Assert-CanISendDoctor
        Invoke-Checked canisend --workspace $workspace workspace init --json
        Invoke-Checked canisend --workspace $workspace workspace check --json

        Invoke-Checked winget upgrade --manifest $toManifest --accept-package-agreements --accept-source-agreements --disable-interactivity
        $toObserved = Read-CanISendVersion
        if ($toObserved -ne $toVersion) { throw "WinGet upgraded to $toObserved instead of $toVersion" }
        Assert-CanISendDoctor
        Invoke-Checked winget uninstall --id PengJiaxin.CanISend --exact --disable-interactivity
        $installed = $false
        Invoke-Checked winget settings --disable LocalManifestFiles
        $wingetLocalEnabled = $false
        $toolVersion = (& winget --version).Trim()
    }

    if (-not (Test-Path -LiteralPath (Join-Path $workspace "canisend.toml")) -or
        -not (Test-Path -LiteralPath (Join-Path $workspace ".canisend") -PathType Container)) {
        throw "Package uninstall removed or damaged the external CanISend workspace"
    }

    $evidence = [ordered]@{
        schema = "canisend.package-manager-qualification/v1"
        record = $record
        channel = $Channel
        target = $target
        environment = $Environment
        from_tag = $FromTag
        to_tag = $ToTag
        from_candidate_source_sha256 = $fromDigest
        to_candidate_source_sha256 = $toDigest
        github_run_id = $GitHubRunId
        tool_version = $toolVersion
        observed_versions = [ordered]@{ from = $fromObserved; to = $toObserved }
        checks = [ordered]@{
            "candidate-sources-verified" = $true
            "official-validation" = $true
            install = $true
            "from-version" = $true
            "from-doctor" = $true
            "workspace-created" = $true
            upgrade = $true
            "to-version" = $true
            "to-doctor" = $true
            uninstall = $true
            "workspace-retained" = $true
            "no-publication" = $true
        }
        completed_at = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    }
    $parent = Split-Path -Parent $Output
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    $evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Output -Encoding utf8NoBOM
    Write-Host "$Channel package qualification: wrote $Output"
}
finally {
    if ($installed) {
        if ($Channel -eq "scoop") {
            & scoop uninstall canisend | Out-Null
        }
        else {
            & winget uninstall --id PengJiaxin.CanISend --exact --disable-interactivity | Out-Null
        }
    }
    if ($wingetLocalEnabled) {
        & winget settings --disable LocalManifestFiles | Out-Null
    }
    if ($Channel -eq "scoop") {
        & scoop bucket rm $bucketName | Out-Null
    }
    Remove-Item -LiteralPath $workspace -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $bucketRoot -Recurse -Force -ErrorAction SilentlyContinue
}
