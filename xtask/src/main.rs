#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use canisend_contracts::{
    AGENT_PROTOCOL, PUBLIC_SCHEMA_VERSION, WORKSPACE_FORMAT, generate_public_schemas,
    verify_public_schemas,
};
use semver::Version;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

const RELEASE_TARGET_SCHEMA: &str = "canisend.release-targets/v1";
const RELEASE_MANIFEST_SCHEMA: &str = "canisend.release-manifest/v1";
const BETA_READINESS_SCHEMA: &str = "canisend.beta-readiness/v1";
const BETA_CONTRACT_FREEZE_SCHEMA: &str = "canisend.beta-contract-freeze/v1";
const CHANNEL_CANDIDATE_SOURCE_SCHEMA: &str = "canisend.channel-candidate-source/v1";
const SIGNING_POLICY_SCHEMA: &str = "canisend.signing-policy/v1";
const SUPPORT_POLICY_SCHEMA: &str = "canisend.support-policy/v1";
const FEEDBACK_SNAPSHOT_SCHEMA: &str = "canisend.feedback-snapshot/v1";
const RELEASE_QUALIFICATION_SCHEMA: &str = "canisend.release-qualification/v1";
const PACKAGE_MANAGER_QUALIFICATION_POLICY_SCHEMA: &str =
    "canisend.package-manager-qualification-policy/v1";
const PACKAGE_MANAGER_QUALIFICATION_SCHEMA: &str = "canisend.package-manager-qualification/v1";
const CODE_SIGNING_EVIDENCE_SCHEMA: &str = "canisend.code-signing-evidence/v1";
const FUZZ_TOOLCHAIN: &str = "nightly-2026-07-01";
const CARGO_FUZZ_VERSION: &str = "0.13.2";
const WINGET_MANIFEST_VERSION: &str = "1.12.0";
const NATIVE_ALPHA_TAG: &str = "v0.7.0-alpha.1";
const NATIVE_ALPHA_SOURCE: &str = "4cec4ec48cc2e96f3798dde0b438d3aaa617a2f8";
const FROZEN_MIGRATIONS_THROUGH: u32 = 13;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<String>) -> Result<(), String> {
    match arguments.as_slice() {
        [area, command] if area == "schemas" && command == "check" => check_schemas(),
        [area, command] if area == "schemas" && command == "write" => write_schemas(),
        [area, command] if area == "resources" && command == "check" => check_resources(),
        [area, command] if area == "docs" && command == "check" => check_documentation(),
        [area, command] if area == "release" && command == "check" => {
            check_schemas()?;
            check_resources()?;
            check_documentation()?;
            check_property_test_policy()?;
            check_fuzz_policy()?;
            check_internal_dependency_versions()?;
            check_beta_readiness()?;
            check_beta_contract_freeze()?;
            check_channel_candidates()?;
            check_package_manager_qualification_policy()?;
            check_signing_policy()?;
            check_support_policy()?;
            check_release_feedback()?;
            check_release_qualification()?;
            check_release_contract()
        }
        [area, command] if area == "release" && command == "freeze-candidate" => {
            let candidate = build_beta_contract_freeze()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&candidate)
                    .map_err(|error| format!("could not serialize freeze candidate: {error}"))?
            );
            Ok(())
        }
        [area, command, tag] if area == "release" && command == "validate-tag" => {
            validate_release_tag(tag).map(|_| ())
        }
        [area, command, output] if area == "release" && command == "sbom" => {
            write_release_sbom(Path::new(output))
        }
        [area, command, tag, commit, artifacts, output]
            if area == "release" && command == "assemble" =>
        {
            assemble_release(tag, commit, Path::new(artifacts), Path::new(output))
        }
        [area, command, tag, directory] if area == "release" && command == "verify" => {
            verify_release(tag, Path::new(directory))
        }
        [area, command, tag, assets, output]
            if area == "release" && command == "channels" =>
        {
            write_channel_candidates(tag, Path::new(assets), Path::new(output))
        }
        [area, command, tag, target, evidence, binary, archive]
            if area == "release" && command == "bind-signing-evidence" =>
        {
            bind_signing_evidence(
                tag,
                target,
                Path::new(evidence),
                Path::new(binary),
                Path::new(archive),
            )
        }
        [area, command, from_tag, to_tag, evidence]
            if area == "release" && command == "verify-package-evidence" =>
        {
            verify_package_manager_evidence(from_tag, to_tag, Path::new(evidence))
        }
        _ => Err(
            "usage: cargo run -p xtask -- schemas <check|write> | <resources|docs> check | \
             release <check|freeze-candidate|validate-tag TAG|sbom OUTPUT|assemble TAG COMMIT ARTIFACTS OUTPUT|verify TAG DIRECTORY|channels TAG ASSETS OUTPUT|bind-signing-evidence TAG TARGET EVIDENCE BINARY ARCHIVE|verify-package-evidence FROM_TAG TO_TAG DIRECTORY>"
                .to_owned(),
        ),
    }
}

fn check_schemas() -> Result<(), String> {
    verify_public_schemas()?;
    let expected = generate_public_schemas();
    let directory = schema_directory();
    let mut expected_names = BTreeSet::new();
    for schema in expected {
        let file_name = schema.id.file_name();
        expected_names.insert(file_name.clone());
        let path = directory.join(file_name);
        let actual = fs::read_to_string(&path).map_err(|error| {
            format!("generated schema is missing at {}: {error}", path.display())
        })?;
        if actual != schema.canonical_json() {
            return Err(format!(
                "generated schema drift at {}; run `cargo run -p xtask -- schemas write`",
                path.display()
            ));
        }
    }
    let actual_names = json_files(&directory)?;
    if actual_names != expected_names {
        return Err(format!(
            "generated schema file set differs: expected {expected_names:?}, found {actual_names:?}"
        ));
    }
    println!("schemas: ok ({})", expected_names.len());
    Ok(())
}

fn write_schemas() -> Result<(), String> {
    verify_public_schemas()?;
    let directory = schema_directory();
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let schemas = generate_public_schemas();
    let expected_names = schemas
        .iter()
        .map(|schema| schema.id.file_name())
        .collect::<BTreeSet<_>>();
    for existing in json_files(&directory)? {
        if !expected_names.contains(&existing) {
            fs::remove_file(directory.join(existing)).map_err(|error| error.to_string())?;
        }
    }
    for schema in schemas {
        let path = directory.join(schema.id.file_name());
        fs::write(&path, schema.canonical_json()).map_err(|error| error.to_string())?;
    }
    println!("schemas: wrote {}", expected_names.len());
    Ok(())
}

fn schema_directory() -> PathBuf {
    repository_root().join("crates/canisend-resources/resources/schemas/v2")
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is inside repository root")
        .to_path_buf()
}

fn json_files(directory: &Path) -> Result<BTreeSet<String>, String> {
    if !directory.exists() {
        return Ok(BTreeSet::new());
    }
    fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(|entry| match entry {
            Ok(entry)
                if entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "json") =>
            {
                Some(Ok(entry.file_name().to_string_lossy().into_owned()))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error.to_string())),
        })
        .collect()
}

fn check_resources() -> Result<(), String> {
    canisend_resources::verify()?;
    if canisend_resources::manifest().is_empty() {
        return Err("embedded resource manifest is empty".to_owned());
    }
    println!("resources: ok");
    Ok(())
}

fn check_documentation() -> Result<(), String> {
    let root = repository_root();
    let guide_root = root.join("docs/guides");
    let required = [
        "installation.md",
        "release-verification.md",
        "quick-start.md",
        "agent-integration.md",
        "privacy-and-consent.md",
        "backup-and-recovery.md",
        "upgrade-and-rollback.md",
        "troubleshooting.md",
    ];
    for file_name in required {
        let path = guide_root.join(file_name);
        let body = fs::read_to_string(&path)
            .map_err(|error| format!("required guide is missing at {}: {error}", path.display()))?;
        check_local_markdown_links(&root, &path, &body)?;
    }
    for path in [
        root.join("README.md"),
        root.join("AGENTS.md"),
        guide_root.join("README.md"),
        root.join("docs/development/defensive-assurance-routing.md"),
    ] {
        let body = fs::read_to_string(&path).map_err(|error| {
            format!(
                "documentation index is missing at {}: {error}",
                path.display()
            )
        })?;
        check_local_markdown_links(&root, &path, &body)?;
    }
    let repository_scope = fs::read_to_string(root.join("AGENTS.md"))
        .map_err(|error| format!("repository agent scope is missing: {error}"))?;
    for required in [
        "defensive software assurance",
        "Do not turn these tasks into instructions for accessing third-party systems",
        "do not ask the host to disable, downgrade, or bypass its safety policy",
        "Verification tiers",
    ] {
        if !repository_scope.contains(required) {
            return Err(format!("repository agent scope is missing `{required}`"));
        }
    }
    let smoke = root.join("scripts/smoke_documented_quickstart.sh");
    if !smoke.is_file() {
        return Err(format!(
            "documented quick-start smoke is missing at {}",
            smoke.display()
        ));
    }
    println!("documentation: ok ({} guides)", required.len());
    Ok(())
}

