# WinGet Windows Sandbox Qualification

The package-manager prequalification workflow deliberately stops before claiming WinGet qualification. GitHub's
hosted Windows runner performs `winget validate` and produces this kit, but the install/upgrade/uninstall lifecycle
must run in a fresh Windows Sandbox as required by the package qualification policy.

## Inputs

Download the `package-manager-prequalification-RUN_ID` artifact from the manual workflow run. Under
`winget-sandbox-kit`, `qualification-inputs.json` records the exact Beta tag, RC tag, and GitHub run ID. The kit also
contains both candidate directories and `qualify_windows_packages.ps1`. Do not substitute other manifests or edit
the recorded run ID.

## Run in Windows Sandbox

1. Start a fresh Windows Sandbox on a supported Windows host.
2. Copy the complete `winget-sandbox-kit` directory into the Sandbox.
3. Open PowerShell inside the Sandbox and change to the copied directory.
4. Read `qualification-inputs.json`, then run the command below with those exact values:

```powershell
$inputs = Get-Content .\qualification-inputs.json -Raw | ConvertFrom-Json
.\qualify_windows_packages.ps1 `
  -Channel winget `
  -FromCandidate ".\candidates\$($inputs.from_tag)" `
  -ToCandidate ".\candidates\$($inputs.to_tag)" `
  -FromTag $inputs.from_tag `
  -ToTag $inputs.to_tag `
  -Environment windows-sandbox `
  -GitHubRunId $inputs.github_run_id `
  -Output ".\winget-x86_64-pc-windows-msvc.json"
```

The script enables WinGet local manifests only for the lifecycle, validates both manifest sets, downloads only the
hash-bound CanISend public archives, installs Beta, runs `version` and `doctor`, creates a synthetic external
workspace, upgrades to RC, repeats the probes, uninstalls, proves the workspace remains, disables local manifests,
and writes one evidence record. It does not publish or contact an external package repository except to download the
CanISend archives named by the manifests.

Copy `winget-x86_64-pc-windows-msvc.json` out before closing the Sandbox. Closing the Sandbox discards its remaining
state.

## Final verification

Place the WinGet record beside the two Homebrew records and Scoop record from the same GitHub run, then execute:

```bash
cargo run -p xtask --locked -- release verify-package-evidence \
  BETA_TAG RC_TAG EVIDENCE_DIRECTORY
```

Only a successful four-record verification may be referenced by the qualification ledger. The preparation workflow
and Sandbox run never authorize Homebrew, Scoop, WinGet, or Stable publication.