fn check_fuzz_policy() -> Result<(), String> {
    let root = repository_root();
    let workflow_path = root.join(".github/workflows/fuzz.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("scheduled fuzz workflow is missing: {error}"))?;
    let manifest_path = root.join("fuzz/Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("fuzz manifest is missing: {error}"))?;
    let targets = ["structured_inputs", "intake_parsers", "pdf_extract"];
    for required in [
        "schedule:",
        "workflow_dispatch:",
        FUZZ_TOOLCHAIN,
        CARGO_FUZZ_VERSION,
        "-max_total_time=300",
        "-timeout=15",
        "upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
    ] {
        if !workflow.contains(required) {
            return Err(format!("scheduled fuzz workflow is missing `{required}`"));
        }
    }
    for required in [
        "cargo-fuzz = true",
        "libfuzzer-sys = \"=0.4.13\"",
        "canisend-contracts",
        "canisend-io",
    ] {
        if !manifest.contains(required) {
            return Err(format!("fuzz manifest is missing `{required}`"));
        }
    }
    for target in targets {
        if !workflow.contains(target)
            || !manifest.contains(&format!("name = \"{target}\""))
            || !root
                .join(format!("fuzz/fuzz_targets/{target}.rs"))
                .is_file()
        {
            return Err(format!("scheduled fuzz target `{target}` is incomplete"));
        }
    }
    let documentation_path = root.join("docs/testing/scheduled-fuzzing.md");
    let documentation = fs::read_to_string(&documentation_path)
        .map_err(|error| format!("scheduled fuzz documentation is missing: {error}"))?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    println!("fuzz policy: ok ({} scheduled targets)", targets.len());
    Ok(())
}

fn check_property_test_policy() -> Result<(), String> {
    let root = repository_root();
    let test_path = root.join("crates/canisend-contracts/tests/property_contract.rs");
    let test = fs::read_to_string(&test_path)
        .map_err(|error| format!("property-test target is missing: {error}"))?;
    for required in [
        "property_generated_portable_paths_round_trip_without_normalization",
        "property_inserting_any_reserved_component_is_always_rejected",
        "property_generated_sha256_digests_round_trip_and_mutations_fail",
        "property_generated_uuidv7_and_revisions_preserve_identity",
        "GENERATED_CASES: usize = 512",
    ] {
        if !test.contains(required) {
            return Err(format!("property-test target is missing `{required}`"));
        }
    }

    let command = "cargo test -p canisend-contracts --locked --test property_contract";
    for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        let path = root.join(workflow);
        let body = fs::read_to_string(&path)
            .map_err(|error| format!("property-test workflow `{workflow}` is missing: {error}"))?;
        if !body.contains(command) {
            return Err(format!(
                "property-test workflow `{workflow}` is missing `{command}`"
            ));
        }
    }

    let documentation_path = root.join("docs/testing/property-testing.md");
    let documentation = fs::read_to_string(&documentation_path)
        .map_err(|error| format!("property-test documentation is missing: {error}"))?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    println!("property-test policy: ok (4 generated properties)");
    Ok(())
}

fn check_local_markdown_links(root: &Path, source: &Path, body: &str) -> Result<(), String> {
    let parent = source
        .parent()
        .ok_or_else(|| format!("documentation path has no parent: {}", source.display()))?;
    let mut remaining = body;
    while let Some(start) = remaining.find("](") {
        let target_start = start + 2;
        remaining = &remaining[target_start..];
        let Some(end) = remaining.find(')') else {
            return Err(format!(
                "unterminated Markdown link in {}",
                source.display()
            ));
        };
        let destination = remaining[..end].trim();
        remaining = &remaining[end + 1..];
        if destination.is_empty()
            || destination.starts_with('#')
            || destination.starts_with("http://")
            || destination.starts_with("https://")
            || destination.starts_with("mailto:")
        {
            continue;
        }
        let relative = destination
            .split('#')
            .next()
            .expect("split always returns one element");
        let candidate = parent.join(relative);
        if !candidate.exists() {
            return Err(format!(
                "broken local link `{destination}` in {}",
                source.strip_prefix(root).unwrap_or(source).display()
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReleaseTarget {
    triple: String,
    runner: String,
    executable: String,
    archive: String,
    signing: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ChannelArtifact {
    target: String,
    archive: String,
    sha256: String,
    size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ChannelCandidateSource {
    tag: String,
    version: String,
    stage: ReleaseStage,
    source_commit: String,
    repository: String,
    manifest_file: String,
    manifest_sha256: String,
    artifacts: BTreeMap<String, ChannelArtifact>,
}

impl ChannelCandidateSource {
    fn artifact(&self, target: &str) -> Result<&ChannelArtifact, String> {
        self.artifacts
            .get(target)
            .ok_or_else(|| format!("channel candidate source has no `{target}` artifact"))
    }

    fn to_value(&self) -> Value {
        let artifacts = self
            .artifacts
            .values()
            .map(|artifact| {
                json!({
                    "target": artifact.target,
                    "archive": artifact.archive,
                    "sha256": artifact.sha256,
                    "size": artifact.size,
                })
            })
            .collect::<Vec<_>>();
        json!({
            "schema": CHANNEL_CANDIDATE_SOURCE_SCHEMA,
            "candidate_only": true,
            "publication_authorized": false,
            "release": {
                "tag": self.tag,
                "version": self.version,
                "stage": self.stage.as_str(),
                "source_commit": self.source_commit,
                "repository": self.repository,
                "manifest_file": self.manifest_file,
                "manifest_sha256": self.manifest_sha256,
            },
            "artifacts": artifacts,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReleaseStage {
    Alpha,
    Beta,
    ReleaseCandidate,
    Stable,
}

impl ReleaseStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Alpha => "alpha",
            Self::Beta => "beta",
            Self::ReleaseCandidate => "rc",
            Self::Stable => "stable",
        }
    }

    fn from_version(version: &Version) -> Result<Self, String> {
        let prerelease = version.pre.as_str();
        if prerelease.starts_with("alpha.") {
            Ok(Self::Alpha)
        } else if prerelease.starts_with("beta.") {
            Ok(Self::Beta)
        } else if prerelease.starts_with("rc.") {
            Ok(Self::ReleaseCandidate)
        } else if prerelease.is_empty() {
            Ok(Self::Stable)
        } else {
            Err(format!(
                "release tag prerelease `{prerelease}` is not alpha, beta, or rc"
            ))
        }
    }
}

fn check_internal_dependency_versions() -> Result<(), String> {
    let root = repository_root();
    let workspace_path = root.join("Cargo.toml");
    let workspace_body = fs::read_to_string(&workspace_path)
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    let members = workspace["workspace"]["members"]
        .as_array()
        .ok_or_else(|| "workspace manifest has no members array".to_owned())?;
    let expected = format!("={}", env!("CARGO_PKG_VERSION"));
    let mut manifests = Vec::with_capacity(members.len());
    let mut internal_packages = BTreeSet::new();

    for member in members {
        let member = member
            .as_str()
            .ok_or_else(|| "workspace member must be a string".to_owned())?;
        let path = root.join(member).join("Cargo.toml");
        let body = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let manifest: toml::Value = body
            .parse()
            .map_err(|error| format!("{} is invalid TOML: {error}", path.display()))?;
        let package = manifest["package"]["name"]
            .as_str()
            .ok_or_else(|| format!("{} has no package name", path.display()))?
            .to_owned();
        internal_packages.insert(package);
        manifests.push((member.to_owned(), manifest));
    }

    for (member, manifest) in &manifests {
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            check_dependency_table(
                member,
                section,
                manifest.get(section),
                &internal_packages,
                &expected,
            )?;
        }
        if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
            for (target, target_manifest) in targets {
                for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                    check_dependency_table(
                        member,
                        &format!("target.{target}.{section}"),
                        target_manifest.get(section),
                        &internal_packages,
                        &expected,
                    )?;
                }
            }
        }
    }

    println!(
        "internal dependency versions: ok ({} packages, {expected})",
        internal_packages.len()
    );
    Ok(())
}

fn check_dependency_table(
    member: &str,
    section: &str,
    dependencies: Option<&toml::Value>,
    internal_packages: &BTreeSet<String>,
    expected: &str,
) -> Result<(), String> {
    let Some(dependencies) = dependencies.and_then(toml::Value::as_table) else {
        return Ok(());
    };
    for (alias, dependency) in dependencies {
        let Some(detail) = dependency.as_table() else {
            continue;
        };
        if !detail.contains_key("path") {
            continue;
        }
        let package = detail
            .get("package")
            .and_then(toml::Value::as_str)
            .unwrap_or(alias);
        if !internal_packages.contains(package) {
            continue;
        }
        let actual = detail.get("version").and_then(toml::Value::as_str);
        if actual != Some(expected) {
            return Err(format!(
                "internal dependency `{alias}` in {member}/Cargo.toml [{section}] must use exact version `{expected}`, found {}",
                actual.unwrap_or("<missing>")
            ));
        }
    }
    Ok(())
}

fn check_release_contract() -> Result<(), String> {
    let root = repository_root();
    let targets = release_targets()?;
    let expected = BTreeSet::from([
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "x86_64-pc-windows-msvc",
    ]);
    let actual = targets
        .iter()
        .map(|target| target.triple.as_str())
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!(
            "release target set differs: expected {expected:?}, found {actual:?}"
        ));
    }
    for required in [
        "release/KNOWN_LIMITATIONS.md",
        "release/ISSUE_COLLECTION.md",
        "release/RELEASE_NOTES.md",
        "release/beta-readiness.json",
        "release/beta-contract-freeze.json",
        "scripts/stage_native_bundle.sh",
        "scripts/package_native_release.sh",
        "scripts/smoke_release_archive.sh",
    ] {
        let path = root.join(required);
        if !path.is_file() {
            return Err(format!("release contract file is missing: {required}"));
        }
    }
    let workflow_path = root.join(".github/workflows/release.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("release workflow is missing: {error}"))?;
    for target in &targets {
        for required in [
            target.triple.as_str(),
            target.runner.as_str(),
            target.executable.as_str(),
            target.archive.as_str(),
        ] {
            if !workflow.contains(required) {
                return Err(format!(
                    "release workflow does not reference `{required}` for {}",
                    target.triple
                ));
            }
        }
    }
    for required in [
        "release validate-tag",
        "release assemble",
        "release verify",
        "attest-build-provenance",
        "SHA256SUMS",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "release workflow is missing required gate `{required}`"
            ));
        }
    }
    println!("release contract: ok ({} targets)", targets.len());
    Ok(())
}

fn check_beta_readiness() -> Result<(), String> {
    let path = repository_root().join("release/beta-readiness.json");
    let body = fs::read_to_string(&path).map_err(|error| {
        format!(
            "Beta readiness ledger is missing at {}: {error}",
            path.display()
        )
    })?;
    let ledger: Value = serde_json::from_str(&body)
        .map_err(|error| format!("Beta readiness ledger is invalid JSON: {error}"))?;
    if ledger["schema"] != BETA_READINESS_SCHEMA
        || ledger["alpha_release"]["tag"] != NATIVE_ALPHA_TAG
        || ledger["alpha_release"]["source_commit"] != NATIVE_ALPHA_SOURCE
    {
        return Err(
            "Beta readiness ledger does not identify the qualified native Alpha".to_owned(),
        );
    }
    let audited_at = ledger["audited_at"]
        .as_str()
        .filter(|value| value.ends_with('Z') && value.contains('T'))
        .ok_or_else(|| "Beta readiness ledger has no UTC audit timestamp".to_owned())?;
    if ledger["default_telemetry"] != false {
        return Err("Beta readiness ledger must preserve disabled default telemetry".to_owned());
    }
    if ledger["github_issue_snapshot"]["open_issue_count"] != 0
        || ledger["unresolved_release_blockers"]
            .as_array()
            .is_none_or(|entries| !entries.is_empty())
    {
        return Err("Beta readiness ledger contains unresolved Alpha blockers".to_owned());
    }
    let expected_classes = BTreeSet::from([
        "data-loss",
        "protocol-compatibility",
        "rendering-corruption",
        "security-privacy",
    ]);
    let entries = ledger["blocker_classes"]
        .as_array()
        .ok_or_else(|| "Beta readiness blocker_classes must be an array".to_owned())?;
    let mut actual_classes = BTreeSet::new();
    for entry in entries {
        let class = entry["class"]
            .as_str()
            .ok_or_else(|| "Beta readiness blocker class is missing".to_owned())?;
        if !actual_classes.insert(class) {
            return Err(format!("duplicate Beta readiness blocker class `{class}`"));
        }
        if !matches!(entry["status"].as_str(), Some("clear" | "resolved")) {
            return Err(format!(
                "Beta readiness blocker class `{class}` is not clear"
            ));
        }
        if entry["open_issue_numbers"]
            .as_array()
            .is_none_or(|issues| !issues.is_empty())
        {
            return Err(format!(
                "Beta readiness blocker class `{class}` contains open issues"
            ));
        }
        if entry["evidence"].as_array().is_none_or(|evidence| {
            evidence.is_empty() || evidence.iter().any(|item| !item.is_string())
        }) {
            return Err(format!(
                "Beta readiness blocker class `{class}` has no evidence"
            ));
        }
    }
    if actual_classes != expected_classes {
        return Err(format!(
            "Beta readiness blocker classes differ: expected {expected_classes:?}, found {actual_classes:?}"
        ));
    }
    println!(
        "beta readiness: ok ({} blocker classes, audited {audited_at})",
        actual_classes.len()
    );
    Ok(())
}

fn check_beta_contract_freeze() -> Result<(), String> {
    let path = repository_root().join("release/beta-contract-freeze.json");
    let body = fs::read_to_string(&path).map_err(|error| {
        format!(
            "Beta contract freeze is missing at {}: {error}",
            path.display()
        )
    })?;
    let actual: Value = serde_json::from_str(&body)
        .map_err(|error| format!("Beta contract freeze is invalid JSON: {error}"))?;
    let expected = build_beta_contract_freeze()?;
    if actual != expected {
        return Err(
            "Beta agent/workspace contract freeze drifted; review the compatibility impact and regenerate with \
             `cargo run -p xtask -- release freeze-candidate`"
                .to_owned(),
        );
    }
    println!(
        "beta contract freeze: ok ({} schemas, migrations frozen through {})",
        expected["agent"]["public_schema_files"], FROZEN_MIGRATIONS_THROUGH
    );
    Ok(())
}

fn check_signing_policy() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/signing-policy.json");
    let actual: Value =
        serde_json::from_slice(&fs::read(&path).map_err(|error| {
            format!("signing policy is missing at {}: {error}", path.display())
        })?)
        .map_err(|error| format!("signing policy is invalid JSON: {error}"))?;
    let expected = json!({
        "schema": SIGNING_POLICY_SCHEMA,
        "stage_boundary": {
            "alpha_may_be_unsigned": true,
            "beta_rc_stable_require_all_configured_signers": true,
            "missing_credentials": "fail-closed"
        },
        "macos": {
            "targets": ["aarch64-apple-darwin", "x86_64-apple-darwin"],
            "service": "apple-developer-id-notarytool",
            "certificate_type": "Developer ID Application",
            "code_identifier": "io.github.jxpeng98.canisend",
            "hardened_runtime": true,
            "secure_timestamp": true,
            "notarization_submission": "zip",
            "standalone_ticket_stapling_supported": false,
            "required_secrets": [
                "APPLE_DEVELOPER_ID_P12_BASE64",
                "APPLE_DEVELOPER_ID_P12_PASSWORD",
                "APPLE_NOTARY_KEY_P8_BASE64"
            ],
            "required_variables": [
                "APPLE_SIGNING_IDENTITY",
                "APPLE_TEAM_ID",
                "APPLE_NOTARY_KEY_ID",
                "APPLE_NOTARY_ISSUER_ID"
            ]
        },
        "windows": {
            "targets": ["x86_64-pc-windows-msvc"],
            "service": "azure-artifact-signing",
            "trust_model": "public-trust",
            "authentication": "github-oidc",
            "file_digest": "SHA256",
            "timestamp_digest": "SHA256",
            "timestamp_url": "http://timestamp.acs.microsoft.com",
            "required_secrets": [],
            "required_variables": [
                "AZURE_CLIENT_ID",
                "AZURE_TENANT_ID",
                "AZURE_SUBSCRIPTION_ID",
                "AZURE_ARTIFACT_SIGNING_ENDPOINT",
                "AZURE_ARTIFACT_SIGNING_ACCOUNT",
                "AZURE_ARTIFACT_SIGNING_PROFILE",
                "WINDOWS_SIGNING_EXPECTED_SUBJECT"
            ]
        },
        "linux": {
            "targets": ["x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"],
            "code_signing": "none",
            "integrity": ["sha256sums", "github-oidc-provenance"]
        }
    });
    if actual != expected {
        return Err("release signing policy drifted from the fail-closed Beta contract".to_owned());
    }
    let readiness_path = root.join("scripts/check_signing_readiness.sh");
    let audit_path = root.join("scripts/audit_github_signing_configuration.sh");
    let macos_path = root.join("scripts/sign_and_notarize_macos.sh");
    let windows_path = root.join("scripts/verify_windows_authenticode.ps1");
    let operations_path = root.join("docs/release/signing-operations.md");
    let readiness = fs::read_to_string(&readiness_path)
        .map_err(|error| format!("release signing readiness script is missing: {error}"))?;
    let audit = fs::read_to_string(&audit_path)
        .map_err(|error| format!("GitHub signing configuration audit is missing: {error}"))?;
    let macos = fs::read_to_string(&macos_path)
        .map_err(|error| format!("macOS signing script is missing: {error}"))?;
    let windows = fs::read_to_string(&windows_path)
        .map_err(|error| format!("Windows signing verifier is missing: {error}"))?;
    let operations = fs::read_to_string(&operations_path)
        .map_err(|error| format!("release signing operations guide is missing: {error}"))?;
    for required in ["release/signing-policy.json", "alpha|beta|rc|stable"] {
        if !readiness.contains(required) {
            return Err(format!(
                "release signing readiness script is missing `{required}`"
            ));
        }
    }
    let configuration_names = [
        "APPLE_DEVELOPER_ID_P12_BASE64",
        "APPLE_DEVELOPER_ID_P12_PASSWORD",
        "APPLE_NOTARY_KEY_P8_BASE64",
        "APPLE_SIGNING_IDENTITY",
        "APPLE_TEAM_ID",
        "APPLE_NOTARY_KEY_ID",
        "APPLE_NOTARY_ISSUER_ID",
        "AZURE_CLIENT_ID",
        "AZURE_TENANT_ID",
        "AZURE_SUBSCRIPTION_ID",
        "AZURE_ARTIFACT_SIGNING_ENDPOINT",
        "AZURE_ARTIFACT_SIGNING_ACCOUNT",
        "AZURE_ARTIFACT_SIGNING_PROFILE",
        "WINDOWS_SIGNING_EXPECTED_SUBJECT",
    ];
    for required in configuration_names {
        if !readiness.contains(required)
            || !audit.contains(required)
            || !operations.contains(required)
        {
            return Err(format!(
                "release signing configuration surfaces are missing `{required}`"
            ));
        }
    }
    for required in ["gh secret list", "gh variable list", "values were not read"] {
        if !audit.contains(required) {
            return Err(format!(
                "GitHub signing configuration audit is missing `{required}`"
            ));
        }
    }
    for required in [
        "--identifier io.github.jxpeng98.canisend",
        "--options runtime",
        "--timestamp",
        "com.apple.security.get-task-allow",
        "notarytool submit",
        "notarytool log",
        "notary_error_count",
        "canisend.code-signing-evidence/v1",
        "stapling_supported: false",
    ] {
        if !macos.contains(required) {
            return Err(format!("macOS signing script is missing `{required}`"));
        }
    }
    for required in [
        "signtool.exe",
        "verify /pa /all /v",
        "Get-AuthenticodeSignature",
        "CANISEND_WINDOWS_EXPECTED_SIGNER_SUBJECT",
        "TimeStamperCertificate",
        "canisend.code-signing-evidence/v1",
        "timestamp_present = $true",
        "service = \"azure-artifact-signing\"",
    ] {
        if !windows.contains(required) {
            return Err(format!("Windows signing verifier is missing `{required}`"));
        }
    }
    let workflow_path = root.join(".github/workflows/release.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("release workflow is missing: {error}"))?;
    for required in configuration_names {
        if !workflow.contains(required) {
            return Err(format!(
                "release workflow is missing signing configuration `{required}`"
            ));
        }
    }
    for required in [
        "release/signing-policy.json",
        "check_signing_readiness.sh",
        "sign_and_notarize_macos.sh",
        "verify_windows_authenticode.ps1",
        "bind-signing-evidence",
        "azure/login@532459ea530d8321f2fb9bb10d1e0bcf23869a43",
        "azure/artifact-signing-action@c7ab2a863ab5f9a846ddb8265964877ef296ee82",
        "id-token: write",
        "file-digest: SHA256",
        "timestamp-rfc3161: http://timestamp.acs.microsoft.com",
        "timestamp-digest: SHA256",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "release workflow is missing signing gate `{required}`"
            ));
        }
    }
    println!("signing policy: ok (Apple notarization + Windows Artifact Signing)");
    Ok(())
}

fn check_support_policy() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/support-policy.json");
    let actual: Value =
        serde_json::from_slice(&fs::read(&path).map_err(|error| {
            format!("support policy is missing at {}: {error}", path.display())
        })?)
        .map_err(|error| format!("support policy is invalid JSON: {error}"))?;
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let publication_status = support_policy_publication_status(&version);
    let target_count = release_targets()?.len();
    let expected = json!({
        "schema": SUPPORT_POLICY_SCHEMA,
        "publication_status": publication_status,
        "release_line": format!("{}.{}", version.major, version.minor),
        "version_support": {
            "prerelease": "current-only-until-superseded",
            "stable": "current-minor-latest-patch",
            "long_term_support": false,
            "service_level_agreement": false,
            "python_0_6_line": "archived-unsupported"
        },
        "contracts": {
            "agent_protocol": AGENT_PROTOCOL,
            "public_schema_version": PUBLIC_SCHEMA_VERSION,
            "resource_format": canisend_resources::RESOURCE_VERSION,
            "beta_freeze": "release/beta-contract-freeze.json",
            "breaking_agent_change": "new-protocol-and-schema-major"
        },
        "workspace": {
            "format": WORKSPACE_FORMAT,
            "current_database_schema_version": declared_database_schema_version()?,
            "frozen_migrations_through": FROZEN_MIGRATIONS_THROUGH,
            "migration_policy": "append-only",
            "future_schema": "reject-without-mutation",
            "downgrade": "restore-verified-pre-upgrade-backup-to-new-path"
        },
        "platforms": {
            "authority": "release/targets.json",
            "target_count": target_count,
            "linux_arm64": "unsupported-in-0.7",
            "runtime_requirements": {
                "python": false,
                "node": false,
                "java": false,
                "external_typst": false,
                "external_sqlite": false
            }
        },
        "host_assets": {
            "codex": "generated-by-installed-binary",
            "claude": "generated-by-installed-binary",
            "refresh_after_upgrade": true,
            "private_workspace_bodies_included_by_default": false
        },
        "security": {
            "reporting": "SECURITY.md",
            "default_telemetry": false,
            "private_issue_content": "prohibited"
        }
    });
    if actual != expected {
        return Err(
            "release/support-policy.json differs from the current product, contract, workspace, or platform policy"
                .to_owned(),
        );
    }

    let document_path = root.join("docs/release/support-policy.md");
    let document = fs::read_to_string(&document_path).map_err(|error| {
        format!(
            "support policy documentation is missing at {}: {error}",
            document_path.display()
        )
    })?;
    check_local_markdown_links(&root, &document_path, &document)?;
    for required in [
        AGENT_PROTOCOL,
        PUBLIC_SCHEMA_VERSION,
        canisend_resources::RESOURCE_VERSION,
        WORKSPACE_FORMAT,
        "current-only-until-superseded",
        "current-minor-latest-patch",
        "restore into a new path",
        "Linux arm64",
        "No service-level agreement",
    ] {
        if !document.contains(required) {
            return Err(format!(
                "support policy documentation is missing `{required}`"
            ));
        }
    }
    println!("support policy: ok ({publication_status}, {target_count} targets)");
    Ok(())
}

fn support_policy_publication_status(version: &Version) -> &'static str {
    if version.pre.is_empty() {
        "published"
    } else {
        "pre-stable-draft"
    }
}

fn check_release_feedback() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/feedback-snapshot.json");
    let snapshot: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "release feedback snapshot is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("release feedback snapshot is invalid JSON: {error}"))?;
    if snapshot["schema"] != FEEDBACK_SNAPSHOT_SCHEMA
        || snapshot["default_telemetry"] != false
        || snapshot["privacy_boundary"] != "public-metadata-only"
    {
        return Err("release feedback snapshot identity or privacy boundary is invalid".to_owned());
    }
    let captured_at = snapshot["captured_at"]
        .as_str()
        .filter(|value| value.contains('T') && value.ends_with('Z'))
        .ok_or_else(|| "release feedback snapshot has no UTC captured_at".to_owned())?;
    let snapshot_stage = snapshot["snapshot_stage"]
        .as_str()
        .ok_or_else(|| "release feedback snapshot has no stage".to_owned())?;
    if !matches!(snapshot_stage, "alpha-baseline" | "beta" | "rc") {
        return Err(format!(
            "unsupported release feedback snapshot stage `{snapshot_stage}`"
        ));
    }
    let release = &snapshot["release"];
    let repository = env!("CARGO_PKG_REPOSITORY").trim_start_matches("https://github.com/");
    let published_at = release["published_at"]
        .as_str()
        .filter(|value| value.contains('T') && value.ends_with('Z'))
        .ok_or_else(|| "release feedback snapshot has no release publication time".to_owned())?;
    let release_tag = release["tag"]
        .as_str()
        .and_then(|value| value.strip_prefix('v'))
        .ok_or_else(|| "release feedback snapshot has no valid release tag".to_owned())?;
    let release_version = Version::parse(release_tag)
        .map_err(|error| format!("release feedback tag is invalid SemVer: {error}"))?;
    let expected_prerelease_prefix = match snapshot_stage {
        "alpha-baseline" => "alpha.",
        "beta" => "beta.",
        "rc" => "rc.",
        _ => unreachable!("snapshot stage was validated"),
    };
    if release["repository"] != repository
        || !release_version
            .pre
            .as_str()
            .starts_with(expected_prerelease_prefix)
        || published_at > captured_at
    {
        return Err("release feedback snapshot does not match its public release stage".to_owned());
    }

    let feedback = &snapshot["public_feedback"];
    let open = feedback["open_issue_count"]
        .as_u64()
        .ok_or_else(|| "release feedback open issue count is invalid".to_owned())?;
    let closed = feedback["closed_issue_count"]
        .as_u64()
        .ok_or_else(|| "release feedback closed issue count is invalid".to_owned())?;
    let total = feedback["total_issue_count"]
        .as_u64()
        .ok_or_else(|| "release feedback total issue count is invalid".to_owned())?;
    let issue_numbers = feedback["issue_numbers"]
        .as_array()
        .ok_or_else(|| "release feedback issue_numbers must be an array".to_owned())?;
    let unique_issue_numbers = issue_numbers
        .iter()
        .filter_map(Value::as_u64)
        .collect::<BTreeSet<_>>();
    if open + closed != total
        || usize::try_from(total).ok() != Some(issue_numbers.len())
        || unique_issue_numbers.len() != issue_numbers.len()
        || unique_issue_numbers.contains(&0)
    {
        return Err("release feedback issue counts are inconsistent".to_owned());
    }

    let downloads = &snapshot["release_downloads"];
    let asset_count = downloads["asset_count"]
        .as_u64()
        .ok_or_else(|| "release feedback asset count is invalid".to_owned())?;
    let total_downloads = downloads["total_downloads"]
        .as_u64()
        .ok_or_else(|| "release feedback total downloads are invalid".to_owned())?;
    let native_archive_count = downloads["native_archive_count"]
        .as_u64()
        .ok_or_else(|| "release feedback native archive count is invalid".to_owned())?;
    let native_archive_downloads = downloads["native_archive_downloads"]
        .as_u64()
        .ok_or_else(|| "release feedback native archive downloads are invalid".to_owned())?;
    if native_archive_count > asset_count
        || native_archive_downloads > total_downloads
        || downloads["maintainer_verification_included"] != true
    {
        return Err("release download evidence overclaims independent adoption".to_owned());
    }

    let findings = snapshot["qualification_findings"]
        .as_array()
        .filter(|findings| !findings.is_empty())
        .ok_or_else(|| "release feedback snapshot has no qualification findings".to_owned())?;
    for finding in findings {
        for field in ["id", "evidence", "resolution"] {
            if finding[field]
                .as_str()
                .is_none_or(|value| value.trim().is_empty())
            {
                return Err(format!(
                    "release qualification finding is missing `{field}`"
                ));
            }
        }
    }

    let roadmap = &snapshot["next_roadmap"];
    let roadmap_path = roadmap["path"]
        .as_str()
        .ok_or_else(|| "release feedback snapshot has no next-roadmap path".to_owned())?;
    if roadmap_path != "docs/superpowers/plans/2026-07-18-post-0.7-roadmap.md" {
        return Err("release feedback snapshot references an unexpected next roadmap".to_owned());
    }
    let roadmap_status = roadmap["status"]
        .as_str()
        .ok_or_else(|| "release feedback snapshot has no next-roadmap status".to_owned())?;
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let (required_stage, required_status) = feedback_publication_requirements(&version);
    if required_stage.is_some_and(|required| snapshot_stage != required)
        || roadmap_status != required_status
    {
        return Err(format!(
            "release feedback snapshot must be stage {} with roadmap status `{required_status}` for version {version}",
            required_stage.unwrap_or("alpha, beta, or rc")
        ));
    }
    let roadmap_file = root.join(roadmap_path);
    let roadmap_body = fs::read_to_string(&roadmap_file).map_err(|error| {
        format!(
            "next roadmap is missing at {}: {error}",
            roadmap_file.display()
        )
    })?;
    check_local_markdown_links(&root, &roadmap_file, &roadmap_body)?;
    let required_status_marker = if required_status == "published" {
        "**Status:** Published"
    } else {
        "**Status:** Draft"
    };
    if !roadmap_body.contains(required_status_marker) {
        return Err(format!(
            "next roadmap is missing status marker `{required_status_marker}`"
        ));
    }
    for required in [
        "Measured baseline",
        "No public user issue",
        "maintainer verification",
        "Beta/RC refresh gate",
    ] {
        if !roadmap_body.contains(required) {
            return Err(format!("next roadmap is missing `{required}`"));
        }
    }
    println!(
        "release feedback: ok ({snapshot_stage}, {total} public issues, {total_downloads} downloads, captured {captured_at})"
    );
    Ok(())
}

fn feedback_publication_requirements(version: &Version) -> (Option<&'static str>, &'static str) {
    if version.pre.is_empty() {
        (Some("rc"), "published")
    } else {
        (None, "draft")
    }
}

fn check_release_qualification() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/qualification-ledger.json");
    let ledger: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "release qualification ledger is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("release qualification ledger is invalid JSON: {error}"))?;
    if ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA {
        return Err("release qualification ledger schema is invalid".to_owned());
    }
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let stage = ReleaseStage::from_version(&version)?;
    let required_status = qualification_status_for_stage(stage);
    if ledger["workspace_stage"] != stage.as_str() || ledger["status"] != required_status {
        return Err(format!(
            "release qualification ledger must be `{required_status}` for {} stage",
            stage.as_str()
        ));
    }

    let feature_freeze = &ledger["feature_freeze"];
    let freeze_status = required_string(feature_freeze, "status", "feature freeze")?;
    if !matches!(freeze_status, "planned" | "frozen") {
        return Err("release feature-freeze status is invalid".to_owned());
    }
    if freeze_status == "planned" && !feature_freeze["baseline_commit"].is_null() {
        return Err("a planned feature freeze cannot claim a baseline commit".to_owned());
    }
    if freeze_status == "frozen" {
        let baseline = required_string(feature_freeze, "baseline_commit", "feature freeze")?;
        validate_lower_hex("feature-freeze baseline commit", baseline, 40)?;
    }
    if matches!(stage, ReleaseStage::ReleaseCandidate | ReleaseStage::Stable)
        && freeze_status != "frozen"
    {
        return Err("RC and Stable stages require a frozen feature baseline".to_owned());
    }
    let allowed_change_classes = feature_freeze["allowed_change_classes"]
        .as_array()
        .ok_or_else(|| "feature freeze has no allowed change classes".to_owned())?;
    let expected_change_classes =
        BTreeSet::from(["release-blocker", "release-evidence", "documentation"]);
    let actual_change_classes = allowed_change_classes
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if actual_change_classes != expected_change_classes
        || actual_change_classes.len() != allowed_change_classes.len()
    {
        return Err("feature-freeze allowed change classes differ".to_owned());
    }

    let package_managers = &ledger["package_managers"];
    let channels = package_managers["channels"]
        .as_array()
        .ok_or_else(|| "release qualification package-manager channels are missing".to_owned())?;
    let expected_channels = BTreeSet::from(["homebrew-cask", "scoop", "winget"]);
    let actual_channels = channels
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if actual_channels != expected_channels || actual_channels.len() != channels.len() {
        return Err("release qualification package-manager channels differ".to_owned());
    }
    let beta_status = required_string(&ledger["beta"], "status", "Beta qualification")?;
    if !matches!(beta_status, "pending" | "qualified") {
        return Err("Beta qualification status is invalid".to_owned());
    }
    if matches!(stage, ReleaseStage::ReleaseCandidate | ReleaseStage::Stable)
        && beta_status != "qualified"
    {
        return Err("RC and Stable stages require a qualified Beta".to_owned());
    }
    let release_candidates = ledger["release_candidates"]
        .as_array()
        .ok_or_else(|| "release candidate qualification must be an array".to_owned())?;
    for (section, allowed) in [
        ("upgrade_matrix", &["pending", "passed"][..]),
        (
            "documentation_uninstall",
            &["prepared-local", "prepared-native", "passed"][..],
        ),
        ("package_managers", &["candidates-only", "passed"][..]),
    ] {
        let status = required_string(&ledger[section], "status", section)?;
        if !allowed.contains(&status) {
            return Err(format!(
                "release qualification `{section}` status is invalid"
            ));
        }
    }
    validate_documentation_uninstall_progress(&ledger["documentation_uninstall"])?;

    let release_notes = &ledger["release_notes"];
    let release_notes_status = required_string(release_notes, "status", "release notes")?;
    if !matches!(
        release_notes_status,
        "alpha-current" | "beta-current" | "rc-final" | "stable-final"
    ) {
        return Err("release notes qualification status is invalid".to_owned());
    }
    for (field, expected_path) in [
        ("notes", "release/RELEASE_NOTES.md"),
        ("rollback", "docs/guides/upgrade-and-rollback.md"),
    ] {
        if release_notes[field] != expected_path || !root.join(expected_path).is_file() {
            return Err(format!(
                "release qualification {field} path must be `{expected_path}`"
            ));
        }
    }

    if matches!(stage, ReleaseStage::Stable) {
        validate_stable_qualification(&ledger)?;
    } else if ledger["stable_authorized"] != false {
        return Err("a prerelease qualification ledger cannot authorize Stable".to_owned());
    }
    if matches!(stage, ReleaseStage::ReleaseCandidate) && release_candidates.is_empty() {
        println!(
            "release qualification: RC evidence collection has not recorded a clean-tag matrix yet"
        );
    }
    println!(
        "release qualification: ok ({required_status}, stage {})",
        stage.as_str()
    );
    Ok(())
}

fn validate_documentation_uninstall_progress(value: &Value) -> Result<(), String> {
    let status = required_string(value, "status", "documentation/uninstall qualification")?;
    let run = value["native_matrix_run"].as_u64().filter(|run| *run > 0);
    let evidence_is_complete = value["evidence"].as_array().is_some_and(|items| {
        !items.is_empty()
            && items
                .iter()
                .all(|item| item.as_str().is_some_and(|item| !item.is_empty()))
    });
    match status {
        "prepared-local" if run.is_some() => Err(
            "local documentation/uninstall preparation cannot claim a native matrix run".to_owned(),
        ),
        "prepared-local" => Ok(()),
        "prepared-native" | "passed" if run.is_none() => Err(format!(
            "`{status}` documentation/uninstall evidence requires a native matrix run"
        )),
        "prepared-native" | "passed" if !evidence_is_complete => Err(format!(
            "`{status}` documentation/uninstall evidence requires non-empty evidence"
        )),
        "prepared-native" | "passed" => Ok(()),
        _ => Err("documentation/uninstall qualification status is invalid".to_owned()),
    }
}

fn qualification_status_for_stage(stage: ReleaseStage) -> &'static str {
    match stage {
        ReleaseStage::Alpha => "pre-beta",
        ReleaseStage::Beta => "beta-qualifying",
        ReleaseStage::ReleaseCandidate => "rc-qualifying",
        ReleaseStage::Stable => "qualified",
    }
}

fn validate_stable_qualification(ledger: &Value) -> Result<(), String> {
    let feature_freeze = &ledger["feature_freeze"];
    if feature_freeze["status"] != "frozen" {
        return Err("Stable requires a frozen feature baseline".to_owned());
    }
    let freeze_commit = required_string(feature_freeze, "baseline_commit", "feature freeze")?;
    validate_lower_hex("feature-freeze baseline commit", freeze_commit, 40)?;

    let beta = &ledger["beta"];
    if beta["status"] != "qualified" {
        return Err("Stable requires a qualified signed Beta".to_owned());
    }
    validate_qualification_release(beta, ReleaseStage::Beta, "Beta")?;
    let signing_targets = beta["signing_evidence_targets"]
        .as_array()
        .ok_or_else(|| "qualified Beta signing targets are missing".to_owned())?;
    let expected_signing_targets = BTreeSet::from([
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
    ]);
    let actual_signing_targets = signing_targets
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if actual_signing_targets != expected_signing_targets
        || actual_signing_targets.len() != signing_targets.len()
    {
        return Err("qualified Beta signing evidence targets differ".to_owned());
    }

    let candidates = ledger["release_candidates"]
        .as_array()
        .filter(|entries| entries.len() >= 2)
        .ok_or_else(|| "Stable requires two successful clean-tag RC matrices".to_owned())?;
    let mut tags = BTreeSet::new();
    let mut commits = BTreeSet::new();
    let mut runs = BTreeSet::new();
    for candidate in candidates {
        if candidate["status"] != "success" {
            return Err("every Stable RC matrix must have success status".to_owned());
        }
        let (tag, commit, run) =
            validate_qualification_release(candidate, ReleaseStage::ReleaseCandidate, "RC")?;
        if !tags.insert(tag) || !commits.insert(commit) || !runs.insert(run) {
            return Err(
                "Stable RC matrices must use distinct tags, commits, and run IDs".to_owned(),
            );
        }
    }

    for (section, expected_status) in [
        ("upgrade_matrix", "passed"),
        ("documentation_uninstall", "passed"),
        ("package_managers", "passed"),
    ] {
        let evidence = &ledger[section];
        if evidence["status"] != expected_status
            || evidence["evidence"].as_array().is_none_or(|items| {
                items.is_empty()
                    || items
                        .iter()
                        .any(|item| item.as_str().is_none_or(str::is_empty))
            })
        {
            return Err(format!("Stable requires passed `{section}` evidence"));
        }
    }
    let documentation_run = ledger["documentation_uninstall"]["native_matrix_run"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| "Stable documentation/uninstall evidence has no native run ID".to_owned())?;
    if !runs.contains(&documentation_run) {
        return Err(
            "Stable documentation/uninstall evidence must come from one qualified RC matrix"
                .to_owned(),
        );
    }
    if ledger["release_notes"]["status"] != "stable-final" || ledger["stable_authorized"] != true {
        return Err("Stable release notes or authorization are incomplete".to_owned());
    }
    Ok(())
}

fn validate_qualification_release(
    value: &Value,
    expected_stage: ReleaseStage,
    context: &str,
) -> Result<(String, String, u64), String> {
    let tag = required_string(value, "tag", context)?;
    let version = Version::parse(
        tag.strip_prefix('v')
            .ok_or_else(|| format!("{context} tag must start with `v`"))?,
    )
    .map_err(|error| format!("{context} tag is invalid SemVer: {error}"))?;
    if ReleaseStage::from_version(&version)? != expected_stage {
        return Err(format!("{context} tag has the wrong release stage"));
    }
    let commit = required_string(value, "source_commit", context)?;
    validate_lower_hex(&format!("{context} source commit"), commit, 40)?;
    let run = value["signed_matrix_run"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| format!("{context} has no signed matrix run ID"))?;
    Ok((tag.to_owned(), commit.to_owned(), run))
}

fn build_beta_contract_freeze() -> Result<Value, String> {
    let root = repository_root();
    let schema_root = schema_directory();
    let schema_names = json_files(&schema_root)?.into_iter().collect::<Vec<_>>();
    if schema_names.len() != generate_public_schemas().len() {
        return Err("public schema inventory is incomplete before Beta freeze".to_owned());
    }
    let schema_entries = schema_names
        .iter()
        .map(|name| {
            read_frozen_contract_text(&schema_root.join(name), "schema")
                .map(|bytes| (name.clone(), bytes))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let snapshot_names = ["agent-capabilities.json", "agent-context.json"];
    let snapshot_root = root.join("crates/canisend-cli/tests/snapshots");
    let snapshot_entries = snapshot_names
        .iter()
        .map(|name| {
            let path = snapshot_root.join(name);
            let mut value: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
                format!(
                    "could not read frozen agent snapshot {}: {error}",
                    path.display()
                )
            })?)
            .map_err(|error| format!("agent snapshot `{name}` is invalid JSON: {error}"))?;
            normalize_product_version(&mut value);
            serde_json::to_vec(&value)
                .map(|bytes| ((*name).to_owned(), bytes))
                .map_err(|error| format!("could not normalize agent snapshot `{name}`: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let migrations = migration_inventory()?;
    let current_schema_version = migrations
        .last()
        .map(|(version, _, _)| *version)
        .ok_or_else(|| "workspace migration inventory is empty".to_owned())?;
    let declared_schema_version = declared_database_schema_version()?;
    if current_schema_version != declared_schema_version {
        return Err(format!(
            "database schema constant {declared_schema_version} does not match migration inventory {current_schema_version}"
        ));
    }
    let frozen_migrations = migrations
        .iter()
        .filter(|(version, _, _)| *version <= FROZEN_MIGRATIONS_THROUGH)
        .map(|(_, name, path)| {
            read_frozen_contract_text(path, "migration").map(|bytes| (name.clone(), bytes))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if frozen_migrations.len() != FROZEN_MIGRATIONS_THROUGH as usize {
        return Err(format!(
            "expected migrations 1 through {FROZEN_MIGRATIONS_THROUGH} to exist"
        ));
    }

    Ok(json!({
        "schema": BETA_CONTRACT_FREEZE_SCHEMA,
        "baseline": {
            "release": NATIVE_ALPHA_TAG,
            "source_commit": NATIVE_ALPHA_SOURCE
        },
        "agent": {
            "protocol": AGENT_PROTOCOL,
            "public_schema_version": PUBLIC_SCHEMA_VERSION,
            "public_schema_files": schema_entries.len(),
            "public_schema_tree_sha256": digest_named_bytes(&schema_entries),
            "normalized_snapshot_files": snapshot_names,
            "normalized_snapshot_tree_sha256": digest_named_bytes(&snapshot_entries),
            "product_version_is_excluded_from_snapshot_digest": true
        },
        "workspace": {
            "format": WORKSPACE_FORMAT,
            "current_database_schema_version": current_schema_version,
            "frozen_migrations_through": FROZEN_MIGRATIONS_THROUGH,
            "frozen_migration_tree_sha256": digest_named_bytes(&frozen_migrations),
            "migration_policy": "append-only",
            "reject_future_schema_versions": true
        }
    }))
}

fn read_frozen_contract_text(path: &Path, kind: &str) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("could not read frozen {kind} `{}`: {error}", path.display()))?;
    canonicalize_frozen_contract_text(&bytes).map_err(|error| {
        format!(
            "frozen {kind} `{}` is not canonical text: {error}",
            path.display()
        )
    })
}

fn canonicalize_frozen_contract_text(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| format!("invalid UTF-8: {error}"))?;
    let normalized = text.replace("\r\n", "\n");
    if normalized.contains('\r') {
        return Err("bare carriage returns are not supported".to_owned());
    }
    Ok(normalized.into_bytes())
}

fn migration_inventory() -> Result<Vec<(u32, String, PathBuf)>, String> {
    let directory = repository_root().join("crates/canisend-store/migrations");
    let mut migrations = Vec::new();
    for entry in fs::read_dir(&directory).map_err(|error| {
        format!(
            "could not inspect migrations at {}: {error}",
            directory.display()
        )
    })? {
        let entry = entry.map_err(|error| format!("could not inspect migration: {error}"))?;
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "sql") {
            continue;
        }
        reject_symlink(&path)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let prefix = name
            .split_once('_')
            .map(|(prefix, _)| prefix)
            .filter(|prefix| prefix.len() == 4 && prefix.bytes().all(|byte| byte.is_ascii_digit()))
            .ok_or_else(|| format!("migration name is not versioned: `{name}`"))?;
        let version = prefix
            .parse::<u32>()
            .map_err(|error| format!("migration version is invalid in `{name}`: {error}"))?;
        migrations.push((version, name, path));
    }
    migrations.sort_by_key(|(version, _, _)| *version);
    for (index, (version, name, _)) in migrations.iter().enumerate() {
        let expected =
            u32::try_from(index + 1).map_err(|_| "migration inventory exceeds u32".to_owned())?;
        if *version != expected {
            return Err(format!(
                "migration inventory is not contiguous at `{name}`: expected {expected}, found {version}"
            ));
        }
    }
    Ok(migrations)
}

fn declared_database_schema_version() -> Result<u32, String> {
    let path = repository_root().join("crates/canisend-store/src/database.rs");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let prefix = "pub const DATABASE_SCHEMA_VERSION: u32 = ";
    source
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .and_then(|value| value.strip_suffix(';'))
        .ok_or_else(|| "DATABASE_SCHEMA_VERSION declaration is missing".to_owned())?
        .parse()
        .map_err(|error| format!("DATABASE_SCHEMA_VERSION is invalid: {error}"))
}

fn normalize_product_version(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(normalize_product_version),
        Value::Object(fields) => {
            for (name, field) in fields {
                if name == "product_version" {
                    *field = Value::String("<release-version>".to_owned());
                } else {
                    normalize_product_version(field);
                }
            }
        }
        _ => {}
    }
}

fn digest_named_bytes(entries: &[(String, Vec<u8>)]) -> String {
    let mut digest = Sha256::new();
    for (name, bytes) in entries {
        digest.update((name.len() as u64).to_be_bytes());
        digest.update(name.as_bytes());
        digest.update((bytes.len() as u64).to_be_bytes());
        digest.update(bytes);
    }
    hex::encode(digest.finalize())
}

fn release_targets() -> Result<Vec<ReleaseTarget>, String> {
    let path = repository_root().join("release/targets.json");
    let body = fs::read_to_string(&path)
        .map_err(|error| format!("release targets are missing at {}: {error}", path.display()))?;
    let document: Value = serde_json::from_str(&body)
        .map_err(|error| format!("release targets are invalid JSON: {error}"))?;
    if document["schema"] != RELEASE_TARGET_SCHEMA {
        return Err(format!(
            "release target schema must be `{RELEASE_TARGET_SCHEMA}`"
        ));
    }
    let entries = document["targets"]
        .as_array()
        .ok_or_else(|| "release targets must contain an array".to_owned())?;
    let mut targets = Vec::with_capacity(entries.len());
    let mut triples = BTreeSet::new();
    for entry in entries {
        let field = |name: &str| -> Result<String, String> {
            entry[name]
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("release target field `{name}` is missing"))
        };
        let target = ReleaseTarget {
            triple: field("triple")?,
            runner: field("runner")?,
            executable: field("executable")?,
            archive: field("archive")?,
            signing: field("signing")?,
        };
        if !triples.insert(target.triple.clone()) {
            return Err(format!("duplicate release target `{}`", target.triple));
        }
        if !matches!(target.archive.as_str(), "tar.gz" | "zip") {
            return Err(format!(
                "unsupported release archive `{}` for {}",
                target.archive, target.triple
            ));
        }
        targets.push(target);
    }
    if targets.is_empty() {
        return Err("release target list is empty".to_owned());
    }
    Ok(targets)
}

fn write_channel_candidates(tag: &str, assets: &Path, output: &Path) -> Result<(), String> {
    verify_release(tag, assets)?;
    if output.exists() {
        return Err(format!(
            "channel candidate output must not already exist: {}",
            output.display()
        ));
    }
    let version = env!("CARGO_PKG_VERSION");
    let manifest_path = assets.join(format!("canisend-{version}-manifest.json"));
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| {
        format!(
            "release manifest is missing at {}: {error}",
            manifest_path.display()
        )
    })?)
    .map_err(|error| format!("release manifest is invalid JSON: {error}"))?;
    let source = build_channel_candidate_source(tag, &manifest_path, &manifest)?;
    let files = render_channel_candidates(&source)?;

    fs::create_dir_all(output).map_err(|error| {
        format!(
            "could not create channel candidate output {}: {error}",
            output.display()
        )
    })?;
    write_pretty_json(&output.join("candidate-source.json"), &source.to_value())?;
    for (relative, body) in files {
        let path = output.join(&relative);
        let parent = path
            .parent()
            .ok_or_else(|| format!("candidate file has no parent: {relative}"))?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        fs::write(&path, body)
            .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    }
    check_channel_candidate_directory(output)?;
    println!(
        "channel candidates: wrote Homebrew, Scoop, and WinGet candidates to {}",
        output.display()
    );
    Ok(())
}

fn build_channel_candidate_source(
    tag: &str,
    manifest_path: &Path,
    manifest: &Value,
) -> Result<ChannelCandidateSource, String> {
    let version = required_string(manifest, "version", "release manifest")?.to_owned();
    let parsed = Version::parse(&version)
        .map_err(|error| format!("release manifest version is invalid SemVer: {error}"))?;
    let stage = ReleaseStage::from_version(&parsed)?;
    if manifest["tag"] != tag || manifest["stage"] != stage.as_str() {
        return Err("release manifest stage does not match the channel candidate tag".to_owned());
    }
    let source_commit = required_string(&manifest["source"], "commit", "release source")?;
    let repository = required_string(&manifest["source"], "repository", "release source")?;
    let entries = manifest["artifacts"]
        .as_array()
        .ok_or_else(|| "release manifest artifacts are missing".to_owned())?;
    let required_targets = BTreeSet::from([
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
    ]);
    let mut artifacts = BTreeMap::new();
    for entry in entries {
        let target = required_string(entry, "target", "release artifact")?;
        if !required_targets.contains(target) {
            continue;
        }
        let artifact = ChannelArtifact {
            target: target.to_owned(),
            archive: required_string(entry, "archive", "release artifact")?.to_owned(),
            sha256: required_string(entry, "sha256", "release artifact")?.to_owned(),
            size: entry["size"]
                .as_u64()
                .ok_or_else(|| format!("release artifact `{target}` has no size"))?,
        };
        if artifacts.insert(target.to_owned(), artifact).is_some() {
            return Err(format!("duplicate channel artifact `{target}`"));
        }
    }
    if artifacts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != required_targets
    {
        return Err("release manifest does not contain all channel candidate artifacts".to_owned());
    }
    let manifest_file = manifest_path
        .file_name()
        .ok_or_else(|| "release manifest has no file name".to_owned())?
        .to_string_lossy()
        .into_owned();
    let candidate = ChannelCandidateSource {
        tag: tag.to_owned(),
        version,
        stage,
        source_commit: source_commit.to_owned(),
        repository: repository.to_owned(),
        manifest_file,
        manifest_sha256: sha256_file(manifest_path)?,
        artifacts,
    };
    channel_candidate_source_from_value(&candidate.to_value())
}

fn channel_candidate_source_from_value(value: &Value) -> Result<ChannelCandidateSource, String> {
    if value["schema"] != CHANNEL_CANDIDATE_SOURCE_SCHEMA
        || value["candidate_only"] != true
        || value["publication_authorized"] != false
    {
        return Err(
            "channel candidate source must be candidate-only and must not authorize publication"
                .to_owned(),
        );
    }
    let release = &value["release"];
    let tag = required_string(release, "tag", "channel release")?.to_owned();
    let version = required_string(release, "version", "channel release")?.to_owned();
    let parsed = Version::parse(&version)
        .map_err(|error| format!("channel candidate version is invalid SemVer: {error}"))?;
    if tag != format!("v{version}") {
        return Err(format!(
            "channel candidate tag `{tag}` does not match version `{version}`"
        ));
    }
    let stage = ReleaseStage::from_version(&parsed)?;
    if release["stage"] != stage.as_str() {
        return Err("channel candidate stage does not match its version".to_owned());
    }
    let source_commit = required_string(release, "source_commit", "channel release")?.to_owned();
    validate_lower_hex("channel source commit", &source_commit, 40)?;
    let repository = required_string(release, "repository", "channel release")?.to_owned();
    if repository != env!("CARGO_PKG_REPOSITORY") {
        return Err(format!(
            "channel candidate repository must be `{}`",
            env!("CARGO_PKG_REPOSITORY")
        ));
    }
    let manifest_file = required_string(release, "manifest_file", "channel release")?.to_owned();
    if manifest_file != format!("canisend-{version}-manifest.json") {
        return Err("channel candidate manifest file does not match its version".to_owned());
    }
    let manifest_sha256 =
        required_string(release, "manifest_sha256", "channel release")?.to_owned();
    validate_lower_hex("channel release manifest SHA-256", &manifest_sha256, 64)?;

    let entries = value["artifacts"]
        .as_array()
        .ok_or_else(|| "channel candidate artifacts must be an array".to_owned())?;
    let expected = BTreeMap::from([
        ("aarch64-apple-darwin", "tar.gz"),
        ("x86_64-apple-darwin", "tar.gz"),
        ("x86_64-pc-windows-msvc", "zip"),
    ]);
    let mut artifacts = BTreeMap::new();
    for entry in entries {
        let target = required_string(entry, "target", "channel artifact")?.to_owned();
        let archive = required_string(entry, "archive", "channel artifact")?.to_owned();
        let sha256 = required_string(entry, "sha256", "channel artifact")?.to_owned();
        let size = entry["size"]
            .as_u64()
            .filter(|size| *size > 0)
            .ok_or_else(|| format!("channel artifact `{target}` has no positive size"))?;
        let extension = expected
            .get(target.as_str())
            .ok_or_else(|| format!("unsupported channel artifact target `{target}`"))?;
        let expected_archive = format!("canisend-{version}-{target}.{extension}");
        if archive != expected_archive {
            return Err(format!(
                "channel artifact `{target}` must be named `{expected_archive}`"
            ));
        }
        validate_lower_hex(&format!("channel artifact `{target}` SHA-256"), &sha256, 64)?;
        let artifact = ChannelArtifact {
            target: target.clone(),
            archive,
            sha256,
            size,
        };
        if artifacts.insert(target.clone(), artifact).is_some() {
            return Err(format!("duplicate channel artifact `{target}`"));
        }
    }
    if artifacts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected.keys().copied().collect()
    {
        return Err("channel candidate source has an incomplete artifact set".to_owned());
    }
    let source = ChannelCandidateSource {
        tag,
        version,
        stage,
        source_commit,
        repository,
        manifest_file,
        manifest_sha256,
        artifacts,
    };
    if source.to_value() != *value {
        return Err("channel candidate source contains unknown or non-canonical fields".to_owned());
    }
    Ok(source)
}

fn render_channel_candidates(
    source: &ChannelCandidateSource,
) -> Result<BTreeMap<String, String>, String> {
    let arm = source.artifact("aarch64-apple-darwin")?;
    let intel = source.artifact("x86_64-apple-darwin")?;
    let windows = source.artifact("x86_64-pc-windows-msvc")?;
    let download = |archive: &str| {
        format!(
            "{}/releases/download/{}/{}",
            source.repository, source.tag, archive
        )
    };
    let homebrew = format!(
        r##"cask "canisend" do
  arch arm: "aarch64", intel: "x86_64"

  version "{version}"
  sha256 arm:   "{arm_sha256}",
         intel: "{intel_sha256}"

  url "{repository}/releases/download/v#{{version}}/canisend-#{{version}}-#{{arch}}-apple-darwin.tar.gz"
  name "CanISend"
  desc "Prepare evidence-backed academic job applications with agent hosts"
  homepage "{repository}"

  binary "canisend-#{{version}}-#{{arch}}-apple-darwin/canisend"
end
"##,
        repository = source.repository,
        version = source.version,
        arm_sha256 = arm.sha256,
        intel_sha256 = intel.sha256,
    );
    let scoop = serde_json::to_string_pretty(&json!({
        "version": source.version,
        "description": "Prepare evidence-backed academic job applications with agent hosts",
        "homepage": source.repository,
        "license": "MIT",
        "architecture": {
            "64bit": {
                "url": download(&windows.archive),
                "hash": windows.sha256,
            }
        },
        "extract_dir": format!("canisend-{}-x86_64-pc-windows-msvc", source.version),
        "bin": "canisend.exe",
    }))
    .map_err(|error| format!("could not render Scoop candidate: {error}"))?
        + "\n";

    let identifier = "PengJiaxin.CanISend";
    let winget_base = format!("winget/manifests/p/PengJiaxin/CanISend/{}/", source.version);
    let winget_version = format!(
        r#"# yaml-language-server: $schema=https://aka.ms/winget-manifest.version.{schema}.schema.json

PackageIdentifier: {identifier}
PackageVersion: {version}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: {schema}
"#,
        schema = WINGET_MANIFEST_VERSION,
        version = source.version,
    );
    let winget_locale = format!(
        r#"# yaml-language-server: $schema=https://aka.ms/winget-manifest.defaultLocale.{schema}.schema.json

PackageIdentifier: {identifier}
PackageVersion: {version}
PackageLocale: en-US
Publisher: Peng Jiaxin
PublisherUrl: https://github.com/jxpeng98
PublisherSupportUrl: {repository}/issues
PackageName: CanISend
PackageUrl: {repository}
License: MIT
LicenseUrl: {repository}/blob/{tag}/LICENSE
ShortDescription: Prepare evidence-backed academic job applications with agent hosts
Moniker: canisend
Tags:
- academic-jobs
- agent
- cli
ReleaseNotesUrl: {repository}/releases/tag/{tag}
ManifestType: defaultLocale
ManifestVersion: {schema}
"#,
        schema = WINGET_MANIFEST_VERSION,
        version = source.version,
        repository = source.repository,
        tag = source.tag,
    );
    let winget_installer = format!(
        r#"# yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.{schema}.schema.json

PackageIdentifier: {identifier}
PackageVersion: {version}
InstallerType: zip
NestedInstallerType: portable
NestedInstallerFiles:
- RelativeFilePath: canisend-{version}-x86_64-pc-windows-msvc\canisend.exe
  PortableCommandAlias: canisend
UpgradeBehavior: install
Installers:
- Architecture: x64
  InstallerUrl: {url}
  InstallerSha256: {sha256}
ManifestType: installer
ManifestVersion: {schema}
"#,
        schema = WINGET_MANIFEST_VERSION,
        version = source.version,
        url = download(&windows.archive),
        sha256 = windows.sha256,
    );

    Ok(BTreeMap::from([
        ("homebrew/Casks/canisend.rb".to_owned(), homebrew),
        ("scoop/bucket/canisend.json".to_owned(), scoop),
        (format!("{winget_base}{identifier}.yaml"), winget_version),
        (
            format!("{winget_base}{identifier}.locale.en-US.yaml"),
            winget_locale,
        ),
        (
            format!("{winget_base}{identifier}.installer.yaml"),
            winget_installer,
        ),
    ]))
}

fn check_channel_candidates() -> Result<(), String> {
    let root = repository_root().join("packaging/candidates");
    let mut entries = fs::read_dir(&root)
        .map_err(|error| format!("channel candidate directory is missing: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("could not inspect channel candidates: {error}"))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    if entries.is_empty() {
        return Err("no package-manager channel candidates exist".to_owned());
    }
    let mut has_alpha_baseline = false;
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(format!(
                "channel candidate entry must be a regular directory: {}",
                path.display()
            ));
        }
        let source = check_channel_candidate_directory(&path)?;
        if path.file_name().and_then(|name| name.to_str()) != Some(source.tag.as_str()) {
            return Err(format!(
                "channel candidate directory {} must be named `{}`",
                path.display(),
                source.tag
            ));
        }
        if source.tag == NATIVE_ALPHA_TAG && source.source_commit == NATIVE_ALPHA_SOURCE {
            has_alpha_baseline = true;
        }
    }
    if !has_alpha_baseline {
        return Err(
            "channel candidates do not retain the qualified native Alpha baseline".to_owned(),
        );
    }
    println!("channel candidates: ok");
    Ok(())
}

fn check_package_manager_qualification_policy() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/package-manager-qualification-policy.json");
    let actual: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "package-manager qualification policy is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("package-manager qualification policy is invalid JSON: {error}"))?;
    let lifecycle = [
        "install-beta",
        "run-version-and-doctor",
        "create-external-workspace",
        "upgrade-to-rc",
        "run-version-and-doctor",
        "uninstall",
        "prove-workspace-retained",
    ];
    let expected = json!({
        "schema": PACKAGE_MANAGER_QUALIFICATION_POLICY_SCHEMA,
        "publication_authorized": false,
        "release_pair": {
            "from_stage": "beta",
            "to_stage": "rc",
            "same_release_line": true,
            "require_public_signed_candidates": true
        },
        "channels": [
            {
                "id": "homebrew-cask",
                "targets": ["aarch64-apple-darwin", "x86_64-apple-darwin"],
                "official_validators": ["brew-style", "brew-audit-strict-cask"],
                "lifecycle": lifecycle
            },
            {
                "id": "scoop",
                "targets": ["x86_64-pc-windows-msvc"],
                "official_validators": ["scoop-manifest-install"],
                "lifecycle": lifecycle
            },
            {
                "id": "winget",
                "targets": ["x86_64-pc-windows-msvc"],
                "official_validators": ["winget-validate", "winget-sandbox-test"],
                "lifecycle": [
                    "validate-beta-manifest",
                    "validate-rc-manifest",
                    "sandbox-install-beta",
                    "run-version-and-doctor",
                    "create-external-workspace",
                    "upgrade-to-rc",
                    "run-version-and-doctor",
                    "uninstall",
                    "prove-workspace-retained"
                ]
            }
        ],
        "evidence": {
            "schema": "canisend.package-manager-qualification/v1",
            "required_records": [
                "homebrew-aarch64-apple-darwin",
                "homebrew-x86_64-apple-darwin",
                "scoop-x86_64-pc-windows-msvc",
                "winget-x86_64-pc-windows-msvc"
            ],
            "bind_candidate_source_sha256": true,
            "bind_github_run_id": true,
            "required_checks": [
                "candidate-sources-verified",
                "official-validation",
                "install",
                "from-version",
                "from-doctor",
                "workspace-created",
                "upgrade",
                "to-version",
                "to-doctor",
                "uninstall",
                "workspace-retained",
                "no-publication"
            ],
            "all_checks_must_pass": true
        }
    });
    if actual != expected {
        return Err(
            "package-manager qualification policy differs from the native release contract"
                .to_owned(),
        );
    }
    let documentation_path = root.join("docs/release/package-manager-qualification.md");
    let documentation = fs::read_to_string(&documentation_path).map_err(|error| {
        format!("package-manager qualification documentation is missing: {error}")
    })?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    println!("package-manager qualification policy: ok (4 native records)");
    Ok(())
}

fn verify_package_manager_evidence(
    from_tag: &str,
    to_tag: &str,
    directory: &Path,
) -> Result<(), String> {
    let (from_version, from_stage) = parse_release_tag(from_tag)?;
    let (to_version, to_stage) = parse_release_tag(to_tag)?;
    if from_stage != ReleaseStage::Beta || to_stage != ReleaseStage::ReleaseCandidate {
        return Err("package-manager qualification requires a Beta-to-RC tag pair".to_owned());
    }
    if (from_version.major, from_version.minor, from_version.patch)
        != (to_version.major, to_version.minor, to_version.patch)
    {
        return Err("package-manager qualification tags must use the same release line".to_owned());
    }
    let expected = BTreeMap::from([
        (
            "homebrew-aarch64-apple-darwin.json",
            (
                "homebrew-aarch64-apple-darwin",
                "homebrew-cask",
                "aarch64-apple-darwin",
                "macos-15",
            ),
        ),
        (
            "homebrew-x86_64-apple-darwin.json",
            (
                "homebrew-x86_64-apple-darwin",
                "homebrew-cask",
                "x86_64-apple-darwin",
                "macos-15-intel",
            ),
        ),
        (
            "scoop-x86_64-pc-windows-msvc.json",
            (
                "scoop-x86_64-pc-windows-msvc",
                "scoop",
                "x86_64-pc-windows-msvc",
                "windows-2025",
            ),
        ),
        (
            "winget-x86_64-pc-windows-msvc.json",
            (
                "winget-x86_64-pc-windows-msvc",
                "winget",
                "x86_64-pc-windows-msvc",
                "windows-sandbox",
            ),
        ),
    ]);
    let mut actual_paths = BTreeSet::new();
    collect_relative_files(directory, directory, &mut actual_paths)?;
    if actual_paths != expected.keys().map(|name| (*name).to_owned()).collect() {
        return Err(format!(
            "package-manager evidence file set differs: expected {:?}, found {actual_paths:?}",
            expected.keys().collect::<Vec<_>>()
        ));
    }

    let mut run_ids = BTreeSet::new();
    let mut from_digests = BTreeSet::new();
    let mut to_digests = BTreeSet::new();
    for (file, (record, channel, target, environment)) in expected {
        let path = directory.join(file);
        reject_symlink(&path)?;
        let value: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
            format!(
                "could not read package-manager evidence {}: {error}",
                path.display()
            )
        })?)
        .map_err(|error| format!("package-manager evidence `{file}` is invalid JSON: {error}"))?;
        let (run_id, from_digest, to_digest) = validate_package_manager_evidence_record(
            &value,
            record,
            channel,
            target,
            environment,
            from_tag,
            to_tag,
        )?;
        run_ids.insert(run_id);
        from_digests.insert(from_digest);
        to_digests.insert(to_digest);
    }
    if run_ids.len() != 1 || from_digests.len() != 1 || to_digests.len() != 1 {
        return Err(
            "package-manager evidence records must bind one run and one shared candidate pair"
                .to_owned(),
        );
    }
    if from_digests == to_digests {
        return Err("Beta and RC candidate-source digests must differ".to_owned());
    }
    println!(
        "package-manager evidence: ok ({from_tag} -> {to_tag}, run {})",
        run_ids.first().expect("one checked run ID")
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_package_manager_evidence_record(
    value: &Value,
    expected_record: &str,
    expected_channel: &str,
    expected_target: &str,
    expected_environment: &str,
    from_tag: &str,
    to_tag: &str,
) -> Result<(u64, String, String), String> {
    let context = format!("package-manager evidence `{expected_record}`");
    let run_id = value["github_run_id"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| format!("{context} has no positive GitHub run ID"))?;
    let from_digest = required_string(value, "from_candidate_source_sha256", &context)?.to_owned();
    let to_digest = required_string(value, "to_candidate_source_sha256", &context)?.to_owned();
    validate_lower_hex(
        &format!("{context} Beta candidate digest"),
        &from_digest,
        64,
    )?;
    validate_lower_hex(&format!("{context} RC candidate digest"), &to_digest, 64)?;
    let tool_version = required_string(value, "tool_version", &context)?;
    let completed_at = required_string(value, "completed_at", &context)?;
    if !completed_at.ends_with('Z') {
        return Err(format!("{context} completion timestamp must be UTC"));
    }
    let checks = value["checks"]
        .as_object()
        .ok_or_else(|| format!("{context} checks are missing"))?;
    let required_checks = [
        "candidate-sources-verified",
        "official-validation",
        "install",
        "from-version",
        "from-doctor",
        "workspace-created",
        "upgrade",
        "to-version",
        "to-doctor",
        "uninstall",
        "workspace-retained",
        "no-publication",
    ];
    if checks.len() != required_checks.len()
        || required_checks
            .iter()
            .any(|check| checks.get(*check) != Some(&Value::Bool(true)))
    {
        return Err(format!("{context} does not pass every required check"));
    }
    let expected = json!({
        "schema": PACKAGE_MANAGER_QUALIFICATION_SCHEMA,
        "record": expected_record,
        "channel": expected_channel,
        "target": expected_target,
        "environment": expected_environment,
        "from_tag": from_tag,
        "to_tag": to_tag,
        "from_candidate_source_sha256": from_digest,
        "to_candidate_source_sha256": to_digest,
        "github_run_id": run_id,
        "tool_version": tool_version,
        "observed_versions": {
            "from": from_tag.trim_start_matches('v'),
            "to": to_tag.trim_start_matches('v')
        },
        "checks": checks,
        "completed_at": completed_at
    });
    if *value != expected {
        return Err(format!(
            "{context} contains unknown, noncanonical, or mismatched fields"
        ));
    }
    Ok((run_id, from_digest, to_digest))
}

fn parse_release_tag(tag: &str) -> Result<(Version, ReleaseStage), String> {
    let version = Version::parse(
        tag.strip_prefix('v')
            .ok_or_else(|| format!("release tag `{tag}` must start with `v`"))?,
    )
    .map_err(|error| format!("release tag `{tag}` is invalid SemVer: {error}"))?;
    let stage = ReleaseStage::from_version(&version)?;
    Ok((version, stage))
}

fn check_channel_candidate_directory(path: &Path) -> Result<ChannelCandidateSource, String> {
    let source_path = path.join("candidate-source.json");
    reject_symlink(&source_path)?;
    let source_value: Value = serde_json::from_slice(&fs::read(&source_path).map_err(|error| {
        format!(
            "candidate source is missing at {}: {error}",
            source_path.display()
        )
    })?)
    .map_err(|error| format!("candidate source is invalid JSON: {error}"))?;
    let source = channel_candidate_source_from_value(&source_value)?;
    let expected = render_channel_candidates(&source)?;
    let mut actual_paths = BTreeSet::new();
    collect_relative_files(path, path, &mut actual_paths)?;
    actual_paths.remove("candidate-source.json");
    if actual_paths != expected.keys().cloned().collect() {
        return Err(format!(
            "channel candidate file set differs at {}: expected {:?}, found {actual_paths:?}",
            path.display(),
            expected.keys().collect::<Vec<_>>()
        ));
    }
    for (relative, expected_body) in expected {
        let actual = fs::read_to_string(path.join(&relative))
            .map_err(|error| format!("could not read channel candidate `{relative}`: {error}"))?;
        if actual != expected_body {
            return Err(format!(
                "channel candidate `{relative}` drifted from its verified release source"
            ));
        }
    }
    Ok(source)
}

fn collect_relative_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeSet<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("could not inspect {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("could not inspect candidate file: {error}"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "channel candidate tree contains a symlink: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            collect_relative_files(root, &path, files)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("could not relativize candidate path: {error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            if !files.insert(relative.clone()) {
                return Err(format!("duplicate channel candidate file `{relative}`"));
            }
        } else {
            return Err(format!(
                "channel candidate is not a regular file: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn required_string<'a>(value: &'a Value, name: &str, context: &str) -> Result<&'a str, String> {
    value[name]
        .as_str()
        .filter(|field| !field.is_empty())
        .ok_or_else(|| format!("{context} field `{name}` is missing"))
}

fn validate_lower_hex(context: &str, value: &str, length: usize) -> Result<(), String> {
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{context} must be exactly {length} lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

fn bind_signing_evidence(
    tag: &str,
    target_name: &str,
    evidence_path: &Path,
    binary_path: &Path,
    archive_path: &Path,
) -> Result<(), String> {
    validate_release_tag(tag)?;
    let version = env!("CARGO_PKG_VERSION");
    let target = release_targets()?
        .into_iter()
        .find(|target| target.triple == target_name)
        .ok_or_else(|| format!("unknown release signing target `{target_name}`"))?;
    if target.signing == "none" {
        return Err(format!(
            "release target `{target_name}` does not use platform code signing"
        ));
    }
    reject_symlink(binary_path)?;
    if binary_path.file_name().and_then(|name| name.to_str()) != Some(&target.executable) {
        return Err(format!(
            "signing evidence binary must be named `{}`",
            target.executable
        ));
    }
    reject_symlink(evidence_path)?;
    reject_symlink(archive_path)?;
    let expected_archive = format!("canisend-{version}-{target_name}.{}", target.archive);
    if archive_path.file_name().and_then(|name| name.to_str()) != Some(&expected_archive) {
        return Err(format!(
            "signing evidence archive must be named `{expected_archive}`"
        ));
    }
    let actual: Value = serde_json::from_slice(&fs::read(evidence_path).map_err(|error| {
        format!(
            "signing evidence is missing at {}: {error}",
            evidence_path.display()
        )
    })?)
    .map_err(|error| format!("signing evidence is invalid JSON: {error}"))?;
    let mut canonical = canonical_signing_evidence(&actual, &target, version, None)?;
    if canonical != actual {
        return Err("unbound signing evidence contains unknown or non-canonical fields".to_owned());
    }
    if canonical["binary"]["sha256"] != sha256_file(binary_path)?
        || canonical["binary"]["size"] != file_size(binary_path)?
    {
        return Err(format!(
            "signing evidence does not match signed binary `{}`",
            binary_path.display()
        ));
    }
    canonical["archive"] = json!({
        "file": expected_archive,
        "sha256": sha256_file(archive_path)?,
        "size": file_size(archive_path)?,
    });
    let canonical = canonical_signing_evidence(&canonical, &target, version, Some(archive_path))?;
    write_pretty_json(evidence_path, &canonical)?;
    println!(
        "signing evidence: bound {target_name} to {}",
        archive_path.display()
    );
    Ok(())
}

fn read_bound_signing_evidence(
    path: &Path,
    target: &ReleaseTarget,
    version: &str,
    archive: &Path,
) -> Result<Value, String> {
    reject_symlink(path)?;
    let actual: Value =
        serde_json::from_slice(&fs::read(path).map_err(|error| {
            format!("signing evidence is missing at {}: {error}", path.display())
        })?)
        .map_err(|error| format!("signing evidence is invalid JSON: {error}"))?;
    let canonical = canonical_signing_evidence(&actual, target, version, Some(archive))?;
    if actual != canonical {
        return Err(format!(
            "signing evidence contains unknown or non-canonical fields: {}",
            path.display()
        ));
    }
    Ok(canonical)
}

fn canonical_signing_evidence(
    value: &Value,
    target: &ReleaseTarget,
    version: &str,
    archive: Option<&Path>,
) -> Result<Value, String> {
    if value["schema"] != CODE_SIGNING_EVIDENCE_SCHEMA
        || value["version"] != version
        || value["target"] != target.triple
        || value["status"] != "verified"
    {
        return Err(format!(
            "code-signing evidence identity is invalid for `{}`",
            target.triple
        ));
    }
    let expected_kind = match target.signing.as_str() {
        "apple" => "apple-developer-id-notarization",
        "authenticode" => "windows-authenticode-artifact-signing",
        other => {
            return Err(format!(
                "target `{}` has unsupported signing kind `{other}`",
                target.triple
            ));
        }
    };
    if value["kind"] != expected_kind {
        return Err(format!(
            "code-signing evidence kind is invalid for `{}`",
            target.triple
        ));
    }
    let binary = &value["binary"];
    if binary["file"] != target.executable {
        return Err(format!(
            "signed binary file is invalid for `{}`",
            target.triple
        ));
    }
    let binary_sha = required_string(binary, "sha256", "signed binary")?;
    validate_lower_hex("signed binary SHA-256", binary_sha, 64)?;
    let binary_size = binary["size"]
        .as_u64()
        .filter(|size| *size > 0)
        .ok_or_else(|| "signed binary has no positive size".to_owned())?;

    let archive_value = if let Some(archive_path) = archive {
        reject_symlink(archive_path)?;
        let expected_name = format!("canisend-{version}-{}.{}", target.triple, target.archive);
        if archive_path.file_name().and_then(|name| name.to_str()) != Some(&expected_name) {
            return Err(format!("signed archive must be named `{expected_name}`"));
        }
        let archive_value = &value["archive"];
        let archive_sha = required_string(archive_value, "sha256", "signed archive")?;
        validate_lower_hex("signed archive SHA-256", archive_sha, 64)?;
        let archive_size = archive_value["size"]
            .as_u64()
            .filter(|size| *size > 0)
            .ok_or_else(|| "signed archive has no positive size".to_owned())?;
        if archive_value["file"] != expected_name
            || archive_sha != sha256_file(archive_path)?
            || archive_size != file_size(archive_path)?
        {
            return Err(format!(
                "code-signing evidence is not bound to `{expected_name}`"
            ));
        }
        json!({
            "file": expected_name,
            "sha256": archive_sha,
            "size": archive_size,
        })
    } else {
        if !value["archive"].is_null() {
            return Err("unbound signing evidence must use a null archive".to_owned());
        }
        Value::Null
    };

    let signer = &value["signer"];
    let identity = bounded_evidence_string(signer, "identity", "signer", 256)?;
    let (canonical_signer, canonical_verification) = match target.signing.as_str() {
        "apple" => {
            if !identity.starts_with("Developer ID Application:") {
                return Err("macOS signer is not a Developer ID Application identity".to_owned());
            }
            let team_id = bounded_evidence_string(signer, "team_id", "Apple signer", 32)?;
            if team_id.len() != 10
                || !team_id
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
            {
                return Err(
                    "Apple signer team ID must be 10 uppercase alphanumeric characters".to_owned(),
                );
            }
            let code_identifier =
                bounded_evidence_string(signer, "code_identifier", "Apple signer", 128)?;
            if code_identifier != "io.github.jxpeng98.canisend" {
                return Err("Apple code-signing identifier is invalid".to_owned());
            }
            let verification = &value["verification"];
            if verification["developer_id"] != true
                || verification["hardened_runtime"] != true
                || verification["secure_timestamp"] != true
                || verification["notarization_status"] != "Accepted"
                || verification["standalone_ticket_stapled"] != false
                || verification["stapling_supported"] != false
                || verification["notary_error_count"] != 0
            {
                return Err("Apple signing/notarization evidence is incomplete".to_owned());
            }
            let submission_id = bounded_evidence_string(
                verification,
                "notary_submission_id",
                "Apple notarization",
                36,
            )?;
            validate_uuid_text("Apple notarization submission ID", submission_id)?;
            let log_sha = required_string(verification, "notary_log_sha256", "Apple notarization")?;
            validate_lower_hex("Apple notarization log SHA-256", log_sha, 64)?;
            let warning_count = verification["notary_warning_count"]
                .as_u64()
                .ok_or_else(|| "Apple notarization warning count is missing".to_owned())?;
            (
                json!({
                    "identity": identity,
                    "team_id": team_id,
                    "code_identifier": code_identifier,
                }),
                json!({
                    "developer_id": true,
                    "hardened_runtime": true,
                    "secure_timestamp": true,
                    "notarization_status": "Accepted",
                    "notary_submission_id": submission_id,
                    "notary_log_sha256": log_sha,
                    "notary_error_count": 0,
                    "notary_warning_count": warning_count,
                    "standalone_ticket_stapled": false,
                    "stapling_supported": false,
                }),
            )
        }
        "authenticode" => {
            let thumbprint = required_string(signer, "thumbprint", "Windows signer")?;
            validate_lower_hex("Windows signer thumbprint", thumbprint, 40)?;
            let verification = &value["verification"];
            if verification["authenticode_status"] != "Valid"
                || verification["file_digest"] != "SHA256"
                || verification["timestamp_digest"] != "SHA256"
                || verification["timestamp_present"] != true
                || verification["service"] != "azure-artifact-signing"
            {
                return Err("Windows Authenticode evidence is incomplete".to_owned());
            }
            let timestamp_identity = bounded_evidence_string(
                verification,
                "timestamp_identity",
                "Windows timestamp",
                256,
            )?;
            (
                json!({
                    "identity": identity,
                    "thumbprint": thumbprint,
                }),
                json!({
                    "authenticode_status": "Valid",
                    "file_digest": "SHA256",
                    "timestamp_digest": "SHA256",
                    "timestamp_present": true,
                    "timestamp_identity": timestamp_identity,
                    "service": "azure-artifact-signing",
                }),
            )
        }
        _ => unreachable!("signing kind was checked above"),
    };

    Ok(json!({
        "schema": CODE_SIGNING_EVIDENCE_SCHEMA,
        "version": version,
        "target": target.triple,
        "kind": expected_kind,
        "status": "verified",
        "binary": {
            "file": target.executable,
            "sha256": binary_sha,
            "size": binary_size,
        },
        "archive": archive_value,
        "signer": canonical_signer,
        "verification": canonical_verification,
    }))
}

fn bounded_evidence_string<'a>(
    value: &'a Value,
    name: &str,
    context: &str,
    maximum: usize,
) -> Result<&'a str, String> {
    let field = required_string(value, name, context)?;
    if field.len() > maximum || field.chars().any(char::is_control) {
        return Err(format!(
            "{context} field `{name}` exceeds its bound or contains control characters"
        ));
    }
    Ok(field)
}

fn validate_uuid_text(context: &str, value: &str) -> Result<(), String> {
    if value.len() != 36
        || value.bytes().enumerate().any(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte != b'-',
            _ => !byte.is_ascii_hexdigit(),
        })
    {
        return Err(format!("{context} must be a UUID"));
    }
    Ok(())
}

fn validate_release_tag(tag: &str) -> Result<ReleaseStage, String> {
    let expected = format!("v{}", env!("CARGO_PKG_VERSION"));
    if tag != expected {
        return Err(format!(
            "release tag `{tag}` does not match workspace version `{expected}`"
        ));
    }
    let version = Version::parse(tag.trim_start_matches('v'))
        .map_err(|error| format!("release tag is not valid SemVer: {error}"))?;
    let stage = ReleaseStage::from_version(&version)?;
    println!("release tag: ok ({tag}, stage {})", stage.as_str());
    Ok(stage)
}

fn write_release_sbom(output: &Path) -> Result<(), String> {
    let root = repository_root();
    let metadata_output = Command::new("cargo")
        .current_dir(&root)
        .args(["metadata", "--format-version", "1", "--locked"])
        .output()
        .map_err(|error| format!("failed to run cargo metadata: {error}"))?;
    if !metadata_output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&metadata_output.stderr).trim()
        ));
    }
    let metadata: Value = serde_json::from_slice(&metadata_output.stdout)
        .map_err(|error| format!("cargo metadata returned invalid JSON: {error}"))?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| "cargo metadata has no packages array".to_owned())?;
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or_else(|| "cargo metadata has no resolve nodes".to_owned())?;
    let package_by_id = packages
        .iter()
        .filter_map(|package| package["id"].as_str().map(|id| (id.to_owned(), package)))
        .collect::<BTreeMap<_, _>>();
    let node_by_id = nodes
        .iter()
        .filter_map(|node| node["id"].as_str().map(|id| (id.to_owned(), node)))
        .collect::<BTreeMap<_, _>>();
    let root_id = packages
        .iter()
        .find(|package| package["name"] == "canisend-cli")
        .and_then(|package| package["id"].as_str())
        .ok_or_else(|| "cargo metadata does not contain canisend-cli".to_owned())?
        .to_owned();
    let mut included = BTreeSet::new();
    let mut queue = VecDeque::from([root_id.clone()]);
    while let Some(id) = queue.pop_front() {
        if !included.insert(id.clone()) {
            continue;
        }
        let node = node_by_id
            .get(&id)
            .ok_or_else(|| format!("cargo metadata resolve node is missing for `{id}`"))?;
        let dependencies = node["dependencies"]
            .as_array()
            .ok_or_else(|| format!("cargo metadata dependencies are missing for `{id}`"))?;
        for dependency in dependencies {
            let dependency = dependency
                .as_str()
                .ok_or_else(|| format!("cargo metadata dependency is not a string for `{id}`"))?;
            queue.push_back(dependency.to_owned());
        }
    }
    let root_package = package_by_id
        .get(&root_id)
        .ok_or_else(|| "canisend-cli package metadata is missing".to_owned())?;
    let root_ref = cargo_bom_ref(root_package)?;
    let mut components = included
        .iter()
        .filter(|id| *id != &root_id)
        .map(|id| {
            package_by_id
                .get(id)
                .ok_or_else(|| format!("cargo package metadata is missing for `{id}`"))
                .and_then(|package| cargo_component(package, "library"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    components.sort_by(|left, right| left["bom-ref"].as_str().cmp(&right["bom-ref"].as_str()));
    let mut dependencies = Vec::with_capacity(included.len());
    for id in &included {
        let package = package_by_id
            .get(id)
            .ok_or_else(|| format!("cargo package metadata is missing for `{id}`"))?;
        let node = node_by_id
            .get(id)
            .ok_or_else(|| format!("cargo resolve node is missing for `{id}`"))?;
        let mut depends_on = node["dependencies"]
            .as_array()
            .ok_or_else(|| format!("cargo dependencies are missing for `{id}`"))?
            .iter()
            .filter_map(Value::as_str)
            .filter(|dependency| included.contains(*dependency))
            .map(|dependency| {
                package_by_id
                    .get(dependency)
                    .ok_or_else(|| format!("cargo package metadata is missing for `{dependency}`"))
                    .and_then(|package| cargo_bom_ref(package))
            })
            .collect::<Result<Vec<_>, _>>()?;
        depends_on.sort();
        dependencies.push(json!({
            "ref": cargo_bom_ref(package)?,
            "dependsOn": depends_on,
        }));
    }
    dependencies.sort_by(|left, right| left["ref"].as_str().cmp(&right["ref"].as_str()));
    let sbom = json!({
        "$schema": "https://cyclonedx.org/schema/bom-1.6.schema.json",
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "version": 1,
        "metadata": {
            "component": cargo_component(root_package, "application")?,
            "tools": {
                "components": [{
                    "type": "application",
                    "name": "canisend-xtask",
                    "version": env!("CARGO_PKG_VERSION")
                }]
            },
            "properties": [
                {"name": "canisend:agent_protocol", "value": AGENT_PROTOCOL},
                {"name": "canisend:workspace_format", "value": WORKSPACE_FORMAT},
                {"name": "canisend:schema_version", "value": PUBLIC_SCHEMA_VERSION}
            ]
        },
        "components": components,
        "dependencies": dependencies,
        "compositions": [{
            "aggregate": "complete",
            "assemblies": [root_ref]
        }]
    });
    write_pretty_json(output, &sbom)?;
    println!(
        "release SBOM: wrote {} components to {}",
        included.len(),
        output.display()
    );
    Ok(())
}

fn cargo_component(package: &Value, component_type: &str) -> Result<Value, String> {
    let name = required_json_string(package, "name")?;
    let version = required_json_string(package, "version")?;
    let mut component = Map::from_iter([
        ("type".to_owned(), Value::String(component_type.to_owned())),
        ("bom-ref".to_owned(), Value::String(cargo_bom_ref(package)?)),
        ("name".to_owned(), Value::String(name.to_owned())),
        ("version".to_owned(), Value::String(version.to_owned())),
        (
            "purl".to_owned(),
            Value::String(format!("pkg:cargo/{name}@{version}")),
        ),
    ]);
    if let Some(license) = package["license"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        component.insert(
            "licenses".to_owned(),
            json!([{"license": {"name": license}}]),
        );
    }
    if let Some(repository) = package["repository"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        component.insert(
            "externalReferences".to_owned(),
            json!([{"type": "vcs", "url": repository}]),
        );
    }
    if let Some(checksum) = package["checksum"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        component.insert(
            "hashes".to_owned(),
            json!([{"alg": "SHA-256", "content": checksum}]),
        );
    }
    Ok(Value::Object(component))
}

fn cargo_bom_ref(package: &Value) -> Result<String, String> {
    let id = required_json_string(package, "id")?;
    Ok(format!(
        "urn:canisend:cargo:sha256:{}",
        sha256(id.as_bytes())
    ))
}

fn required_json_string<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value[name]
        .as_str()
        .filter(|field| !field.is_empty())
        .ok_or_else(|| format!("cargo metadata package field `{name}` is missing"))
}

fn assemble_release(
    tag: &str,
    commit: &str,
    artifacts_root: &Path,
    output: &Path,
) -> Result<(), String> {
    let stage = validate_release_tag(tag)?;
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("release commit must be a full 40-character hexadecimal Git commit".to_owned());
    }
    if output.exists() {
        return Err(format!(
            "release output must not already exist: {}",
            output.display()
        ));
    }
    fs::create_dir_all(output).map_err(|error| {
        format!(
            "could not create release output {}: {error}",
            output.display()
        )
    })?;
    let version = env!("CARGO_PKG_VERSION");
    let targets = release_targets()?;
    let mut archive_entries = Vec::with_capacity(targets.len());
    let mut signing_evidence_paths = Vec::new();
    for target in &targets {
        let file_name = format!("canisend-{version}-{}.{}", target.triple, target.archive);
        let source = find_unique_file(artifacts_root, &file_name)?;
        reject_symlink(&source)?;
        let destination = output.join(&file_name);
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "could not copy release archive {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
        let signing_evidence = if !matches!(stage, ReleaseStage::Alpha) && target.signing != "none"
        {
            let evidence_name = format!("canisend-{version}-{}-signing.json", target.triple);
            let evidence_source = find_unique_file(artifacts_root, &evidence_name)?;
            read_bound_signing_evidence(&evidence_source, target, version, &destination)?;
            let evidence_destination = output.join(&evidence_name);
            fs::copy(&evidence_source, &evidence_destination).map_err(|error| {
                format!(
                    "could not copy signing evidence {} to {}: {error}",
                    evidence_source.display(),
                    evidence_destination.display()
                )
            })?;
            signing_evidence_paths.push(evidence_destination);
            Value::String(evidence_name)
        } else {
            Value::Null
        };
        archive_entries.push(json!({
            "archive": file_name,
            "archive_format": target.archive,
            "executable": target.executable,
            "runner": target.runner,
            "sha256": sha256_file(&destination)?,
            "signing_kind": target.signing,
            "size": file_size(&destination)?,
            "signing_evidence": signing_evidence,
            "target": target.triple,
        }));
    }
    let sbom_name = format!("canisend-{version}-sbom.cdx.json");
    let sbom_path = output.join(&sbom_name);
    write_release_sbom(&sbom_path)?;
    let supplemental_sources = [
        ("KNOWN_LIMITATIONS.md", "release/KNOWN_LIMITATIONS.md"),
        ("ISSUE_COLLECTION.md", "release/ISSUE_COLLECTION.md"),
        ("RELEASE_NOTES.md", "release/RELEASE_NOTES.md"),
        ("THIRD_PARTY_NOTICES.md", "THIRD_PARTY_NOTICES.md"),
    ];
    let mut supplemental_entries = vec![release_file_entry(&sbom_path)?];
    for evidence in &signing_evidence_paths {
        supplemental_entries.push(release_file_entry(evidence)?);
    }
    for (name, source) in supplemental_sources {
        let source = repository_root().join(source);
        let destination = output.join(name);
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "could not copy supplemental release file {}: {error}",
                source.display()
            )
        })?;
        supplemental_entries.push(release_file_entry(&destination)?);
    }
    supplemental_entries.sort_by(|left, right| left["file"].as_str().cmp(&right["file"].as_str()));
    let manifest_name = format!("canisend-{version}-manifest.json");
    let manifest_path = output.join(&manifest_name);
    let manifest = json!({
        "schema": RELEASE_MANIFEST_SCHEMA,
        "product": "canisend",
        "version": version,
        "tag": tag,
        "stage": stage.as_str(),
        "source": {
            "commit": commit.to_ascii_lowercase(),
            "locked_dependencies": true,
            "repository": env!("CARGO_PKG_REPOSITORY")
        },
        "contracts": {
            "agent_protocol": AGENT_PROTOCOL,
            "public_schema_version": PUBLIC_SCHEMA_VERSION,
            "resource_format": canisend_resources::RESOURCE_VERSION,
            "workspace_format": WORKSPACE_FORMAT
        },
        "artifacts": archive_entries,
        "supplemental_files": supplemental_entries,
        "trust": {
            "archive_code_signing_required": !matches!(stage, ReleaseStage::Alpha),
            "default_telemetry": false,
            "manifest_attestation": "GitHub OIDC artifact attestation",
            "verification_command": format!(
                "gh attestation verify {manifest_name} --repo {}",
                env!("CARGO_PKG_REPOSITORY").trim_start_matches("https://github.com/")
            )
        }
    });
    write_pretty_json(&manifest_path, &manifest)?;
    write_checksums(output)?;
    verify_release(tag, output)?;
    println!("release assets: assembled {}", output.display());
    Ok(())
}

fn verify_release(tag: &str, directory: &Path) -> Result<(), String> {
    let stage = validate_release_tag(tag)?;
    let version = env!("CARGO_PKG_VERSION");
    let manifest_path = directory.join(format!("canisend-{version}-manifest.json"));
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| {
        format!(
            "release manifest is missing at {}: {error}",
            manifest_path.display()
        )
    })?)
    .map_err(|error| format!("release manifest is invalid JSON: {error}"))?;
    if manifest["schema"] != RELEASE_MANIFEST_SCHEMA
        || manifest["version"] != version
        || manifest["tag"] != tag
        || manifest["stage"] != stage.as_str()
    {
        return Err("release manifest identity does not match this build".to_owned());
    }
    verify_release_manifest_contents(stage, version, directory, &manifest)?;
    let checksums_path = directory.join("SHA256SUMS");
    let checksums = fs::read_to_string(&checksums_path).map_err(|error| {
        format!(
            "SHA256SUMS is missing at {}: {error}",
            checksums_path.display()
        )
    })?;
    let mut verified = BTreeSet::new();
    for (line_number, line) in checksums.lines().enumerate() {
        let (expected, file_name) = line
            .split_once("  ")
            .ok_or_else(|| format!("invalid SHA256SUMS line {}: `{line}`", line_number + 1))?;
        if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "invalid SHA-256 at SHA256SUMS line {}",
                line_number + 1
            ));
        }
        if file_name.is_empty()
            || file_name.contains('/')
            || file_name.contains('\\')
            || file_name == "SHA256SUMS"
        {
            return Err(format!("unsafe checksum file name `{file_name}`"));
        }
        if !verified.insert(file_name.to_owned()) {
            return Err(format!("duplicate checksum entry `{file_name}`"));
        }
        let actual = sha256_file(&directory.join(file_name))?;
        if actual != expected.to_ascii_lowercase() {
            return Err(format!("checksum mismatch for `{file_name}`"));
        }
    }
    let actual_files = fs::read_dir(directory)
        .map_err(|error| format!("could not inspect release directory: {error}"))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name() != "SHA256SUMS")
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>();
    if verified != actual_files {
        return Err(format!(
            "checksum coverage differs: verified {verified:?}, files {actual_files:?}"
        ));
    }
    println!("release assets: verified {} files", verified.len());
    Ok(())
}

fn verify_release_manifest_contents(
    stage: ReleaseStage,
    version: &str,
    directory: &Path,
    manifest: &Value,
) -> Result<(), String> {
    if manifest["product"] != "canisend"
        || manifest["source"]["locked_dependencies"] != true
        || manifest["source"]["repository"] != env!("CARGO_PKG_REPOSITORY")
        || manifest["contracts"]["agent_protocol"] != AGENT_PROTOCOL
        || manifest["contracts"]["public_schema_version"] != PUBLIC_SCHEMA_VERSION
        || manifest["contracts"]["resource_format"] != canisend_resources::RESOURCE_VERSION
        || manifest["contracts"]["workspace_format"] != WORKSPACE_FORMAT
        || manifest["trust"]["default_telemetry"] != false
        || manifest["trust"]["archive_code_signing_required"]
            != !matches!(stage, ReleaseStage::Alpha)
    {
        return Err("release manifest policy or contract metadata is invalid".to_owned());
    }
    let commit = required_string(&manifest["source"], "commit", "release source")?;
    validate_lower_hex("release source commit", commit, 40)?;

    let targets = release_targets()?;
    let entries = manifest["artifacts"]
        .as_array()
        .ok_or_else(|| "release manifest artifacts are missing".to_owned())?;
    if entries.len() != targets.len() {
        return Err(format!(
            "release manifest must contain exactly {} artifacts",
            targets.len()
        ));
    }
    let mut by_target = BTreeMap::new();
    for entry in entries {
        let target = required_string(entry, "target", "release artifact")?;
        if by_target.insert(target, entry).is_some() {
            return Err(format!("duplicate release manifest target `{target}`"));
        }
    }
    for target in targets {
        let entry = by_target
            .get(target.triple.as_str())
            .ok_or_else(|| format!("release manifest target `{}` is missing", target.triple))?;
        let file_name = format!("canisend-{version}-{}.{}", target.triple, target.archive);
        if entry["archive"] != file_name
            || entry["archive_format"] != target.archive
            || entry["executable"] != target.executable
            || entry["runner"] != target.runner
            || entry["signing_kind"] != target.signing
        {
            return Err(format!(
                "release manifest metadata is invalid for target `{}`",
                target.triple
            ));
        }
        let signing_evidence_name =
            if !matches!(stage, ReleaseStage::Alpha) && target.signing != "none" {
                Some(format!("canisend-{version}-{}-signing.json", target.triple))
            } else {
                None
            };
        match signing_evidence_name {
            Some(ref evidence_name) if entry["signing_evidence"] == *evidence_name => {
                read_bound_signing_evidence(
                    &directory.join(evidence_name),
                    &target,
                    version,
                    &directory.join(&file_name),
                )?;
            }
            None if entry["signing_evidence"].is_null() => {}
            _ => {
                return Err(format!(
                    "release signing evidence reference is invalid for `{}`",
                    target.triple
                ));
            }
        }
        let declared_sha = required_string(entry, "sha256", "release artifact")?;
        validate_lower_hex(
            &format!("release artifact `{}` SHA-256", target.triple),
            declared_sha,
            64,
        )?;
        let declared_size = entry["size"]
            .as_u64()
            .filter(|size| *size > 0)
            .ok_or_else(|| format!("release artifact `{}` has no positive size", target.triple))?;
        let path = directory.join(&file_name);
        reject_symlink(&path)?;
        if sha256_file(&path)? != declared_sha || file_size(&path)? != declared_size {
            return Err(format!(
                "release manifest digest or size does not match `{file_name}`"
            ));
        }
    }

    let mut expected_supplemental = BTreeSet::from([
        "ISSUE_COLLECTION.md".to_owned(),
        "KNOWN_LIMITATIONS.md".to_owned(),
        "RELEASE_NOTES.md".to_owned(),
        "THIRD_PARTY_NOTICES.md".to_owned(),
        format!("canisend-{version}-sbom.cdx.json"),
    ]);
    if !matches!(stage, ReleaseStage::Alpha) {
        for target in release_targets()?
            .into_iter()
            .filter(|target| target.signing != "none")
        {
            expected_supplemental
                .insert(format!("canisend-{version}-{}-signing.json", target.triple));
        }
    }
    let supplemental = manifest["supplemental_files"]
        .as_array()
        .ok_or_else(|| "release manifest supplemental files are missing".to_owned())?;
    if supplemental.len() != expected_supplemental.len() {
        return Err("release manifest supplemental file count is invalid".to_owned());
    }
    let mut actual_supplemental = BTreeSet::new();
    for entry in supplemental {
        let file = required_string(entry, "file", "supplemental release file")?;
        if file.contains('/') || file.contains('\\') || !actual_supplemental.insert(file.to_owned())
        {
            return Err(format!(
                "unsafe or duplicate supplemental release file `{file}`"
            ));
        }
        let declared_sha = required_string(entry, "sha256", "supplemental release file")?;
        validate_lower_hex(
            &format!("supplemental release file `{file}` SHA-256"),
            declared_sha,
            64,
        )?;
        let declared_size = entry["size"]
            .as_u64()
            .filter(|size| *size > 0)
            .ok_or_else(|| format!("supplemental release file `{file}` has no positive size"))?;
        let path = directory.join(file);
        reject_symlink(&path)?;
        if sha256_file(&path)? != declared_sha || file_size(&path)? != declared_size {
            return Err(format!(
                "release manifest digest or size does not match supplemental file `{file}`"
            ));
        }
    }
    if actual_supplemental != expected_supplemental {
        return Err("release manifest supplemental file set is invalid".to_owned());
    }
    Ok(())
}

fn write_checksums(directory: &Path) -> Result<(), String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("could not inspect release output: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("could not inspect release output entry: {error}"))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    let mut body = String::new();
    for entry in entries {
        let path = entry.path();
        if !path.is_file() || entry.file_name() == "SHA256SUMS" {
            continue;
        }
        reject_symlink(&path)?;
        body.push_str(&format!(
            "{}  {}\n",
            sha256_file(&path)?,
            entry.file_name().to_string_lossy()
        ));
    }
    fs::write(directory.join("SHA256SUMS"), body)
        .map_err(|error| format!("could not write SHA256SUMS: {error}"))
}

fn release_file_entry(path: &Path) -> Result<Value, String> {
    Ok(json!({
        "file": path
            .file_name()
            .ok_or_else(|| format!("release file has no name: {}", path.display()))?
            .to_string_lossy(),
        "sha256": sha256_file(path)?,
        "size": file_size(path)?,
    }))
}

fn find_unique_file(root: &Path, file_name: &str) -> Result<PathBuf, String> {
    let mut matches = Vec::new();
    collect_named_files(root, file_name, &mut matches)?;
    match matches.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err(format!(
            "release archive `{file_name}` was not found under {}",
            root.display()
        )),
        _ => Err(format!(
            "release archive `{file_name}` appears more than once under {}",
            root.display()
        )),
    }
}

fn collect_named_files(
    root: &Path,
    file_name: &str,
    matches: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for entry in fs::read_dir(root)
        .map_err(|error| format!("could not inspect {}: {error}", root.display()))?
    {
        let entry =
            entry.map_err(|error| format!("could not inspect release artifact: {error}"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "release artifact tree contains a symlink: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            collect_named_files(&path, file_name, matches)?;
        } else if entry.file_name() == file_name {
            matches.push(path);
        }
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "release input is not a regular file: {}",
            path.display()
        ));
    }
    Ok(())
}

fn file_size(path: &Path) -> Result<u64, String> {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|error| format!("could not inspect {}: {error}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("could not read {} for hashing: {error}", path.display()))?;
    Ok(sha256(&bytes))
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn write_pretty_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let mut file = fs::File::create(path)
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, value)
        .map_err(|error| format!("could not serialize {}: {error}", path.display()))?;
    file.write_all(b"\n")
        .map_err(|error| format!("could not finish {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_channel_source() -> ChannelCandidateSource {
        channel_candidate_source_from_value(&json!({
            "schema": CHANNEL_CANDIDATE_SOURCE_SCHEMA,
            "candidate_only": true,
            "publication_authorized": false,
            "release": {
                "tag": NATIVE_ALPHA_TAG,
                "version": "0.7.0-alpha.1",
                "stage": "alpha",
                "source_commit": NATIVE_ALPHA_SOURCE,
                "repository": env!("CARGO_PKG_REPOSITORY"),
                "manifest_file": "canisend-0.7.0-alpha.1-manifest.json",
                "manifest_sha256": "1111111111111111111111111111111111111111111111111111111111111111"
            },
            "artifacts": [
                {
                    "target": "aarch64-apple-darwin",
                    "archive": "canisend-0.7.0-alpha.1-aarch64-apple-darwin.tar.gz",
                    "sha256": "2222222222222222222222222222222222222222222222222222222222222222",
                    "size": 1
                },
                {
                    "target": "x86_64-apple-darwin",
                    "archive": "canisend-0.7.0-alpha.1-x86_64-apple-darwin.tar.gz",
                    "sha256": "3333333333333333333333333333333333333333333333333333333333333333",
                    "size": 2
                },
                {
                    "target": "x86_64-pc-windows-msvc",
                    "archive": "canisend-0.7.0-alpha.1-x86_64-pc-windows-msvc.zip",
                    "sha256": "4444444444444444444444444444444444444444444444444444444444444444",
                    "size": 3
                }
            ]
        }))
        .expect("sample channel source")
    }

    fn sample_package_manager_evidence(
        record: &str,
        channel: &str,
        target: &str,
        environment: &str,
    ) -> Value {
        json!({
            "schema": PACKAGE_MANAGER_QUALIFICATION_SCHEMA,
            "record": record,
            "channel": channel,
            "target": target,
            "environment": environment,
            "from_tag": "v0.7.0-beta.1",
            "to_tag": "v0.7.0-rc.1",
            "from_candidate_source_sha256": "a".repeat(64),
            "to_candidate_source_sha256": "b".repeat(64),
            "github_run_id": 29_640_000_001_u64,
            "tool_version": "native package tool 1.0.0",
            "observed_versions": {
                "from": "0.7.0-beta.1",
                "to": "0.7.0-rc.1"
            },
            "checks": {
                "candidate-sources-verified": true,
                "official-validation": true,
                "install": true,
                "from-version": true,
                "from-doctor": true,
                "workspace-created": true,
                "upgrade": true,
                "to-version": true,
                "to-doctor": true,
                "uninstall": true,
                "workspace-retained": true,
                "no-publication": true
            },
            "completed_at": "2026-07-18T10:00:00Z"
        })
    }

    fn sample_apple_signing_evidence() -> Value {
        json!({
            "schema": CODE_SIGNING_EVIDENCE_SCHEMA,
            "version": env!("CARGO_PKG_VERSION"),
            "target": "aarch64-apple-darwin",
            "kind": "apple-developer-id-notarization",
            "status": "verified",
            "binary": {
                "file": "canisend",
                "sha256": "5555555555555555555555555555555555555555555555555555555555555555",
                "size": 42
            },
            "archive": null,
            "signer": {
                "identity": "Developer ID Application: CanISend Test (ABCDE12345)",
                "team_id": "ABCDE12345",
                "code_identifier": "io.github.jxpeng98.canisend"
            },
            "verification": {
                "developer_id": true,
                "hardened_runtime": true,
                "secure_timestamp": true,
                "notarization_status": "Accepted",
                "notary_submission_id": "12345678-1234-1234-1234-123456789abc",
                "notary_log_sha256": "6666666666666666666666666666666666666666666666666666666666666666",
                "notary_error_count": 0,
                "notary_warning_count": 0,
                "standalone_ticket_stapled": false,
                "stapling_supported": false
            }
        })
    }

    fn sample_windows_signing_evidence() -> Value {
        json!({
            "schema": CODE_SIGNING_EVIDENCE_SCHEMA,
            "version": env!("CARGO_PKG_VERSION"),
            "target": "x86_64-pc-windows-msvc",
            "kind": "windows-authenticode-artifact-signing",
            "status": "verified",
            "binary": {
                "file": "canisend.exe",
                "sha256": "7777777777777777777777777777777777777777777777777777777777777777",
                "size": 84
            },
            "archive": null,
            "signer": {
                "identity": "CN=CanISend Test",
                "thumbprint": "8888888888888888888888888888888888888888"
            },
            "verification": {
                "authenticode_status": "Valid",
                "file_digest": "SHA256",
                "timestamp_digest": "SHA256",
                "timestamp_present": true,
                "timestamp_identity": "CN=Microsoft Public RSA Time Stamping Authority",
                "service": "azure-artifact-signing"
            }
        })
    }

    #[test]
    fn workspace_version_maps_to_exact_alpha_tag() {
        assert_eq!(
            validate_release_tag("v0.7.0-alpha.1"),
            Ok(ReleaseStage::Alpha)
        );
        assert!(validate_release_tag("v0.7.0-alpha.2").is_err());
        assert!(validate_release_tag("0.7.0-alpha.1").is_err());
    }

    #[test]
    fn release_contract_has_five_unique_targets() {
        let targets = release_targets().expect("release targets");
        assert_eq!(targets.len(), 5);
        assert_eq!(
            targets
                .iter()
                .map(|target| target.triple.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            5
        );
    }

    #[test]
    fn frozen_contract_digest_is_checkout_line_ending_independent() {
        let lf = canonicalize_frozen_contract_text(b"{\n  \"schema\": 2\n}\n")
            .expect("LF contract text");
        let crlf = canonicalize_frozen_contract_text(b"{\r\n  \"schema\": 2\r\n}\r\n")
            .expect("CRLF contract text");
        assert_eq!(lf, crlf);
        assert_eq!(
            digest_named_bytes(&[("schema.json".to_owned(), lf)]),
            digest_named_bytes(&[("schema.json".to_owned(), crlf)])
        );
        assert!(canonicalize_frozen_contract_text(b"invalid\rtext").is_err());
    }

    #[test]
    fn internal_path_dependencies_are_exactly_versioned() {
        check_internal_dependency_versions().expect("internal dependency versions");
    }

    #[test]
    fn beta_readiness_has_no_unresolved_alpha_blockers() {
        check_beta_readiness().expect("Beta readiness ledger");
    }

    #[test]
    fn beta_agent_and_workspace_contracts_match_freeze() {
        check_beta_contract_freeze().expect("Beta contract freeze");
    }

    #[test]
    fn signing_policy_matches_fail_closed_workflow_contract() {
        check_signing_policy().expect("signing policy");
    }

    #[test]
    fn support_policy_matches_current_contracts_and_release_line() {
        check_support_policy().expect("support policy");
    }

    #[test]
    fn support_policy_cannot_remain_draft_for_stable_version() {
        let prerelease = Version::parse("0.7.0-rc.1").expect("RC version");
        let stable = Version::parse("0.7.0").expect("Stable version");
        assert_eq!(
            support_policy_publication_status(&prerelease),
            "pre-stable-draft"
        );
        assert_eq!(support_policy_publication_status(&stable), "published");
    }

    #[test]
    fn stable_requires_rc_feedback_and_published_next_roadmap() {
        let prerelease = Version::parse("0.7.0-rc.1").expect("RC version");
        let stable = Version::parse("0.7.0").expect("Stable version");
        assert_eq!(
            feedback_publication_requirements(&prerelease),
            (None, "draft")
        );
        assert_eq!(
            feedback_publication_requirements(&stable),
            (Some("rc"), "published")
        );
    }

    #[test]
    fn release_stage_requires_progressive_qualification_status() {
        assert_eq!(
            qualification_status_for_stage(ReleaseStage::Alpha),
            "pre-beta"
        );
        assert_eq!(
            qualification_status_for_stage(ReleaseStage::Beta),
            "beta-qualifying"
        );
        assert_eq!(
            qualification_status_for_stage(ReleaseStage::ReleaseCandidate),
            "rc-qualifying"
        );
        assert_eq!(
            qualification_status_for_stage(ReleaseStage::Stable),
            "qualified"
        );
    }

    #[test]
    fn native_documentation_preparation_requires_exact_run_evidence() {
        let missing_run = json!({
            "status": "prepared-native",
            "native_matrix_run": null,
            "evidence": ["five-target lifecycle smoke passed"]
        });
        assert!(validate_documentation_uninstall_progress(&missing_run).is_err());

        let qualified = json!({
            "status": "prepared-native",
            "native_matrix_run": 29_637_471_699_u64,
            "evidence": ["five-target lifecycle smoke passed"]
        });
        validate_documentation_uninstall_progress(&qualified).expect("native preparation evidence");
    }

    #[test]
    fn scheduled_fuzz_policy_is_pinned_and_complete() {
        check_fuzz_policy().expect("scheduled fuzz policy");
    }

    #[test]
    fn generated_property_test_policy_is_distinct_and_pinned() {
        check_property_test_policy().expect("generated property-test policy");
    }

    #[test]
    fn channel_candidates_preserve_archives_and_nested_binary_paths() {
        let files = render_channel_candidates(&sample_channel_source()).expect("render candidates");
        let homebrew = &files["homebrew/Casks/canisend.rb"];
        assert!(homebrew.contains("arch arm: \"aarch64\", intel: \"x86_64\""));
        assert!(homebrew.contains("sha256 arm:"));
        assert!(homebrew.contains("binary \"canisend-#{version}-#{arch}-apple-darwin/canisend\""));
        let scoop: Value =
            serde_json::from_str(&files["scoop/bucket/canisend.json"]).expect("valid Scoop JSON");
        assert_eq!(
            scoop["architecture"]["64bit"]["hash"],
            "4444444444444444444444444444444444444444444444444444444444444444"
        );
        assert_eq!(
            scoop["extract_dir"],
            "canisend-0.7.0-alpha.1-x86_64-pc-windows-msvc"
        );
        let installer = files
            .iter()
            .find(|(path, _)| path.ends_with(".installer.yaml"))
            .map(|(_, body)| body)
            .expect("WinGet installer candidate");
        assert!(installer.contains("  PortableCommandAlias: canisend\n"));
        assert!(installer.contains("  InstallerUrl: https://"));
        assert!(installer.contains("canisend-0.7.0-alpha.1-x86_64-pc-windows-msvc\\canisend.exe"));
    }

    #[test]
    fn package_manager_qualification_policy_is_native_and_nonpublishing() {
        check_package_manager_qualification_policy().expect("package-manager qualification policy");
    }

    #[test]
    fn package_manager_evidence_requires_all_true_canonical_checks() {
        let mut evidence = sample_package_manager_evidence(
            "homebrew-aarch64-apple-darwin",
            "homebrew-cask",
            "aarch64-apple-darwin",
            "macos-15",
        );
        validate_package_manager_evidence_record(
            &evidence,
            "homebrew-aarch64-apple-darwin",
            "homebrew-cask",
            "aarch64-apple-darwin",
            "macos-15",
            "v0.7.0-beta.1",
            "v0.7.0-rc.1",
        )
        .expect("canonical package-manager evidence");

        evidence["checks"]["upgrade"] = Value::Bool(false);
        assert!(
            validate_package_manager_evidence_record(
                &evidence,
                "homebrew-aarch64-apple-darwin",
                "homebrew-cask",
                "aarch64-apple-darwin",
                "macos-15",
                "v0.7.0-beta.1",
                "v0.7.0-rc.1",
            )
            .is_err()
        );
    }

    #[test]
    fn package_manager_evidence_directory_binds_one_native_run() {
        let root = std::env::temp_dir().join(format!(
            "canisend-package-manager-evidence-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale package evidence fixture");
        }
        fs::create_dir_all(&root).expect("create package evidence fixture");
        for (file, record, channel, target, environment) in [
            (
                "homebrew-aarch64-apple-darwin.json",
                "homebrew-aarch64-apple-darwin",
                "homebrew-cask",
                "aarch64-apple-darwin",
                "macos-15",
            ),
            (
                "homebrew-x86_64-apple-darwin.json",
                "homebrew-x86_64-apple-darwin",
                "homebrew-cask",
                "x86_64-apple-darwin",
                "macos-15-intel",
            ),
            (
                "scoop-x86_64-pc-windows-msvc.json",
                "scoop-x86_64-pc-windows-msvc",
                "scoop",
                "x86_64-pc-windows-msvc",
                "windows-2025",
            ),
            (
                "winget-x86_64-pc-windows-msvc.json",
                "winget-x86_64-pc-windows-msvc",
                "winget",
                "x86_64-pc-windows-msvc",
                "windows-sandbox",
            ),
        ] {
            write_pretty_json(
                &root.join(file),
                &sample_package_manager_evidence(record, channel, target, environment),
            )
            .expect("write package evidence fixture");
        }
        verify_package_manager_evidence("v0.7.0-beta.1", "v0.7.0-rc.1", &root)
            .expect("verify complete package evidence");
        assert!(verify_package_manager_evidence("v0.7.0-alpha.1", "v0.7.0-rc.1", &root).is_err());
        fs::remove_dir_all(root).expect("remove package evidence fixture");
    }

    #[test]
    fn channel_candidate_source_cannot_authorize_publication() {
        let mut value = sample_channel_source().to_value();
        value["publication_authorized"] = Value::Bool(true);
        assert!(channel_candidate_source_from_value(&value).is_err());
    }

    #[test]
    fn signing_evidence_binds_exact_final_archive() {
        let root = std::env::temp_dir().join(format!(
            "canisend-xtask-signing-evidence-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale signing fixture");
        }
        fs::create_dir_all(&root).expect("create signing fixture");
        let binary = root.join("canisend");
        fs::write(&binary, b"signed binary fixture").expect("write signed binary fixture");
        let evidence = root.join("evidence.json");
        let mut signing_evidence = sample_apple_signing_evidence();
        signing_evidence["binary"]["sha256"] =
            Value::String(sha256_file(&binary).expect("binary fixture hash"));
        signing_evidence["binary"]["size"] =
            Value::Number(file_size(&binary).expect("binary fixture size").into());
        write_pretty_json(&evidence, &signing_evidence).expect("write signing fixture");
        let archive = root.join(format!(
            "canisend-{}-aarch64-apple-darwin.tar.gz",
            env!("CARGO_PKG_VERSION")
        ));
        fs::write(&archive, b"signed archive fixture").expect("write archive fixture");
        bind_signing_evidence(
            &format!("v{}", env!("CARGO_PKG_VERSION")),
            "aarch64-apple-darwin",
            &evidence,
            &binary,
            &archive,
        )
        .expect("bind signing evidence");
        let bound: Value =
            serde_json::from_slice(&fs::read(&evidence).expect("read bound evidence"))
                .expect("parse bound evidence");
        assert_eq!(
            bound["archive"]["sha256"],
            sha256_file(&archive).expect("archive hash")
        );
        let target = release_targets()
            .expect("release targets")
            .into_iter()
            .find(|target| target.triple == "aarch64-apple-darwin")
            .expect("Apple target");
        read_bound_signing_evidence(&evidence, &target, env!("CARGO_PKG_VERSION"), &archive)
            .expect("verify bound evidence");
        fs::remove_dir_all(root).expect("remove signing fixture");
    }

    #[test]
    fn signing_evidence_rejects_signed_binary_mismatch() {
        let root = std::env::temp_dir().join(format!(
            "canisend-xtask-signing-binary-mismatch-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale signing fixture");
        }
        fs::create_dir_all(&root).expect("create signing fixture");
        let binary = root.join("canisend");
        fs::write(&binary, b"different signed binary").expect("write signed binary fixture");
        let evidence = root.join("evidence.json");
        write_pretty_json(&evidence, &sample_apple_signing_evidence())
            .expect("write signing fixture");
        let archive = root.join(format!(
            "canisend-{}-aarch64-apple-darwin.tar.gz",
            env!("CARGO_PKG_VERSION")
        ));
        fs::write(&archive, b"signed archive fixture").expect("write archive fixture");
        assert!(
            bind_signing_evidence(
                &format!("v{}", env!("CARGO_PKG_VERSION")),
                "aarch64-apple-darwin",
                &evidence,
                &binary,
                &archive,
            )
            .is_err()
        );
        fs::remove_dir_all(root).expect("remove signing fixture");
    }

    #[test]
    fn signing_evidence_rejects_missing_windows_timestamp() {
        let target = release_targets()
            .expect("release targets")
            .into_iter()
            .find(|target| target.triple == "x86_64-pc-windows-msvc")
            .expect("Windows target");
        let mut evidence = sample_windows_signing_evidence();
        evidence["verification"]["timestamp_present"] = Value::Bool(false);
        assert!(
            canonical_signing_evidence(&evidence, &target, env!("CARGO_PKG_VERSION"), None)
                .is_err()
        );
    }
}
