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
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

const RELEASE_TARGET_SCHEMA: &str = "canisend.release-targets/v1";
const RELEASE_MANIFEST_SCHEMA: &str = "canisend.release-manifest/v1";
const BETA_READINESS_SCHEMA: &str = "canisend.beta-readiness/v1";
const BETA_CONTRACT_FREEZE_SCHEMA: &str = "canisend.beta-contract-freeze/v1";
const CHANNEL_CANDIDATE_SOURCE_SCHEMA: &str = "canisend.channel-candidate-source/v1";
const SIGNING_POLICY_SCHEMA: &str = "canisend.signing-policy/v1";
const SUPPORT_POLICY_SCHEMA: &str = "canisend.support-policy/v1";
const FEEDBACK_SNAPSHOT_SCHEMA: &str = "canisend.feedback-snapshot/v1";
const RELEASE_QUALIFICATION_SCHEMA: &str = "canisend.release-qualification/v1";
const STAGE_TRANSITION_POLICY_SCHEMA: &str = "canisend.stage-transition-policy/v1";
const STAGE_TRANSITION_PLAN_SCHEMA: &str = "canisend.stage-transition-plan/v1";
const FEATURE_FREEZE_PLAN_SCHEMA: &str = "canisend.feature-freeze-plan/v1";
const BETA_QUALIFICATION_PLAN_SCHEMA: &str = "canisend.beta-qualification-plan/v1";
const RC_QUALIFICATION_PLAN_SCHEMA: &str = "canisend.rc-qualification-plan/v1";
const FEATURE_FREEZE_EXCEPTIONS_SCHEMA: &str = "canisend.feature-freeze-exceptions/v1";
const PACKAGE_MANAGER_QUALIFICATION_POLICY_SCHEMA: &str =
    "canisend.package-manager-qualification-policy/v1";
const PACKAGE_MANAGER_QUALIFICATION_SCHEMA: &str = "canisend.package-manager-qualification/v1";
const UPGRADE_QUALIFICATION_POLICY_SCHEMA: &str = "canisend.upgrade-qualification-policy/v1";
const UPGRADE_QUALIFICATION_SCHEMA: &str = "canisend.upgrade-qualification/v1";
const UPGRADE_QUALIFICATION_PLAN_SCHEMA: &str = "canisend.upgrade-qualification-plan/v1";
const DOCUMENTATION_UNINSTALL_POLICY_SCHEMA: &str = "canisend.documentation-uninstall-policy/v1";
const DOCUMENTATION_UNINSTALL_SCHEMA: &str = "canisend.documentation-uninstall/v1";
const DOCUMENTATION_UNINSTALL_PLAN_SCHEMA: &str = "canisend.documentation-uninstall-plan/v1";
const RELEASE_NOTES_POLICY_SCHEMA: &str = "canisend.release-notes-policy/v1";
const CODE_SIGNING_EVIDENCE_SCHEMA: &str = "canisend.code-signing-evidence/v1";
const FUZZ_TOOLCHAIN: &str = "nightly-2026-07-01";
const CARGO_FUZZ_VERSION: &str = "0.13.2";
const WINGET_MANIFEST_VERSION: &str = "1.12.0";
const BETA_READINESS_MAX_AGE_HOURS: i64 = 24;
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
            check_release_notes_policy()?;
            check_property_test_policy()?;
            check_fuzz_policy()?;
            check_internal_dependency_versions()?;
            check_beta_readiness()?;
            check_beta_contract_freeze()?;
            check_channel_candidates()?;
            check_package_manager_qualification_policy()?;
            check_upgrade_qualification_policy()?;
            check_documentation_uninstall_policy()?;
            check_signing_policy()?;
            check_stage_transition_policy()?;
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
        [area, command, path] if area == "release" && command == "verify-beta-readiness" => {
            check_beta_readiness_file(Path::new(path))
        }
        [area, command, tag] if area == "release" && command == "prepare-stage" => {
            prepare_stage_transition(tag, false)
        }
        [area, command, tag, write]
            if area == "release" && command == "prepare-stage" && write == "--write" =>
        {
            prepare_stage_transition(tag, true)
        }
        [area, command, baseline]
            if area == "release" && command == "activate-feature-freeze" =>
        {
            activate_feature_freeze(baseline, false)
        }
        [area, command, baseline, write]
            if area == "release" && command == "activate-feature-freeze" && write == "--write" =>
        {
            activate_feature_freeze(baseline, true)
        }
        [area, command, tag, run_id, assets]
            if area == "release" && command == "record-beta-qualification" =>
        {
            record_beta_qualification(tag, run_id, Path::new(assets), false)
        }
        [area, command, tag, run_id, assets, write]
            if area == "release"
                && command == "record-beta-qualification"
                && write == "--write" =>
        {
            record_beta_qualification(tag, run_id, Path::new(assets), true)
        }
        [area, command, tag, run_id, assets]
            if area == "release" && command == "record-rc-qualification" =>
        {
            record_rc_qualification(tag, run_id, Path::new(assets), false)
        }
        [area, command, tag, run_id, assets, write]
            if area == "release"
                && command == "record-rc-qualification"
                && write == "--write" =>
        {
            record_rc_qualification(tag, run_id, Path::new(assets), true)
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
        [area, command, from_tag, to_tag, evidence]
            if area == "release" && command == "verify-upgrade-evidence" =>
        {
            verify_upgrade_qualification_evidence(from_tag, to_tag, Path::new(evidence))
                .map(|_| ())
        }
        [area, command, tag, assets, evidence]
            if area == "release" && command == "verify-documentation-evidence" =>
        {
            verify_documentation_uninstall_evidence(
                tag,
                Path::new(assets),
                Path::new(evidence),
            )
            .map(|_| ())
        }
        [area, command, tag, assets, evidence]
            if area == "release" && command == "record-documentation-qualification" =>
        {
            record_documentation_uninstall_qualification(
                tag,
                Path::new(assets),
                Path::new(evidence),
                false,
            )
        }
        [area, command, tag, assets, evidence, write]
            if area == "release"
                && command == "record-documentation-qualification"
                && write == "--write" =>
        {
            record_documentation_uninstall_qualification(
                tag,
                Path::new(assets),
                Path::new(evidence),
                true,
            )
        }
        [area, command, from_tag, to_tag, evidence]
            if area == "release" && command == "record-upgrade-qualification" =>
        {
            record_upgrade_qualification(
                from_tag,
                to_tag,
                Path::new(evidence),
                false,
            )
        }
        [area, command, from_tag, to_tag, evidence, write]
            if area == "release"
                && command == "record-upgrade-qualification"
                && write == "--write" =>
        {
            record_upgrade_qualification(from_tag, to_tag, Path::new(evidence), true)
        }
        [area, command, from_tag, from_assets, to_tag, to_assets]
            if area == "release" && command == "verify-package-candidates" =>
        {
            verify_package_candidate_pair(
                from_tag,
                Path::new(from_assets),
                to_tag,
                Path::new(to_assets),
            )
        }
        _ => Err(
            "usage: cargo run -p xtask -- schemas <check|write> | <resources|docs> check | \
             release <check|freeze-candidate|validate-tag TAG|verify-beta-readiness FILE|prepare-stage TAG [--write]|activate-feature-freeze COMMIT [--write]|record-beta-qualification TAG RUN_ID ASSETS [--write]|record-rc-qualification TAG RUN_ID ASSETS [--write]|record-upgrade-qualification FROM_TAG TO_TAG EVIDENCE [--write]|record-documentation-qualification TAG ASSETS EVIDENCE [--write]|sbom OUTPUT|assemble TAG COMMIT ARTIFACTS OUTPUT|verify TAG DIRECTORY|channels TAG ASSETS OUTPUT|bind-signing-evidence TAG TARGET EVIDENCE BINARY ARCHIVE|verify-package-candidates FROM_TAG FROM_ASSETS TO_TAG TO_ASSETS|verify-package-evidence FROM_TAG TO_TAG DIRECTORY|verify-upgrade-evidence FROM_TAG TO_TAG DIRECTORY|verify-documentation-evidence TAG ASSETS EVIDENCE>"
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

fn check_release_notes_policy() -> Result<(), String> {
    let root = repository_root();
    let policy_path = root.join("release/release-notes-policy.json");
    let policy: Value = serde_json::from_slice(&fs::read(&policy_path).map_err(|error| {
        format!(
            "release-notes policy is missing at {}: {error}",
            policy_path.display()
        )
    })?)
    .map_err(|error| format!("release-notes policy is invalid JSON: {error}"))?;
    let sections = [
        "Highlights",
        "Compatibility",
        "Install and verify",
        "Upgrade and rollback",
        "Security and privacy",
        "Known limitations",
        "Feedback and support",
    ];
    let guidance = [
        "does not require Python",
        "canisend.workspace/v2",
        "canisend.agent/v2",
        "never submits an application",
        "SHA256SUMS",
        "GitHub build provenance",
        "back up every important workspace",
        "restore the pre-upgrade backup into a new directory",
        "no in-place database downgrade",
        "no telemetry",
        "KNOWN_LIMITATIONS.md",
        "Never attach a workspace",
    ];
    let guides = [
        "docs/guides/release-verification.md",
        "docs/guides/quick-start.md",
        "docs/guides/upgrade-and-rollback.md",
    ];
    let expected = json!({
        "schema": RELEASE_NOTES_POLICY_SCHEMA,
        "stage_neutral_body": true,
        "heading_tracks_workspace_version": true,
        "required_sections": sections,
        "required_guidance": guidance,
        "required_repository_guides": guides,
        "final_review_required_at_rc": true
    });
    if policy != expected {
        return Err("release-notes policy differs from the native release contract".to_owned());
    }
    let workspace_body = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    let version = Version::parse(
        workspace["workspace"]["package"]["version"]
            .as_str()
            .ok_or_else(|| "workspace manifest has no package version".to_owned())?,
    )
    .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let notes_path = root.join("release/RELEASE_NOTES.md");
    let notes = fs::read_to_string(&notes_path)
        .map_err(|error| format!("release notes are missing: {error}"))?;
    validate_release_notes(&root, &version, &notes, &sections, &guidance, &guides)?;
    println!(
        "release notes: ok ({} stage-neutral sections, RC final review required)",
        sections.len()
    );
    Ok(())
}

fn validate_release_notes(
    root: &Path,
    version: &Version,
    notes: &str,
    expected_sections: &[&str],
    required_guidance: &[&str],
    guides: &[&str],
) -> Result<(), String> {
    let mut lines = notes.lines();
    let expected_heading = format!("# CanISend {version}");
    if lines.next() != Some(expected_heading.as_str())
        || notes.matches(expected_heading.as_str()).count() != 1
    {
        return Err(
            "release-note heading must identify the exact workspace version once".to_owned(),
        );
    }
    let sections = notes
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .collect::<Vec<_>>();
    if sections != expected_sections {
        return Err(format!(
            "release-note sections differ: expected {expected_sections:?}, found {sections:?}"
        ));
    }
    let normalized = notes.split_whitespace().collect::<Vec<_>>().join(" ");
    for phrase in required_guidance {
        if !normalized.contains(phrase) {
            return Err(format!(
                "release notes are missing required guidance `{phrase}`"
            ));
        }
    }
    let body = notes
        .split_once('\n')
        .map(|(_, body)| body)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if body
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|word| matches!(word, "alpha" | "beta" | "prerelease" | "stable"))
        || body.contains("release candidate")
    {
        return Err(
            "release-note body must remain stage-neutral; only the version heading may change"
                .to_owned(),
        );
    }
    for guide in guides {
        if !root.join(guide).is_file() {
            return Err(format!("release-note guide is missing: {guide}"));
        }
        let url = format!("https://github.com/jxpeng98/CanISend/blob/main/{guide}");
        if !notes.contains(&url) {
            return Err(format!(
                "release notes do not link required guide `{guide}`"
            ));
        }
    }
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct UpgradeQualificationSummary {
    run_id: u64,
    from_manifest_sha256: String,
    to_manifest_sha256: String,
    records: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DocumentationUninstallSummary {
    run_id: u64,
    records: usize,
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

struct RenderedStageTransition {
    from_version: Version,
    to_version: Version,
    from_stage: ReleaseStage,
    to_stage: ReleaseStage,
    files: BTreeMap<String, Vec<u8>>,
}

struct RenderedFeatureFreeze {
    baseline: String,
    files: BTreeMap<String, Vec<u8>>,
}

struct RenderedReleaseQualification {
    tag: String,
    run_id: u64,
    source_commit: String,
    manifest_sha256: String,
    ledger: Vec<u8>,
}

fn check_stage_transition_policy() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/stage-transition-policy.json");
    let policy: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "stage-transition policy is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("stage-transition policy is invalid JSON: {error}"))?;
    let expected = json!({
        "schema": STAGE_TRANSITION_POLICY_SCHEMA,
        "command": {
            "name": "cargo run -p xtask --locked -- release prepare-stage",
            "dry_run_default": true,
            "write_flag": "--write",
            "clean_worktree_required_for_write": true,
            "beta_readiness_max_age_hours": BETA_READINESS_MAX_AGE_HOURS
        },
        "allowed_transitions": [
            {
                "from": "alpha",
                "to": "beta",
                "target_prerelease": "beta.1",
                "ledger_status": "beta-qualifying",
                "release_notes_status": "beta-current"
            },
            {
                "from": "beta",
                "to": "rc",
                "target_prerelease": "rc.1",
                "ledger_status": "rc-qualifying",
                "release_notes_status": "rc-final"
            },
            {
                "from": "rc",
                "to": "stable",
                "target_prerelease": "",
                "ledger_status": "qualified",
                "release_notes_status": "stable-final"
            }
        ],
        "allowed_iterations": [
            {
                "stage": "rc",
                "target_prerelease": "next-sequential-rc",
                "ledger_status": "rc-qualifying",
                "release_notes_status": "rc-final"
            }
        ],
        "controlled_surfaces": [
            "Cargo.toml workspace version",
            "workspace Cargo.toml exact internal dependencies",
            "Cargo.lock workspace package versions",
            "release/qualification-ledger.json stage and Stable authorization fields",
            "release/RELEASE_NOTES.md heading",
            "release/support-policy.json Stable publication status"
        ],
        "preserved_history": [
            "release/beta-readiness.json",
            "release/beta-contract-freeze.json",
            "release/feedback-snapshot.json",
            "packaging/candidates/alpha"
        ]
    });
    if policy != expected {
        return Err(
            "stage-transition policy differs from the fail-closed release contract".to_owned(),
        );
    }
    let documentation_path = root.join("docs/release/stage-transitions.md");
    let documentation = fs::read_to_string(&documentation_path)
        .map_err(|error| format!("stage-transition runbook is missing: {error}"))?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    for required in [
        "release/stage-transition-policy.json",
        "prepare-stage v0.7.0-beta.1",
        "--write",
        "release/beta-readiness.json",
        "refresh_beta_readiness.sh",
    ] {
        if !documentation.contains(required) {
            return Err(format!("stage-transition runbook is missing `{required}`"));
        }
    }
    let refresh_path = root.join("scripts/refresh_beta_readiness.sh");
    let refresh = fs::read_to_string(&refresh_path)
        .map_err(|error| format!("Beta-readiness refresh script is missing: {error}"))?;
    for required in [
        "gh api --paginate --slurp",
        "select(has(\"pull_request\") | not)",
        "verify-beta-readiness",
        "open_issue_count",
        "--write",
    ] {
        if !refresh.contains(required) {
            return Err(format!(
                "Beta-readiness refresh script is missing `{required}`"
            ));
        }
    }
    println!("stage-transition policy: ok (3 stage transitions + sequential RC iteration)");
    Ok(())
}

fn prepare_stage_transition(tag: &str, write: bool) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "stage transition")?;
    }
    let transition = render_stage_transition(&root, tag)?;
    if write && matches!(transition.to_stage, ReleaseStage::Beta) {
        check_beta_readiness_freshness(&root, OffsetDateTime::now_utc())?;
    }
    let report = stage_transition_report(&root, &transition, write)?;
    if write {
        for (relative, body) in &transition.files {
            let path = root.join(relative);
            fs::write(&path, body)
                .map_err(|error| format!("could not write {}: {error}", path.display()))?;
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("could not serialize stage-transition plan: {error}"))?
    );
    Ok(())
}

fn activate_feature_freeze(baseline: &str, write: bool) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "feature-freeze activation")?;
    }
    let freeze = render_feature_freeze_activation(&root, baseline)?;
    let report = feature_freeze_report(&root, &freeze, write)?;
    if write {
        for (relative, body) in &freeze.files {
            let path = root.join(relative);
            fs::write(&path, body)
                .map_err(|error| format!("could not write {}: {error}", path.display()))?;
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("could not serialize feature-freeze plan: {error}"))?
    );
    Ok(())
}

fn record_beta_qualification(
    tag: &str,
    run_id: &str,
    assets: &Path,
    write: bool,
) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "Beta qualification")?;
    }
    let qualification = render_beta_qualification(&root, tag, run_id, assets)?;
    let ledger_path = root.join("release/qualification-ledger.json");
    let before = fs::read(&ledger_path)
        .map_err(|error| format!("could not read qualification ledger: {error}"))?;
    let report = json!({
        "schema": BETA_QUALIFICATION_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "tag": qualification.tag,
        "source_commit": qualification.source_commit,
        "signed_matrix_run": qualification.run_id,
        "release_manifest_sha256": qualification.manifest_sha256,
        "ledger": {
            "path": "release/qualification-ledger.json",
            "before_sha256": sha256(&before),
            "after_sha256": sha256(&qualification.ledger)
        },
        "next": "independently retain public attestation verification, commit the ledger, then activate the freeze"
    });
    if write {
        fs::write(&ledger_path, &qualification.ledger)
            .map_err(|error| format!("could not write {}: {error}", ledger_path.display()))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("could not serialize Beta qualification plan: {error}"))?
    );
    Ok(())
}

fn record_rc_qualification(
    tag: &str,
    run_id: &str,
    assets: &Path,
    write: bool,
) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "RC qualification")?;
    }
    let qualification = render_rc_qualification(&root, tag, run_id, assets)?;
    let ledger_path = root.join("release/qualification-ledger.json");
    let before = fs::read(&ledger_path)
        .map_err(|error| format!("could not read qualification ledger: {error}"))?;
    let report = json!({
        "schema": RC_QUALIFICATION_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "tag": qualification.tag,
        "source_commit": qualification.source_commit,
        "signed_matrix_run": qualification.run_id,
        "release_manifest_sha256": qualification.manifest_sha256,
        "ledger": {
            "path": "release/qualification-ledger.json",
            "before_sha256": sha256(&before),
            "after_sha256": sha256(&qualification.ledger)
        },
        "next": "commit this clean-tag matrix; record a distinct sequential RC before Stable"
    });
    if write {
        fs::write(&ledger_path, &qualification.ledger)
            .map_err(|error| format!("could not write {}: {error}", ledger_path.display()))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("could not serialize RC qualification plan: {error}"))?
    );
    Ok(())
}

fn record_upgrade_qualification(
    from_tag: &str,
    to_tag: &str,
    evidence: &Path,
    write: bool,
) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "upgrade qualification")?;
    }
    let summary = verify_upgrade_qualification_evidence(from_tag, to_tag, evidence)?;
    let ledger_path = root.join("release/qualification-ledger.json");
    let before = fs::read(&ledger_path)
        .map_err(|error| format!("could not read qualification ledger: {error}"))?;
    let ledger: Value = serde_json::from_slice(&before)
        .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    let qualified = upgrade_qualified_ledger(&ledger, from_tag, to_tag, &summary)?;
    let after = pretty_json_bytes(&qualified)?;
    let report = json!({
        "schema": UPGRADE_QUALIFICATION_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "from_tag": from_tag,
        "to_tag": to_tag,
        "github_run_id": summary.run_id,
        "records": summary.records,
        "manifests": {
            "from_sha256": summary.from_manifest_sha256,
            "to_sha256": summary.to_manifest_sha256
        },
        "ledger": {
            "path": "release/qualification-ledger.json",
            "before_sha256": sha256(&before),
            "after_sha256": sha256(&after)
        },
        "next": "independently inspect the public run and attestations, then commit the qualification ledger"
    });
    if write {
        fs::write(&ledger_path, after)
            .map_err(|error| format!("could not write {}: {error}", ledger_path.display()))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("could not serialize upgrade qualification plan: {error}"))?
    );
    Ok(())
}

fn upgrade_qualified_ledger(
    ledger: &Value,
    from_tag: &str,
    to_tag: &str,
    summary: &UpgradeQualificationSummary,
) -> Result<Value, String> {
    let (_, from_stage) = parse_release_tag(from_tag)?;
    let (_, to_stage) = parse_release_tag(to_tag)?;
    let pending = json!({
        "beta_tag": null,
        "evidence": [],
        "rc_tag": null,
        "status": "pending"
    });
    if from_stage != ReleaseStage::Beta
        || to_stage != ReleaseStage::ReleaseCandidate
        || summary.run_id == 0
        || summary.records != 5
        || ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "rc"
        || ledger["status"] != "rc-qualifying"
        || ledger["beta"]["status"] != "qualified"
        || ledger["beta"]["tag"] != from_tag
        || ledger["feature_freeze"]["status"] != "frozen"
        || ledger["stable_authorized"] != false
        || ledger["upgrade_matrix"] != pending
    {
        return Err("qualification ledger is not canonical pending RC upgrade state".to_owned());
    }
    let has_rc = ledger["release_candidates"]
        .as_array()
        .is_some_and(|candidates| {
            candidates
                .iter()
                .any(|candidate| candidate["status"] == "success" && candidate["tag"] == to_tag)
        });
    if !has_rc {
        return Err(
            "upgrade qualification RC must already have a successful signed matrix".to_owned(),
        );
    }
    let mut qualified = ledger.clone();
    qualified["upgrade_matrix"] = json!({
        "beta_tag": from_tag,
        "evidence": [
            format!(
                "native upgrade qualification run {} passed five signed archive targets",
                summary.run_id
            ),
            format!(
                "{from_tag} to {to_tag} backup, old-binary, restore, host-pack, and uninstall lifecycle passed"
            )
        ],
        "rc_tag": to_tag,
        "status": "passed"
    });
    Ok(qualified)
}

fn record_documentation_uninstall_qualification(
    tag: &str,
    assets: &Path,
    evidence: &Path,
    write: bool,
) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "documentation/uninstall qualification")?;
    }
    let (version, stage) = parse_release_tag(tag)?;
    if stage != ReleaseStage::ReleaseCandidate {
        return Err("documentation/uninstall qualification requires an RC tag".to_owned());
    }
    let workspace_body = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    if workspace["workspace"]["package"]["version"].as_str() != Some(version.to_string().as_str()) {
        return Err(
            "documentation/uninstall tag must match the current workspace version".to_owned(),
        );
    }
    let summary = verify_documentation_uninstall_evidence(tag, assets, evidence)?;
    let ledger_path = root.join("release/qualification-ledger.json");
    let before = fs::read(&ledger_path)
        .map_err(|error| format!("could not read qualification ledger: {error}"))?;
    let ledger: Value = serde_json::from_slice(&before)
        .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    let qualified = documentation_uninstall_qualified_ledger(&ledger, tag, summary)?;
    let after = pretty_json_bytes(&qualified)?;
    let report = json!({
        "schema": DOCUMENTATION_UNINSTALL_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "tag": tag,
        "github_run_id": summary.run_id,
        "records": summary.records,
        "ledger": {
            "path": "release/qualification-ledger.json",
            "before_sha256": sha256(&before),
            "after_sha256": sha256(&after)
        },
        "next": "independently inspect the RC run and public asset attestations, then commit the qualification ledger"
    });
    if write {
        fs::write(&ledger_path, after)
            .map_err(|error| format!("could not write {}: {error}", ledger_path.display()))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| {
            format!("could not serialize documentation/uninstall qualification plan: {error}")
        })?
    );
    Ok(())
}

fn documentation_uninstall_qualified_ledger(
    ledger: &Value,
    tag: &str,
    summary: DocumentationUninstallSummary,
) -> Result<Value, String> {
    let (_, stage) = parse_release_tag(tag)?;
    validate_documentation_uninstall_progress(&ledger["documentation_uninstall"])?;
    if stage != ReleaseStage::ReleaseCandidate
        || summary.run_id == 0
        || summary.records != 5
        || ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "rc"
        || ledger["status"] != "rc-qualifying"
        || ledger["beta"]["status"] != "qualified"
        || ledger["feature_freeze"]["status"] != "frozen"
        || ledger["stable_authorized"] != false
        || ledger["documentation_uninstall"]["status"] == "passed"
    {
        return Err(
            "qualification ledger is not eligible for RC documentation/uninstall evidence"
                .to_owned(),
        );
    }
    let candidates = ledger["release_candidates"]
        .as_array()
        .ok_or_else(|| "qualification ledger release_candidates must be an array".to_owned())?;
    let matching = candidates
        .iter()
        .find(|candidate| candidate["tag"] == tag && candidate["status"] == "success");
    let Some(candidate) = matching else {
        return Err(
            "documentation/uninstall qualification tag has no recorded RC matrix".to_owned(),
        );
    };
    let (_, _, recorded_run) =
        validate_qualification_release(candidate, ReleaseStage::ReleaseCandidate, "RC")?;
    if recorded_run != summary.run_id {
        return Err(
            "documentation/uninstall evidence must come from the same recorded RC matrix run"
                .to_owned(),
        );
    }
    let mut qualified = ledger.clone();
    qualified["documentation_uninstall"] = json!({
        "evidence": [
            format!(
                "native RC run {} passed exact archive documentation and uninstall smoke on five targets",
                summary.run_id
            ),
            format!(
                "{tag} retained external workspaces after installed binary and notice removal"
            )
        ],
        "native_matrix_run": summary.run_id,
        "status": "passed"
    });
    Ok(qualified)
}

fn render_beta_qualification(
    root: &Path,
    tag: &str,
    run_id: &str,
    assets: &Path,
) -> Result<RenderedReleaseQualification, String> {
    let run_id = run_id
        .parse::<u64>()
        .ok()
        .filter(|run| *run > 0)
        .ok_or_else(|| "Beta qualification run ID must be a positive integer".to_owned())?;
    let (version, stage) = parse_release_tag(tag)?;
    if stage != ReleaseStage::Beta {
        return Err("Beta qualification requires a Beta tag".to_owned());
    }
    let workspace_body = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    if workspace["workspace"]["package"]["version"].as_str() != Some(version.to_string().as_str()) {
        return Err("Beta qualification tag must match the current workspace version".to_owned());
    }

    verify_release(tag, assets)?;
    let manifest_path = assets.join(format!("canisend-{version}-manifest.json"));
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("could not read verified release manifest: {error}"))?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("verified release manifest is invalid JSON: {error}"))?;
    let source_commit = required_string(&manifest["source"], "commit", "Beta release source")?;
    validate_lower_hex("Beta release source commit", source_commit, 40)?;

    let ledger_path = root.join("release/qualification-ledger.json");
    let ledger: Value = serde_json::from_slice(
        &fs::read(&ledger_path)
            .map_err(|error| format!("could not read qualification ledger: {error}"))?,
    )
    .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    let ledger = beta_qualified_ledger(&ledger, tag, run_id, source_commit)?;
    Ok(RenderedReleaseQualification {
        tag: tag.to_owned(),
        run_id,
        source_commit: source_commit.to_owned(),
        manifest_sha256: sha256(&manifest_bytes),
        ledger: pretty_json_bytes(&ledger)?,
    })
}

fn render_rc_qualification(
    root: &Path,
    tag: &str,
    run_id: &str,
    assets: &Path,
) -> Result<RenderedReleaseQualification, String> {
    let run_id = run_id
        .parse::<u64>()
        .ok()
        .filter(|run| *run > 0)
        .ok_or_else(|| "RC qualification run ID must be a positive integer".to_owned())?;
    let (version, stage) = parse_release_tag(tag)?;
    if stage != ReleaseStage::ReleaseCandidate {
        return Err("RC qualification requires an RC tag".to_owned());
    }
    let workspace_body = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    if workspace["workspace"]["package"]["version"].as_str() != Some(version.to_string().as_str()) {
        return Err("RC qualification tag must match the current workspace version".to_owned());
    }

    verify_release(tag, assets)?;
    let manifest_path = assets.join(format!("canisend-{version}-manifest.json"));
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("could not read verified release manifest: {error}"))?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("verified release manifest is invalid JSON: {error}"))?;
    let source_commit = required_string(&manifest["source"], "commit", "RC release source")?;
    validate_lower_hex("RC release source commit", source_commit, 40)?;

    let ledger_path = root.join("release/qualification-ledger.json");
    let ledger: Value = serde_json::from_slice(
        &fs::read(&ledger_path)
            .map_err(|error| format!("could not read qualification ledger: {error}"))?,
    )
    .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    let ledger = rc_qualified_ledger(&ledger, tag, run_id, source_commit)?;
    Ok(RenderedReleaseQualification {
        tag: tag.to_owned(),
        run_id,
        source_commit: source_commit.to_owned(),
        manifest_sha256: sha256(&manifest_bytes),
        ledger: pretty_json_bytes(&ledger)?,
    })
}

fn beta_qualified_ledger(
    ledger: &Value,
    tag: &str,
    run_id: u64,
    source_commit: &str,
) -> Result<Value, String> {
    let (_, stage) = parse_release_tag(tag)?;
    validate_lower_hex("Beta source commit", source_commit, 40)?;
    let pending = json!({
        "signed_matrix_run": null,
        "signing_evidence_targets": [],
        "source_commit": null,
        "status": "pending",
        "tag": null
    });
    if stage != ReleaseStage::Beta
        || run_id == 0
        || ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "beta"
        || ledger["status"] != "beta-qualifying"
        || ledger["beta"] != pending
        || ledger["feature_freeze"]["status"] != "planned"
        || !ledger["feature_freeze"]["baseline_commit"].is_null()
        || ledger["stable_authorized"] != false
    {
        return Err("qualification ledger is not canonical pending Beta state".to_owned());
    }
    let mut qualified = ledger.clone();
    qualified["beta"] = json!({
        "signed_matrix_run": run_id,
        "signing_evidence_targets": [
            "aarch64-apple-darwin",
            "x86_64-apple-darwin",
            "x86_64-pc-windows-msvc"
        ],
        "source_commit": source_commit,
        "status": "qualified",
        "tag": tag
    });
    Ok(qualified)
}

fn rc_qualified_ledger(
    ledger: &Value,
    tag: &str,
    run_id: u64,
    source_commit: &str,
) -> Result<Value, String> {
    let (_, stage) = parse_release_tag(tag)?;
    validate_lower_hex("RC source commit", source_commit, 40)?;
    let baseline = required_string(
        &ledger["feature_freeze"],
        "baseline_commit",
        "feature freeze",
    )?;
    validate_lower_hex("feature-freeze baseline commit", baseline, 40)?;
    if stage != ReleaseStage::ReleaseCandidate
        || run_id == 0
        || ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "rc"
        || ledger["status"] != "rc-qualifying"
        || ledger["beta"]["status"] != "qualified"
        || ledger["feature_freeze"]["status"] != "frozen"
        || ledger["stable_authorized"] != false
    {
        return Err("qualification ledger is not canonical RC-qualifying state".to_owned());
    }
    let candidates = ledger["release_candidates"]
        .as_array()
        .ok_or_else(|| "qualification ledger release_candidates must be an array".to_owned())?;
    for candidate in candidates {
        if candidate["status"] != "success" {
            return Err("existing RC qualification is not successful".to_owned());
        }
        let (existing_tag, existing_commit, existing_run) =
            validate_qualification_release(candidate, ReleaseStage::ReleaseCandidate, "RC")?;
        if existing_tag == tag || existing_commit == source_commit || existing_run == run_id {
            return Err(
                "RC qualification tag, source commit, and run ID must all be distinct".to_owned(),
            );
        }
    }
    let mut qualified = ledger.clone();
    qualified["release_candidates"]
        .as_array_mut()
        .expect("validated RC candidate array")
        .push(json!({
            "signed_matrix_run": run_id,
            "source_commit": source_commit,
            "status": "success",
            "tag": tag
        }));
    Ok(qualified)
}

fn require_clean_worktree(root: &Path, context: &str) -> Result<(), String> {
    let changes = run_git_lines(root, &["status", "--porcelain", "--untracked-files=all"])?;
    if changes.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{context} write requires a clean worktree; commit or stash owned changes first"
        ))
    }
}

fn render_feature_freeze_activation(
    root: &Path,
    baseline: &str,
) -> Result<RenderedFeatureFreeze, String> {
    validate_lower_hex("feature-freeze baseline commit", baseline, 40)?;
    run_git(root, &["cat-file", "-e", &format!("{baseline}^{{commit}}")])?;
    let head = run_git_lines(root, &["rev-parse", "HEAD"])?
        .into_iter()
        .next()
        .ok_or_else(|| "could not resolve HEAD for feature-freeze activation".to_owned())?;
    if baseline != head {
        return Err(format!(
            "feature-freeze activation baseline must equal current HEAD `{head}`"
        ));
    }

    let ledger_path = root.join("release/qualification-ledger.json");
    let mut ledger: Value = serde_json::from_slice(
        &fs::read(&ledger_path)
            .map_err(|error| format!("could not read qualification ledger: {error}"))?,
    )
    .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    if ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "beta"
        || ledger["status"] != "beta-qualifying"
        || ledger["beta"]["status"] != "qualified"
        || ledger["stable_authorized"] != false
    {
        return Err(
            "feature-freeze activation requires a qualified signed Beta workspace".to_owned(),
        );
    }
    if ledger["feature_freeze"]["status"] != "planned"
        || !ledger["feature_freeze"]["baseline_commit"].is_null()
    {
        return Err("feature freeze is not in canonical planned state".to_owned());
    }

    let exceptions_path = root.join("release/feature-freeze-exceptions.json");
    let mut exceptions: Value = serde_json::from_slice(
        &fs::read(&exceptions_path)
            .map_err(|error| format!("could not read feature-freeze exception record: {error}"))?,
    )
    .map_err(|error| format!("feature-freeze exception record is invalid JSON: {error}"))?;
    let planned = json!({
        "schema": FEATURE_FREEZE_EXCEPTIONS_SCHEMA,
        "status": "planned",
        "baseline_commit": null,
        "exceptions": []
    });
    if exceptions != planned {
        return Err("feature-freeze exception record is not canonical planned state".to_owned());
    }

    ledger["feature_freeze"]["status"] = Value::String("frozen".to_owned());
    ledger["feature_freeze"]["baseline_commit"] = Value::String(baseline.to_owned());
    exceptions["status"] = Value::String("frozen".to_owned());
    exceptions["baseline_commit"] = Value::String(baseline.to_owned());
    let files = BTreeMap::from([
        (
            "release/feature-freeze-exceptions.json".to_owned(),
            pretty_json_bytes(&exceptions)?,
        ),
        (
            "release/qualification-ledger.json".to_owned(),
            pretty_json_bytes(&ledger)?,
        ),
    ]);
    Ok(RenderedFeatureFreeze {
        baseline: baseline.to_owned(),
        files,
    })
}

fn feature_freeze_report(
    root: &Path,
    freeze: &RenderedFeatureFreeze,
    write: bool,
) -> Result<Value, String> {
    let files = freeze
        .files
        .iter()
        .map(|(relative, after)| {
            let before = fs::read(root.join(relative))
                .map_err(|error| format!("could not read {relative}: {error}"))?;
            Ok(json!({
                "path": relative,
                "before_sha256": sha256(&before),
                "after_sha256": sha256(after)
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!({
        "schema": FEATURE_FREEZE_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "baseline_commit": freeze.baseline,
        "files": files,
        "next": "commit the two automatic release-state files, then run release check"
    }))
}

fn check_beta_readiness_freshness(root: &Path, now: OffsetDateTime) -> Result<(), String> {
    let path = root.join("release/beta-readiness.json");
    let readiness: Value =
        serde_json::from_slice(&fs::read(&path).map_err(|error| {
            format!("could not read Beta readiness for stage transition: {error}")
        })?)
        .map_err(|error| format!("Beta readiness is invalid JSON: {error}"))?;
    let audited_at = required_string(&readiness, "audited_at", "Beta readiness")?;
    let audited_at = OffsetDateTime::parse(audited_at, &Rfc3339)
        .map_err(|error| format!("Beta readiness audit timestamp is invalid: {error}"))?;
    if audited_at > now + Duration::minutes(5) {
        return Err("Beta readiness audit timestamp is unreasonably in the future".to_owned());
    }
    let age = now - audited_at;
    if age > Duration::hours(BETA_READINESS_MAX_AGE_HOURS) {
        return Err(format!(
            "Beta readiness audit is older than {BETA_READINESS_MAX_AGE_HOURS} hours; refresh it before --write"
        ));
    }
    Ok(())
}

fn render_stage_transition(root: &Path, tag: &str) -> Result<RenderedStageTransition, String> {
    let workspace_path = root.join("Cargo.toml");
    let workspace_body = fs::read_to_string(&workspace_path)
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    let from_version = Version::parse(
        workspace["workspace"]["package"]["version"]
            .as_str()
            .ok_or_else(|| "workspace manifest has no package version".to_owned())?,
    )
    .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let from_stage = ReleaseStage::from_version(&from_version)?;
    let (to_version, to_stage) = parse_release_tag(tag)?;
    validate_stage_transition(&from_version, from_stage, &to_version, to_stage)?;

    let members = workspace["workspace"]["members"]
        .as_array()
        .ok_or_else(|| "workspace manifest has no members array".to_owned())?
        .iter()
        .map(|member| {
            member
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "workspace member must be a string".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut package_names = BTreeSet::new();
    for member in &members {
        let body = fs::read_to_string(root.join(member).join("Cargo.toml"))
            .map_err(|error| format!("could not read {member}/Cargo.toml: {error}"))?;
        let manifest: toml::Value = body
            .parse()
            .map_err(|error| format!("{member}/Cargo.toml is invalid TOML: {error}"))?;
        package_names.insert(
            manifest["package"]["name"]
                .as_str()
                .ok_or_else(|| format!("{member}/Cargo.toml has no package name"))?
                .to_owned(),
        );
    }

    let from = from_version.to_string();
    let to = to_version.to_string();
    let mut files = BTreeMap::new();
    let workspace_after = replace_exact_count(
        &workspace_body,
        &format!("version = \"{from}\""),
        &format!("version = \"{to}\""),
        1,
        "workspace version",
    )?;
    files.insert("Cargo.toml".to_owned(), workspace_after.into_bytes());

    for member in &members {
        let relative = format!("{member}/Cargo.toml");
        let body = fs::read_to_string(root.join(&relative))
            .map_err(|error| format!("could not read {relative}: {error}"))?;
        let needle = format!("version = \"={from}\"");
        let occurrences = body.matches(&needle).count();
        if occurrences > 0 {
            let updated = replace_exact_count(
                &body,
                &needle,
                &format!("version = \"={to}\""),
                occurrences,
                &format!("internal dependency versions in {relative}"),
            )?;
            files.insert(relative, updated.into_bytes());
        }
    }

    let lock_path = root.join("Cargo.lock");
    let mut lock = fs::read_to_string(&lock_path)
        .map_err(|error| format!("could not read Cargo.lock: {error}"))?;
    for package in &package_names {
        lock = replace_exact_count(
            &lock,
            &format!("name = \"{package}\"\nversion = \"{from}\""),
            &format!("name = \"{package}\"\nversion = \"{to}\""),
            1,
            &format!("Cargo.lock package `{package}`"),
        )?;
    }
    files.insert("Cargo.lock".to_owned(), lock.into_bytes());

    let ledger_path = root.join("release/qualification-ledger.json");
    let mut ledger: Value = serde_json::from_slice(
        &fs::read(&ledger_path)
            .map_err(|error| format!("could not read qualification ledger: {error}"))?,
    )
    .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    validate_transition_ledger_preconditions(&ledger, from_stage, to_stage)?;
    ledger["workspace_stage"] = Value::String(to_stage.as_str().to_owned());
    ledger["status"] = Value::String(qualification_status_for_stage(to_stage).to_owned());
    ledger["release_notes"]["status"] =
        Value::String(release_notes_status_for_stage(to_stage).to_owned());
    if matches!(to_stage, ReleaseStage::Stable) {
        ledger["stable_authorized"] = Value::Bool(true);
    }
    let ledger_bytes = pretty_json_bytes(&ledger)?;
    if fs::read(&ledger_path)
        .map_err(|error| format!("could not reread qualification ledger: {error}"))?
        != ledger_bytes
    {
        files.insert("release/qualification-ledger.json".to_owned(), ledger_bytes);
    }

    let notes_path = root.join("release/RELEASE_NOTES.md");
    let notes = fs::read_to_string(&notes_path)
        .map_err(|error| format!("could not read release notes: {error}"))?;
    files.insert(
        "release/RELEASE_NOTES.md".to_owned(),
        replace_exact_count(
            &notes,
            &format!("# CanISend {from}"),
            &format!("# CanISend {to}"),
            1,
            "release-note version heading",
        )?
        .into_bytes(),
    );

    if matches!(to_stage, ReleaseStage::Stable) {
        let support_path = root.join("release/support-policy.json");
        let mut support: Value = serde_json::from_slice(
            &fs::read(&support_path)
                .map_err(|error| format!("could not read support policy: {error}"))?,
        )
        .map_err(|error| format!("support policy is invalid JSON: {error}"))?;
        if support["publication_status"] != "pre-stable-draft" {
            return Err("Stable transition requires a pre-stable support-policy draft".to_owned());
        }
        support["publication_status"] = Value::String("published".to_owned());
        files.insert(
            "release/support-policy.json".to_owned(),
            pretty_json_bytes(&support)?,
        );
    }

    for (relative, body) in &files {
        let current = fs::read(root.join(relative))
            .map_err(|error| format!("could not read {relative}: {error}"))?;
        if current == *body {
            return Err(format!(
                "stage transition would not change controlled file `{relative}`"
            ));
        }
    }
    Ok(RenderedStageTransition {
        from_version,
        to_version,
        from_stage,
        to_stage,
        files,
    })
}

fn validate_stage_transition(
    from: &Version,
    from_stage: ReleaseStage,
    to: &Version,
    to_stage: ReleaseStage,
) -> Result<(), String> {
    if (from.major, from.minor, from.patch) != (to.major, to.minor, to.patch)
        || !from.build.is_empty()
        || !to.build.is_empty()
    {
        return Err(
            "stage transitions must preserve the release line and omit build metadata".to_owned(),
        );
    }
    let expected_prerelease = match (from_stage, to_stage) {
        (ReleaseStage::ReleaseCandidate, ReleaseStage::ReleaseCandidate) => {
            let from_iteration = prerelease_iteration(from, "rc")?;
            let to_iteration = prerelease_iteration(to, "rc")?;
            if to_iteration != from_iteration + 1 {
                return Err(
                    "RC iteration target must increment the prerelease number by one".to_owned(),
                );
            }
            return Ok(());
        }
        (ReleaseStage::Alpha, ReleaseStage::Beta) => "beta.1",
        (ReleaseStage::Beta, ReleaseStage::ReleaseCandidate) => "rc.1",
        (ReleaseStage::ReleaseCandidate, ReleaseStage::Stable) => "",
        _ => {
            return Err(format!(
                "unsupported stage transition {} -> {}; only the next release stage is allowed",
                from_stage.as_str(),
                to_stage.as_str()
            ));
        }
    };
    if to.pre.as_str() != expected_prerelease {
        return Err(format!(
            "{} -> {} transition target must use prerelease `{expected_prerelease}`",
            from_stage.as_str(),
            to_stage.as_str()
        ));
    }
    Ok(())
}

fn prerelease_iteration(version: &Version, prefix: &str) -> Result<u64, String> {
    let (actual_prefix, iteration) = version
        .pre
        .as_str()
        .split_once('.')
        .ok_or_else(|| format!("{prefix} version has no numeric prerelease iteration"))?;
    if actual_prefix != prefix || iteration.contains('.') {
        return Err(format!("version prerelease must use `{prefix}.N`"));
    }
    iteration
        .parse::<u64>()
        .ok()
        .filter(|iteration| *iteration > 0)
        .ok_or_else(|| format!("{prefix} prerelease iteration must be a positive integer"))
}

fn validate_transition_ledger_preconditions(
    ledger: &Value,
    from_stage: ReleaseStage,
    to_stage: ReleaseStage,
) -> Result<(), String> {
    if ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != from_stage.as_str()
        || ledger["status"] != qualification_status_for_stage(from_stage)
        || ledger["stable_authorized"] != false
    {
        return Err("qualification ledger does not match the current workspace stage".to_owned());
    }
    if matches!(to_stage, ReleaseStage::ReleaseCandidate)
        && (ledger["beta"]["status"] != "qualified"
            || ledger["feature_freeze"]["status"] != "frozen")
    {
        return Err(
            "RC transition requires a qualified signed Beta and active feature freeze".to_owned(),
        );
    }
    if matches!(to_stage, ReleaseStage::Stable) {
        let mut candidate = ledger.clone();
        candidate["release_notes"]["status"] = Value::String("stable-final".to_owned());
        candidate["stable_authorized"] = Value::Bool(true);
        validate_stable_qualification(&candidate)?;
    }
    Ok(())
}

fn release_notes_status_for_stage(stage: ReleaseStage) -> &'static str {
    match stage {
        ReleaseStage::Alpha => "alpha-current",
        ReleaseStage::Beta => "beta-current",
        ReleaseStage::ReleaseCandidate => "rc-final",
        ReleaseStage::Stable => "stable-final",
    }
}

fn replace_exact_count(
    body: &str,
    from: &str,
    to: &str,
    expected: usize,
    context: &str,
) -> Result<String, String> {
    let actual = body.matches(from).count();
    if actual != expected {
        return Err(format!(
            "{context} expected {expected} exact source values, found {actual}"
        ));
    }
    Ok(body.replace(from, to))
}

fn pretty_json_bytes(value: &Value) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("could not serialize stage-transition JSON: {error}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn stage_transition_report(
    root: &Path,
    transition: &RenderedStageTransition,
    write: bool,
) -> Result<Value, String> {
    let files = transition
        .files
        .iter()
        .map(|(relative, after)| {
            let before = fs::read(root.join(relative))
                .map_err(|error| format!("could not read {relative}: {error}"))?;
            Ok(json!({
                "path": relative,
                "before_sha256": sha256(&before),
                "after_sha256": sha256(after)
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!({
        "schema": STAGE_TRANSITION_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "from": {
            "version": transition.from_version.to_string(),
            "stage": transition.from_stage.as_str()
        },
        "to": {
            "version": transition.to_version.to_string(),
            "stage": transition.to_stage.as_str()
        },
        "files": files,
        "preserved_history": [
            "release/beta-readiness.json",
            "release/beta-contract-freeze.json",
            "release/feedback-snapshot.json",
            "packaging/candidates/alpha"
        ]
    }))
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
        "release/stage-transition-policy.json",
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
    check_beta_readiness_file(&path)
}

fn check_beta_readiness_file(path: &Path) -> Result<(), String> {
    let body = fs::read_to_string(path).map_err(|error| {
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
    check_feature_freeze_exceptions(feature_freeze)?;

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
    let expected_release_notes_status = release_notes_status_for_stage(stage);
    if release_notes_status != expected_release_notes_status {
        return Err(format!(
            "release notes status must be `{expected_release_notes_status}` for {} stage",
            stage.as_str()
        ));
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
    let documentation_path = root.join("docs/release/qualification-ledger.md");
    let documentation = fs::read_to_string(&documentation_path)
        .map_err(|error| format!("qualification-ledger documentation is missing: {error}"))?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    for required in [
        "record-beta-qualification",
        "record-rc-qualification",
        "DOWNLOADED_ASSET_DIRECTORY",
        "gh attestation verify",
        "--write",
    ] {
        if !documentation.contains(required) {
            return Err(format!(
                "qualification-ledger documentation is missing `{required}`"
            ));
        }
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

fn check_feature_freeze_exceptions(feature_freeze: &Value) -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/feature-freeze-exceptions.json");
    let record: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "feature-freeze exception record is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("feature-freeze exception record is invalid JSON: {error}"))?;
    if record["schema"] != FEATURE_FREEZE_EXCEPTIONS_SCHEMA {
        return Err("feature-freeze exception schema is invalid".to_owned());
    }
    let status = required_string(feature_freeze, "status", "feature freeze")?;
    if record["status"] != status || record["baseline_commit"] != feature_freeze["baseline_commit"]
    {
        return Err(
            "feature-freeze exception record differs from the qualification ledger".to_owned(),
        );
    }
    let exceptions = record["exceptions"]
        .as_array()
        .ok_or_else(|| "feature-freeze exceptions must be an array".to_owned())?;
    let documentation_path = root.join("docs/release/feature-freeze.md");
    let documentation = fs::read_to_string(&documentation_path)
        .map_err(|error| format!("feature-freeze documentation is missing: {error}"))?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    for required in [
        "activate-feature-freeze FULL_HEAD_COMMIT",
        "--write",
        "equal to current `HEAD`",
    ] {
        if !documentation.contains(required) {
            return Err(format!(
                "feature-freeze documentation is missing `{required}`"
            ));
        }
    }
    if status == "planned" {
        let expected = json!({
            "schema": FEATURE_FREEZE_EXCEPTIONS_SCHEMA,
            "status": "planned",
            "baseline_commit": null,
            "exceptions": []
        });
        if record != expected {
            return Err("planned feature freeze cannot pre-authorize exceptions".to_owned());
        }
        println!("feature freeze: ok (planned, no preauthorized exceptions)");
        return Ok(());
    }

    let baseline = required_string(feature_freeze, "baseline_commit", "feature freeze")?;
    validate_lower_hex("feature-freeze baseline commit", baseline, 40)?;
    validate_feature_freeze_history(&root, baseline, exceptions)?;
    println!(
        "feature freeze: ok (frozen at {baseline}, {} exceptions)",
        exceptions.len()
    );
    Ok(())
}

fn validate_feature_freeze_history(
    root: &Path,
    baseline: &str,
    exceptions: &[Value],
) -> Result<(), String> {
    run_git(root, &["cat-file", "-e", &format!("{baseline}^{{commit}}")])?;
    run_git(root, &["merge-base", "--is-ancestor", baseline, "HEAD"])?;
    let range = format!("{baseline}..HEAD");
    let commits = run_git_lines(root, &["rev-list", "--reverse", &range])?;
    let mut changed_by_commit = BTreeMap::new();
    for commit in &commits {
        let paths = run_git_lines(
            root,
            &[
                "diff-tree",
                "--first-parent",
                "-m",
                "--no-commit-id",
                "--name-only",
                "-r",
                commit,
            ],
        )?;
        let nonautomatic = paths
            .into_iter()
            .filter(|path| !is_automatic_feature_freeze_path(path))
            .collect::<BTreeSet<_>>();
        if !nonautomatic.is_empty() {
            changed_by_commit.insert(commit.clone(), nonautomatic);
        }
    }

    let mut recorded_commits = Vec::new();
    for entry in exceptions {
        let commit = required_string(entry, "commit", "feature-freeze exception")?;
        validate_lower_hex("feature-freeze exception commit", commit, 40)?;
        let class = required_string(entry, "class", "feature-freeze exception")?;
        if !matches!(class, "release-blocker" | "release-evidence") {
            return Err("feature-freeze exception class is invalid".to_owned());
        }
        let reason = required_string(entry, "reason", "feature-freeze exception")?;
        if reason.len() > 500 || reason.chars().any(char::is_control) {
            return Err("feature-freeze exception reason is invalid".to_owned());
        }
        let paths = entry["paths"]
            .as_array()
            .ok_or_else(|| "feature-freeze exception paths are missing".to_owned())?
            .iter()
            .map(|path| {
                path.as_str()
                    .filter(|path| !path.is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| "feature-freeze exception path is invalid".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let path_set = paths.iter().cloned().collect::<BTreeSet<_>>();
        if path_set.len() != paths.len()
            || paths.iter().ne(path_set.iter())
            || paths.iter().any(|path| {
                path.starts_with('/')
                    || path.contains('\\')
                    || path.split('/').any(|part| matches!(part, "" | "." | ".."))
                    || path.chars().any(char::is_control)
            })
        {
            return Err(
                "feature-freeze exception paths must be unique sorted repository paths".to_owned(),
            );
        }
        let actual = changed_by_commit.get(commit).ok_or_else(|| {
            format!("feature-freeze exception commit `{commit}` has no exceptional changed paths")
        })?;
        if &path_set != actual {
            return Err(format!(
                "feature-freeze exception paths differ for commit `{commit}`"
            ));
        }
        let canonical = json!({
            "commit": commit,
            "class": class,
            "reason": reason,
            "paths": paths
        });
        if *entry != canonical {
            return Err(
                "feature-freeze exception contains unknown or noncanonical fields".to_owned(),
            );
        }
        recorded_commits.push(commit.to_owned());
    }
    let expected_commits = commits
        .into_iter()
        .filter(|commit| changed_by_commit.contains_key(commit))
        .collect::<Vec<_>>();
    if recorded_commits != expected_commits {
        return Err(
            "feature-freeze exceptions do not cover the exact post-baseline commit order"
                .to_owned(),
        );
    }
    Ok(())
}

fn is_automatic_feature_freeze_path(path: &str) -> bool {
    path.starts_with("docs/")
        || path.starts_with("packaging/candidates/")
        || path.starts_with("release/evidence/")
        || matches!(
            path,
            "README.md"
                | "CONTRIBUTING.md"
                | "SECURITY.md"
                | "CHANGELOG.md"
                | "release/RELEASE_NOTES.md"
                | "release/qualification-ledger.json"
                | "release/feedback-snapshot.json"
                | "release/support-policy.json"
                | "release/feature-freeze-exceptions.json"
        )
}

fn run_git(root: &Path, arguments: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not execute Git repository check: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Git repository command `git {}` failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn run_git_lines(root: &Path, arguments: &[&str]) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not execute Git repository check: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Git repository command `git {}` failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| "Git repository output is not UTF-8".to_owned())?;
    Ok(stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
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

    let workflow_path = root.join(".github/workflows/package-manager-qualification.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("package-manager qualification workflow is missing: {error}"))?;
    for required in [
        "name: package-manager-prequalification",
        "workflow_dispatch:",
        "verify-package-candidates",
        "macos-15-intel",
        "windows-2025",
        "b0ee913725139b816f9178163af0aecdba07a7ed",
        "48f6ea398b3a3fa26fae0093d37bd85b13e7eaa5d1d4a3e208408768408e35ae",
        "winget-sandbox-kit",
        "No external package repository was changed.",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "package-manager qualification workflow is missing `{required}`"
            ));
        }
    }
    let homebrew = fs::read_to_string(root.join("scripts/qualify_homebrew_packages.sh"))
        .map_err(|error| format!("Homebrew qualification script is missing: {error}"))?;
    for required in [
        "brew audit --strict --cask",
        "brew install --cask",
        "brew upgrade --cask",
        "brew uninstall --cask",
        "workspace-retained",
        "no-publication",
    ] {
        if !homebrew.contains(required) {
            return Err(format!(
                "Homebrew qualification script is missing `{required}`"
            ));
        }
    }
    let windows = fs::read_to_string(root.join("scripts/qualify_windows_packages.ps1"))
        .map_err(|error| format!("Windows package qualification script is missing: {error}"))?;
    for required in [
        "scoop update canisend",
        "winget validate --manifest",
        "winget install --manifest",
        "winget upgrade --manifest",
        "winget uninstall --id PengJiaxin.CanISend",
        "winget settings --enable LocalManifestFiles",
        "winget settings --disable LocalManifestFiles",
        "workspace-retained",
        "no-publication",
    ] {
        if !windows.contains(required) {
            return Err(format!(
                "Windows qualification script is missing `{required}`"
            ));
        }
    }
    let sandbox_path = root.join("docs/release/winget-sandbox-qualification.md");
    let sandbox = fs::read_to_string(&sandbox_path)
        .map_err(|error| format!("WinGet Sandbox qualification guide is missing: {error}"))?;
    check_local_markdown_links(&root, &sandbox_path, &sandbox)?;
    println!("package-manager qualification policy: ok (4 native records)");
    Ok(())
}

fn check_upgrade_qualification_policy() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/upgrade-qualification-policy.json");
    let actual: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "upgrade qualification policy is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("upgrade qualification policy is invalid JSON: {error}"))?;
    let required_checks = [
        "verified-release-pair",
        "from-version-and-doctor",
        "workspace-created-and-checked",
        "verified-pre-upgrade-backup",
        "to-version-and-doctor",
        "workspace-upgraded-and-checked",
        "old-binary-behavior-verified",
        "backup-restored-to-new-path",
        "restored-workspace-checked-by-old-binary",
        "host-pack-regenerated",
        "installed-binary-and-notices-uninstalled",
        "workspace-backup-and-restore-retained",
        "no-publication",
    ];
    let expected = json!({
        "schema": UPGRADE_QUALIFICATION_POLICY_SCHEMA,
        "release_pair": {
            "from_stage": "beta",
            "to_stage": "rc",
            "same_release_line": true,
            "public_signed_assets_required": true
        },
        "records": [
            {
                "record": "upgrade-aarch64-apple-darwin",
                "target": "aarch64-apple-darwin",
                "environment": "macos-15"
            },
            {
                "record": "upgrade-x86_64-apple-darwin",
                "target": "x86_64-apple-darwin",
                "environment": "macos-15-intel"
            },
            {
                "record": "upgrade-x86_64-unknown-linux-gnu",
                "target": "x86_64-unknown-linux-gnu",
                "environment": "ubuntu-24.04"
            },
            {
                "record": "upgrade-x86_64-unknown-linux-musl",
                "target": "x86_64-unknown-linux-musl",
                "environment": "ubuntu-24.04"
            },
            {
                "record": "upgrade-x86_64-pc-windows-msvc",
                "target": "x86_64-pc-windows-msvc",
                "environment": "windows-2025"
            }
        ],
        "allowed_old_binary_behavior": [
            "same-schema-accepted",
            "future-schema-rejected-without-mutation"
        ],
        "required_checks": required_checks,
        "evidence": {
            "schema": UPGRADE_QUALIFICATION_SCHEMA,
            "one_github_run": true,
            "one_manifest_pair": true,
            "all_checks_must_pass": true,
            "exact_record_set": true
        },
        "publication_authorized": false
    });
    if actual != expected {
        return Err(
            "upgrade qualification policy differs from the native release contract".to_owned(),
        );
    }

    let documentation_path = root.join("docs/release/upgrade-qualification.md");
    let documentation = fs::read_to_string(&documentation_path)
        .map_err(|error| format!("upgrade qualification documentation is missing: {error}"))?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    for required in [
        "native-upgrade-qualification",
        "verify-upgrade-evidence",
        "record-upgrade-qualification",
        "gh attestation verify",
        "--write",
    ] {
        if !documentation.contains(required) {
            return Err(format!(
                "upgrade qualification documentation is missing `{required}`"
            ));
        }
    }

    let workflow_path = root.join(".github/workflows/upgrade-qualification.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("upgrade qualification workflow is missing: {error}"))?;
    for required in [
        "name: native-upgrade-qualification",
        "workflow_dispatch:",
        "gh attestation verify",
        "qualify_archive_upgrade.sh",
        "macos-15-intel",
        "ubuntu-24.04",
        "windows-2025",
        "verify-upgrade-evidence",
        "No release or package channel was changed.",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "upgrade qualification workflow is missing `{required}`"
            ));
        }
    }
    let script = fs::read_to_string(root.join("scripts/qualify_archive_upgrade.sh"))
        .map_err(|error| format!("archive upgrade qualification script is missing: {error}"))?;
    for required in [
        ".canisend/state.sqlite3",
        "workspace backup",
        "workspace restore",
        "workspace.conflict",
        "agent assets export --host codex",
        "installed-binary-and-notices-uninstalled",
        "no-publication",
    ] {
        if !script.contains(required) {
            return Err(format!(
                "archive upgrade qualification script is missing `{required}`"
            ));
        }
    }
    println!("upgrade qualification policy: ok (5 native records)");
    Ok(())
}

fn check_documentation_uninstall_policy() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/documentation-uninstall-policy.json");
    let actual: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "documentation/uninstall policy is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("documentation/uninstall policy is invalid JSON: {error}"))?;
    let required_checks = [
        "exact-binary-match",
        "complete-notice-bundle",
        "version-and-doctor",
        "documented-quickstart",
        "host-agent-smoke",
        "isolated-install",
        "uninstall",
        "workspace-retained",
        "no-publication",
    ];
    let expected = json!({
        "schema": DOCUMENTATION_UNINSTALL_POLICY_SCHEMA,
        "release_stage": "rc",
        "same_run_as_qualified_rc": true,
        "records": [
            {
                "record": "documentation-uninstall-aarch64-apple-darwin",
                "target": "aarch64-apple-darwin",
                "environment": "macos-15"
            },
            {
                "record": "documentation-uninstall-x86_64-apple-darwin",
                "target": "x86_64-apple-darwin",
                "environment": "macos-15-intel"
            },
            {
                "record": "documentation-uninstall-x86_64-unknown-linux-gnu",
                "target": "x86_64-unknown-linux-gnu",
                "environment": "ubuntu-24.04"
            },
            {
                "record": "documentation-uninstall-x86_64-unknown-linux-musl",
                "target": "x86_64-unknown-linux-musl",
                "environment": "ubuntu-24.04"
            },
            {
                "record": "documentation-uninstall-x86_64-pc-windows-msvc",
                "target": "x86_64-pc-windows-msvc",
                "environment": "windows-2025"
            }
        ],
        "required_checks": required_checks,
        "evidence": {
            "schema": DOCUMENTATION_UNINSTALL_SCHEMA,
            "exact_record_set": true,
            "one_github_run": true,
            "bind_verified_archive_sha256": true,
            "all_checks_must_pass": true
        },
        "publication_authorized": false
    });
    if actual != expected {
        return Err(
            "documentation/uninstall policy differs from the native release contract".to_owned(),
        );
    }
    let documentation_path = root.join("docs/release/documentation-uninstall-qualification.md");
    let documentation = fs::read_to_string(&documentation_path).map_err(|error| {
        format!("documentation/uninstall qualification guide is missing: {error}")
    })?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    for required in [
        "verify-documentation-evidence",
        "record-documentation-qualification",
        "same RC run",
        "--write",
    ] {
        if !documentation.contains(required) {
            return Err(format!(
                "documentation/uninstall qualification guide is missing `{required}`"
            ));
        }
    }
    let workflow = fs::read_to_string(root.join(".github/workflows/release.yml"))
        .map_err(|error| format!("release workflow is missing: {error}"))?;
    for required in [
        "documentation-uninstall-${{ matrix.target }}.json",
        "verify-documentation-evidence",
        "documentation-uninstall-evidence",
        "needs.release-identity.outputs.stage == 'rc'",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "release workflow is missing documentation/uninstall gate `{required}`"
            ));
        }
    }
    let script = fs::read_to_string(root.join("scripts/smoke_release_archive.sh"))
        .map_err(|error| format!("release archive smoke is missing: {error}"))?;
    for required in [
        "canisend.documentation-uninstall/v1",
        "exact-binary-match",
        "complete-notice-bundle",
        "workspace-retained",
        "no-publication",
    ] {
        if !script.contains(required) {
            return Err(format!(
                "release archive smoke is missing evidence field `{required}`"
            ));
        }
    }
    println!("documentation/uninstall policy: ok (5 same-RC-run records)");
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

fn verify_upgrade_qualification_evidence(
    from_tag: &str,
    to_tag: &str,
    directory: &Path,
) -> Result<UpgradeQualificationSummary, String> {
    let (from_version, from_stage) = parse_release_tag(from_tag)?;
    let (to_version, to_stage) = parse_release_tag(to_tag)?;
    if from_stage != ReleaseStage::Beta || to_stage != ReleaseStage::ReleaseCandidate {
        return Err("upgrade qualification requires a Beta-to-RC tag pair".to_owned());
    }
    if (from_version.major, from_version.minor, from_version.patch)
        != (to_version.major, to_version.minor, to_version.patch)
    {
        return Err("upgrade qualification tags must use the same release line".to_owned());
    }
    let expected = BTreeMap::from([
        (
            "upgrade-aarch64-apple-darwin.json",
            (
                "upgrade-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
            ),
        ),
        (
            "upgrade-x86_64-apple-darwin.json",
            (
                "upgrade-x86_64-apple-darwin",
                "x86_64-apple-darwin",
                "macos-15-intel",
            ),
        ),
        (
            "upgrade-x86_64-unknown-linux-gnu.json",
            (
                "upgrade-x86_64-unknown-linux-gnu",
                "x86_64-unknown-linux-gnu",
                "ubuntu-24.04",
            ),
        ),
        (
            "upgrade-x86_64-unknown-linux-musl.json",
            (
                "upgrade-x86_64-unknown-linux-musl",
                "x86_64-unknown-linux-musl",
                "ubuntu-24.04",
            ),
        ),
        (
            "upgrade-x86_64-pc-windows-msvc.json",
            (
                "upgrade-x86_64-pc-windows-msvc",
                "x86_64-pc-windows-msvc",
                "windows-2025",
            ),
        ),
    ]);
    let mut actual_paths = BTreeSet::new();
    collect_relative_files(directory, directory, &mut actual_paths)?;
    if actual_paths != expected.keys().map(|name| (*name).to_owned()).collect() {
        return Err(format!(
            "upgrade evidence file set differs: expected {:?}, found {actual_paths:?}",
            expected.keys().collect::<Vec<_>>()
        ));
    }

    let mut run_ids = BTreeSet::new();
    let mut from_manifests = BTreeSet::new();
    let mut to_manifests = BTreeSet::new();
    let mut from_archives = BTreeSet::new();
    let mut to_archives = BTreeSet::new();
    for (file, (record, target, environment)) in &expected {
        let path = directory.join(file);
        reject_symlink(&path)?;
        let value: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
            format!(
                "could not read upgrade evidence {}: {error}",
                path.display()
            )
        })?)
        .map_err(|error| format!("upgrade evidence `{file}` is invalid JSON: {error}"))?;
        let (run, from_manifest, to_manifest, from_archive, to_archive) =
            validate_upgrade_qualification_record(
                &value,
                record,
                target,
                environment,
                from_tag,
                to_tag,
            )?;
        run_ids.insert(run);
        from_manifests.insert(from_manifest);
        to_manifests.insert(to_manifest);
        from_archives.insert(from_archive);
        to_archives.insert(to_archive);
    }
    if run_ids.len() != 1 || from_manifests.len() != 1 || to_manifests.len() != 1 {
        return Err(
            "upgrade evidence records must bind one run and one shared release manifest pair"
                .to_owned(),
        );
    }
    if from_manifests == to_manifests {
        return Err("Beta and RC upgrade manifest digests must differ".to_owned());
    }
    if from_archives.len() != expected.len()
        || to_archives.len() != expected.len()
        || !from_archives.is_disjoint(&to_archives)
    {
        return Err(
            "upgrade evidence must bind distinct Beta and RC archives for all five targets"
                .to_owned(),
        );
    }
    let summary = UpgradeQualificationSummary {
        run_id: *run_ids.first().expect("one checked run ID"),
        from_manifest_sha256: from_manifests
            .first()
            .expect("one checked Beta manifest")
            .to_owned(),
        to_manifest_sha256: to_manifests
            .first()
            .expect("one checked RC manifest")
            .to_owned(),
        records: expected.len(),
    };
    println!(
        "upgrade evidence: ok ({from_tag} -> {to_tag}, run {}, {} targets)",
        summary.run_id, summary.records
    );
    Ok(summary)
}

#[allow(clippy::too_many_arguments)]
fn validate_upgrade_qualification_record(
    value: &Value,
    expected_record: &str,
    expected_target: &str,
    expected_environment: &str,
    from_tag: &str,
    to_tag: &str,
) -> Result<(u64, String, String, String, String), String> {
    let context = format!("upgrade evidence `{expected_record}`");
    let run_id = value["github_run_id"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| format!("{context} has no positive GitHub run ID"))?;
    let from_manifest = required_string(&value["manifests"], "from_sha256", &context)?.to_owned();
    let to_manifest = required_string(&value["manifests"], "to_sha256", &context)?.to_owned();
    let from_archive = required_string(&value["archives"], "from_sha256", &context)?.to_owned();
    let to_archive = required_string(&value["archives"], "to_sha256", &context)?.to_owned();
    for (name, digest) in [
        ("Beta manifest", &from_manifest),
        ("RC manifest", &to_manifest),
        ("Beta archive", &from_archive),
        ("RC archive", &to_archive),
    ] {
        validate_lower_hex(&format!("{context} {name} digest"), digest, 64)?;
    }
    if from_manifest == to_manifest || from_archive == to_archive {
        return Err(format!("{context} must bind distinct Beta and RC bytes"));
    }
    let before_schema = value["database_schemas"]["before"]
        .as_u64()
        .filter(|schema| *schema > 0 && *schema <= u64::from(u32::MAX))
        .ok_or_else(|| format!("{context} has an invalid pre-upgrade schema"))?;
    let after_schema = value["database_schemas"]["after"]
        .as_u64()
        .filter(|schema| *schema > 0 && *schema <= u64::from(u32::MAX))
        .ok_or_else(|| format!("{context} has an invalid post-upgrade schema"))?;
    let old_binary_behavior = required_string(value, "old_binary_behavior", &context)?;
    match old_binary_behavior {
        "same-schema-accepted" if before_schema == after_schema => {}
        "future-schema-rejected-without-mutation" if after_schema > before_schema => {}
        _ => {
            return Err(format!(
                "{context} old-binary behavior does not match the observed schemas"
            ));
        }
    }
    let completed_at = required_string(value, "completed_at", &context)?;
    if !completed_at.ends_with('Z') || OffsetDateTime::parse(completed_at, &Rfc3339).is_err() {
        return Err(format!("{context} completion timestamp must be valid UTC"));
    }
    let checks = value["checks"]
        .as_object()
        .ok_or_else(|| format!("{context} checks are missing"))?;
    let required_checks = [
        "verified-release-pair",
        "from-version-and-doctor",
        "workspace-created-and-checked",
        "verified-pre-upgrade-backup",
        "to-version-and-doctor",
        "workspace-upgraded-and-checked",
        "old-binary-behavior-verified",
        "backup-restored-to-new-path",
        "restored-workspace-checked-by-old-binary",
        "host-pack-regenerated",
        "installed-binary-and-notices-uninstalled",
        "workspace-backup-and-restore-retained",
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
        "schema": UPGRADE_QUALIFICATION_SCHEMA,
        "record": expected_record,
        "target": expected_target,
        "environment": expected_environment,
        "from_tag": from_tag,
        "to_tag": to_tag,
        "manifests": {
            "from_sha256": from_manifest,
            "to_sha256": to_manifest
        },
        "archives": {
            "from_sha256": from_archive,
            "to_sha256": to_archive
        },
        "github_run_id": run_id,
        "observed_versions": {
            "from": from_tag.trim_start_matches('v'),
            "to": to_tag.trim_start_matches('v')
        },
        "database_schemas": {
            "before": before_schema,
            "after": after_schema
        },
        "old_binary_behavior": old_binary_behavior,
        "checks": checks,
        "completed_at": completed_at
    });
    if *value != expected {
        return Err(format!(
            "{context} contains unknown, noncanonical, or mismatched fields"
        ));
    }
    Ok((run_id, from_manifest, to_manifest, from_archive, to_archive))
}

fn verify_documentation_uninstall_evidence(
    tag: &str,
    assets: &Path,
    directory: &Path,
) -> Result<DocumentationUninstallSummary, String> {
    let (version, stage) = parse_release_tag(tag)?;
    if stage != ReleaseStage::ReleaseCandidate {
        return Err("documentation/uninstall evidence requires an RC tag".to_owned());
    }
    verify_release(tag, assets)?;
    let manifest_path = assets.join(format!("canisend-{version}-manifest.json"));
    let manifest: Value = serde_json::from_slice(
        &fs::read(&manifest_path)
            .map_err(|error| format!("could not read verified RC release manifest: {error}"))?,
    )
    .map_err(|error| format!("verified RC release manifest is invalid JSON: {error}"))?;
    let artifacts = manifest["artifacts"]
        .as_array()
        .ok_or_else(|| "verified RC release manifest artifacts are missing".to_owned())?;
    let expected = BTreeMap::from([
        (
            "documentation-uninstall-aarch64-apple-darwin.json",
            (
                "documentation-uninstall-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
            ),
        ),
        (
            "documentation-uninstall-x86_64-apple-darwin.json",
            (
                "documentation-uninstall-x86_64-apple-darwin",
                "x86_64-apple-darwin",
                "macos-15-intel",
            ),
        ),
        (
            "documentation-uninstall-x86_64-unknown-linux-gnu.json",
            (
                "documentation-uninstall-x86_64-unknown-linux-gnu",
                "x86_64-unknown-linux-gnu",
                "ubuntu-24.04",
            ),
        ),
        (
            "documentation-uninstall-x86_64-unknown-linux-musl.json",
            (
                "documentation-uninstall-x86_64-unknown-linux-musl",
                "x86_64-unknown-linux-musl",
                "ubuntu-24.04",
            ),
        ),
        (
            "documentation-uninstall-x86_64-pc-windows-msvc.json",
            (
                "documentation-uninstall-x86_64-pc-windows-msvc",
                "x86_64-pc-windows-msvc",
                "windows-2025",
            ),
        ),
    ]);
    let mut actual_paths = BTreeSet::new();
    collect_relative_files(directory, directory, &mut actual_paths)?;
    if actual_paths != expected.keys().map(|name| (*name).to_owned()).collect() {
        return Err(format!(
            "documentation/uninstall evidence file set differs: expected {:?}, found {actual_paths:?}",
            expected.keys().collect::<Vec<_>>()
        ));
    }
    let mut run_ids = BTreeSet::new();
    let mut archive_digests = BTreeSet::new();
    for (file, (record, target, environment)) in &expected {
        let manifest_artifact = artifacts
            .iter()
            .find(|artifact| artifact["target"] == *target)
            .ok_or_else(|| format!("verified RC manifest has no `{target}` archive"))?;
        let expected_archive_sha =
            required_string(manifest_artifact, "sha256", "verified RC archive")?;
        let path = directory.join(file);
        reject_symlink(&path)?;
        let value: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
            format!(
                "could not read documentation/uninstall evidence {}: {error}",
                path.display()
            )
        })?)
        .map_err(|error| {
            format!("documentation/uninstall evidence `{file}` is invalid JSON: {error}")
        })?;
        let (run, archive_digest) = validate_documentation_uninstall_record(
            &value,
            record,
            target,
            environment,
            tag,
            expected_archive_sha,
        )?;
        run_ids.insert(run);
        archive_digests.insert(archive_digest);
    }
    if run_ids.len() != 1 || archive_digests.len() != expected.len() {
        return Err(
            "documentation/uninstall evidence must bind one run and five distinct verified archives"
                .to_owned(),
        );
    }
    let summary = DocumentationUninstallSummary {
        run_id: *run_ids.first().expect("one checked run ID"),
        records: expected.len(),
    };
    println!(
        "documentation/uninstall evidence: ok ({tag}, run {}, {} targets)",
        summary.run_id, summary.records
    );
    Ok(summary)
}

#[allow(clippy::too_many_arguments)]
fn validate_documentation_uninstall_record(
    value: &Value,
    expected_record: &str,
    expected_target: &str,
    expected_environment: &str,
    tag: &str,
    expected_archive_sha: &str,
) -> Result<(u64, String), String> {
    let context = format!("documentation/uninstall evidence `{expected_record}`");
    validate_lower_hex(
        &format!("{context} verified archive digest"),
        expected_archive_sha,
        64,
    )?;
    let archive_sha = required_string(value, "archive_sha256", &context)?.to_owned();
    validate_lower_hex(&format!("{context} archive digest"), &archive_sha, 64)?;
    if archive_sha != expected_archive_sha {
        return Err(format!(
            "{context} archive digest differs from the verified RC manifest"
        ));
    }
    let run_id = value["github_run_id"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| format!("{context} has no positive GitHub run ID"))?;
    let completed_at = required_string(value, "completed_at", &context)?;
    if !completed_at.ends_with('Z') || OffsetDateTime::parse(completed_at, &Rfc3339).is_err() {
        return Err(format!("{context} completion timestamp must be valid UTC"));
    }
    let checks = value["checks"]
        .as_object()
        .ok_or_else(|| format!("{context} checks are missing"))?;
    let required_checks = [
        "exact-binary-match",
        "complete-notice-bundle",
        "version-and-doctor",
        "documented-quickstart",
        "host-agent-smoke",
        "isolated-install",
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
        "schema": DOCUMENTATION_UNINSTALL_SCHEMA,
        "record": expected_record,
        "target": expected_target,
        "environment": expected_environment,
        "tag": tag,
        "archive_sha256": archive_sha,
        "github_run_id": run_id,
        "observed_version": tag.trim_start_matches('v'),
        "checks": checks,
        "completed_at": completed_at
    });
    if *value != expected {
        return Err(format!(
            "{context} contains unknown, noncanonical, or mismatched fields"
        ));
    }
    Ok((run_id, archive_sha))
}

fn verify_package_candidate_pair(
    from_tag: &str,
    from_assets: &Path,
    to_tag: &str,
    to_assets: &Path,
) -> Result<(), String> {
    let (from_version, from_stage) = parse_release_tag(from_tag)?;
    let (to_version, to_stage) = parse_release_tag(to_tag)?;
    if from_stage != ReleaseStage::Beta || to_stage != ReleaseStage::ReleaseCandidate {
        return Err("package-manager candidates require a Beta-to-RC tag pair".to_owned());
    }
    if (from_version.major, from_version.minor, from_version.patch)
        != (to_version.major, to_version.minor, to_version.patch)
    {
        return Err("package-manager candidate tags must use the same release line".to_owned());
    }
    verify_release(from_tag, from_assets)?;
    verify_release(to_tag, to_assets)?;

    let root = repository_root().join("packaging/candidates");
    let from_source = check_channel_candidate_directory(&root.join(from_tag))?;
    let to_source = check_channel_candidate_directory(&root.join(to_tag))?;
    validate_package_candidate_source_against_assets(
        &from_source,
        from_tag,
        from_stage,
        from_assets,
    )?;
    validate_package_candidate_source_against_assets(&to_source, to_tag, to_stage, to_assets)?;
    if from_source.source_commit == to_source.source_commit
        || from_source.manifest_sha256 == to_source.manifest_sha256
    {
        return Err("Beta and RC candidates must bind distinct release sources".to_owned());
    }
    println!("package-manager candidates: ok ({from_tag} -> {to_tag})");
    Ok(())
}

fn validate_package_candidate_source_against_assets(
    source: &ChannelCandidateSource,
    expected_tag: &str,
    expected_stage: ReleaseStage,
    assets: &Path,
) -> Result<(), String> {
    if source.tag != expected_tag || source.stage != expected_stage {
        return Err(format!(
            "package-manager candidate `{expected_tag}` has the wrong release identity"
        ));
    }
    let manifest_path = assets.join(&source.manifest_file);
    if sha256_file(&manifest_path)? != source.manifest_sha256 {
        return Err(format!(
            "package-manager candidate `{expected_tag}` does not bind the verified public manifest"
        ));
    }
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| {
        format!(
            "could not read package-manager source manifest {}: {error}",
            manifest_path.display()
        )
    })?)
    .map_err(|error| format!("package-manager source manifest is invalid JSON: {error}"))?;
    if manifest["tag"] != expected_tag
        || manifest["version"] != source.version
        || manifest["stage"] != expected_stage.as_str()
        || manifest["source"]["commit"] != source.source_commit
    {
        return Err(format!(
            "package-manager candidate `{expected_tag}` differs from its verified release manifest"
        ));
    }
    let manifest_artifacts = manifest["artifacts"]
        .as_array()
        .ok_or_else(|| "verified release manifest artifacts are missing".to_owned())?;
    for (target, source_artifact) in &source.artifacts {
        let manifest_artifact = manifest_artifacts
            .iter()
            .find(|artifact| artifact["target"] == target.as_str())
            .ok_or_else(|| format!("verified release manifest has no `{target}` artifact"))?;
        if manifest_artifact["archive"] != source_artifact.archive
            || manifest_artifact["sha256"] != source_artifact.sha256
            || manifest_artifact["size"] != source_artifact.size
        {
            return Err(format!(
                "package-manager candidate artifact `{target}` differs from the verified release"
            ));
        }
    }
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
    let (parsed_version, stage) = parse_release_tag(tag)?;
    let version = parsed_version.to_string();
    println!("release tag: ok ({tag}, stage {})", stage.as_str());
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
    verify_release_manifest_contents(stage, &version, directory, &manifest)?;
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

    fn sample_upgrade_evidence(record: &str, target: &str, environment: &str) -> Value {
        json!({
            "schema": UPGRADE_QUALIFICATION_SCHEMA,
            "record": record,
            "target": target,
            "environment": environment,
            "from_tag": "v0.7.0-beta.1",
            "to_tag": "v0.7.0-rc.1",
            "manifests": {
                "from_sha256": "a".repeat(64),
                "to_sha256": "b".repeat(64)
            },
            "archives": {
                "from_sha256": sha256(format!("beta-{target}").as_bytes()),
                "to_sha256": sha256(format!("rc-{target}").as_bytes())
            },
            "github_run_id": 29_650_000_001_u64,
            "observed_versions": {
                "from": "0.7.0-beta.1",
                "to": "0.7.0-rc.1"
            },
            "database_schemas": {"before": 13, "after": 13},
            "old_binary_behavior": "same-schema-accepted",
            "checks": {
                "verified-release-pair": true,
                "from-version-and-doctor": true,
                "workspace-created-and-checked": true,
                "verified-pre-upgrade-backup": true,
                "to-version-and-doctor": true,
                "workspace-upgraded-and-checked": true,
                "old-binary-behavior-verified": true,
                "backup-restored-to-new-path": true,
                "restored-workspace-checked-by-old-binary": true,
                "host-pack-regenerated": true,
                "installed-binary-and-notices-uninstalled": true,
                "workspace-backup-and-restore-retained": true,
                "no-publication": true
            },
            "completed_at": "2026-07-18T12:00:00Z"
        })
    }

    fn sample_documentation_uninstall_evidence(
        record: &str,
        target: &str,
        environment: &str,
        archive_sha256: &str,
    ) -> Value {
        json!({
            "schema": DOCUMENTATION_UNINSTALL_SCHEMA,
            "record": record,
            "target": target,
            "environment": environment,
            "tag": "v0.7.0-rc.1",
            "archive_sha256": archive_sha256,
            "github_run_id": 29_660_000_001_u64,
            "observed_version": "0.7.0-rc.1",
            "checks": {
                "exact-binary-match": true,
                "complete-notice-bundle": true,
                "version-and-doctor": true,
                "documented-quickstart": true,
                "host-agent-smoke": true,
                "isolated-install": true,
                "uninstall": true,
                "workspace-retained": true,
                "no-publication": true
            },
            "completed_at": "2026-07-18T13:00:00Z"
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
        assert_eq!(
            parse_release_tag("v0.7.0-beta.1")
                .expect("historical Beta tag")
                .1,
            ReleaseStage::Beta
        );
        assert_eq!(
            parse_release_tag("v0.7.0-rc.1")
                .expect("historical RC tag")
                .1,
            ReleaseStage::ReleaseCandidate
        );
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
    fn release_notes_are_stage_neutral_and_heading_only_transitions() {
        check_release_notes_policy().expect("release notes policy");
        let root = repository_root();
        let notes =
            fs::read_to_string(root.join("release/RELEASE_NOTES.md")).expect("read release notes");
        let transitioned = replace_exact_count(
            &notes,
            "# CanISend 0.7.0-alpha.1",
            "# CanISend 0.7.0-beta.1",
            1,
            "test release-note heading",
        )
        .expect("transition release-note heading");
        assert_eq!(
            notes.split_once('\n').expect("Alpha notes body").1,
            transitioned.split_once('\n').expect("Beta notes body").1
        );

        let stale = notes.replace(
            "CanISend 0.7 is a greenfield Rust-native release.",
            "The alpha is a greenfield Rust-native release.",
        );
        let sections = [
            "Highlights",
            "Compatibility",
            "Install and verify",
            "Upgrade and rollback",
            "Security and privacy",
            "Known limitations",
            "Feedback and support",
        ];
        let guidance = [
            "does not require Python",
            "canisend.workspace/v2",
            "canisend.agent/v2",
            "never submits an application",
            "SHA256SUMS",
            "GitHub build provenance",
            "back up every important workspace",
            "restore the pre-upgrade backup into a new directory",
            "no in-place database downgrade",
            "no telemetry",
            "KNOWN_LIMITATIONS.md",
            "Never attach a workspace",
        ];
        let guides = [
            "docs/guides/release-verification.md",
            "docs/guides/quick-start.md",
            "docs/guides/upgrade-and-rollback.md",
        ];
        assert!(
            validate_release_notes(
                &root,
                &Version::parse("0.7.0-alpha.1").expect("Alpha version"),
                &stale,
                &sections,
                &guidance,
                &guides,
            )
            .is_err()
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
    fn beta_qualification_promotion_requires_canonical_pending_state() {
        let pending = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "beta",
            "status": "beta-qualifying",
            "stable_authorized": false,
            "feature_freeze": {"status": "planned", "baseline_commit": null},
            "beta": {
                "signed_matrix_run": null,
                "signing_evidence_targets": [],
                "source_commit": null,
                "status": "pending",
                "tag": null
            }
        });
        let source = "7".repeat(40);
        let qualified = beta_qualified_ledger(&pending, "v0.7.0-beta.1", 29_640_000_001, &source)
            .expect("qualify canonical Beta ledger");
        assert_eq!(qualified["beta"]["status"], "qualified");
        assert_eq!(qualified["beta"]["tag"], "v0.7.0-beta.1");
        assert_eq!(qualified["beta"]["source_commit"], source);
        assert_eq!(
            qualified["beta"]["signing_evidence_targets"],
            json!([
                "aarch64-apple-darwin",
                "x86_64-apple-darwin",
                "x86_64-pc-windows-msvc"
            ])
        );
        assert!(
            beta_qualified_ledger(&qualified, "v0.7.0-beta.1", 29_640_000_001, &source).is_err()
        );
        assert!(beta_qualified_ledger(&pending, "v0.7.0-alpha.1", 1, &source).is_err());
        assert!(beta_qualified_ledger(&pending, "v0.7.0-beta.1", 0, &source).is_err());
    }

    #[test]
    fn rc_qualification_records_two_distinct_clean_tag_matrices() {
        let ledger = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "rc",
            "status": "rc-qualifying",
            "stable_authorized": false,
            "beta": {"status": "qualified"},
            "feature_freeze": {"status": "frozen", "baseline_commit": "6".repeat(40)},
            "release_candidates": []
        });
        let first_source = "7".repeat(40);
        let second_source = "8".repeat(40);
        let first = rc_qualified_ledger(&ledger, "v0.7.0-rc.1", 29_641_000_001, &first_source)
            .expect("record first RC matrix");
        let second = rc_qualified_ledger(&first, "v0.7.0-rc.2", 29_641_000_002, &second_source)
            .expect("record second RC matrix");
        assert_eq!(
            second["release_candidates"].as_array().map(Vec::len),
            Some(2)
        );
        assert_eq!(
            second["release_candidates"][1],
            json!({
                "signed_matrix_run": 29_641_000_002_u64,
                "source_commit": second_source,
                "status": "success",
                "tag": "v0.7.0-rc.2"
            })
        );
        assert!(
            rc_qualified_ledger(&first, "v0.7.0-rc.1", 29_641_000_002, &second_source).is_err()
        );
        assert!(
            rc_qualified_ledger(&first, "v0.7.0-rc.2", 29_641_000_001, &second_source).is_err()
        );
        assert!(rc_qualified_ledger(&first, "v0.7.0-rc.2", 29_641_000_002, &first_source).is_err());
        assert!(
            rc_qualified_ledger(&ledger, "v0.7.0-beta.1", 29_641_000_001, &first_source).is_err()
        );
    }

    #[test]
    fn stage_transition_policy_is_forward_only_and_dry_run_by_default() {
        check_stage_transition_policy().expect("stage-transition policy");
        let alpha = Version::parse("0.7.0-alpha.1").expect("Alpha version");
        let beta = Version::parse("0.7.0-beta.1").expect("Beta version");
        let beta_two = Version::parse("0.7.0-beta.2").expect("second Beta version");
        let rc = Version::parse("0.7.0-rc.1").expect("RC version");
        let rc_two = Version::parse("0.7.0-rc.2").expect("second RC version");
        let rc_three = Version::parse("0.7.0-rc.3").expect("third RC version");
        validate_stage_transition(&alpha, ReleaseStage::Alpha, &beta, ReleaseStage::Beta)
            .expect("Alpha to first Beta");
        assert!(
            validate_stage_transition(&alpha, ReleaseStage::Alpha, &beta_two, ReleaseStage::Beta)
                .is_err()
        );
        assert!(
            validate_stage_transition(
                &alpha,
                ReleaseStage::Alpha,
                &rc,
                ReleaseStage::ReleaseCandidate
            )
            .is_err()
        );
        validate_stage_transition(
            &rc,
            ReleaseStage::ReleaseCandidate,
            &rc_two,
            ReleaseStage::ReleaseCandidate,
        )
        .expect("sequential RC iteration");
        assert!(
            validate_stage_transition(
                &rc,
                ReleaseStage::ReleaseCandidate,
                &rc_three,
                ReleaseStage::ReleaseCandidate
            )
            .is_err()
        );
        assert!(
            validate_stage_transition(&beta, ReleaseStage::Beta, &beta_two, ReleaseStage::Beta)
                .is_err()
        );
    }

    #[test]
    fn beta_transition_requires_a_recent_nonfuture_readiness_audit() {
        let root = std::env::temp_dir().join(format!(
            "canisend-beta-readiness-age-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale readiness-age fixture");
        }
        fs::create_dir_all(root.join("release")).expect("create readiness-age fixture");
        let now = OffsetDateTime::parse("2026-07-18T12:00:00Z", &Rfc3339).expect("fixture time");
        for (audited_at, accepted) in [
            ("2026-07-17T13:00:00Z", true),
            ("2026-07-17T11:00:00Z", false),
            ("2026-07-18T12:06:00Z", false),
        ] {
            write_pretty_json(
                &root.join("release/beta-readiness.json"),
                &json!({"audited_at": audited_at}),
            )
            .expect("write readiness-age fixture");
            assert_eq!(
                check_beta_readiness_freshness(&root, now).is_ok(),
                accepted,
                "unexpected freshness result for {audited_at}"
            );
        }
        fs::remove_dir_all(root).expect("remove readiness-age fixture");
    }

    #[test]
    fn stage_transition_changes_only_controlled_current_state() {
        let root =
            std::env::temp_dir().join(format!("canisend-stage-transition-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale transition fixture");
        }
        fs::create_dir_all(root.join("crates/app")).expect("create app fixture");
        fs::create_dir_all(root.join("crates/contracts")).expect("create contracts fixture");
        fs::create_dir_all(root.join("release")).expect("create release fixture");
        fs::create_dir_all(root.join("packaging/candidates/alpha"))
            .expect("create historical candidate fixture");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\", \"crates/contracts\"]\n\
             [workspace.package]\nversion = \"0.7.0-alpha.1\"\n",
        )
        .expect("write workspace fixture");
        fs::write(
            root.join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion.workspace = true\n\
             [dependencies]\ncontracts = { package = \"contracts\", path = \"../contracts\", version = \"=0.7.0-alpha.1\" }\n",
        )
        .expect("write app fixture");
        fs::write(
            root.join("crates/contracts/Cargo.toml"),
            "[package]\nname = \"contracts\"\nversion.workspace = true\n",
        )
        .expect("write contracts fixture");
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.7.0-alpha.1\"\n\
             dependencies = [\"contracts\"]\n\n[[package]]\nname = \"contracts\"\nversion = \"0.7.0-alpha.1\"\n",
        )
        .expect("write lock fixture");
        write_pretty_json(
            &root.join("release/qualification-ledger.json"),
            &json!({
                "schema": RELEASE_QUALIFICATION_SCHEMA,
                "workspace_stage": "alpha",
                "status": "pre-beta",
                "stable_authorized": false,
                "beta": {"status": "pending"},
                "feature_freeze": {"status": "planned"},
                "release_notes": {"status": "alpha-current"}
            }),
        )
        .expect("write qualification fixture");
        fs::write(
            root.join("release/RELEASE_NOTES.md"),
            "# CanISend 0.7.0-alpha.1\n\nFixture notes.\n",
        )
        .expect("write notes fixture");
        for relative in [
            "release/beta-readiness.json",
            "release/beta-contract-freeze.json",
            "release/feedback-snapshot.json",
            "packaging/candidates/alpha/candidate-source.json",
        ] {
            fs::write(root.join(relative), b"historical 0.7.0-alpha.1\n")
                .expect("write historical fixture");
        }

        let workspace_before = fs::read(root.join("Cargo.toml")).expect("read workspace before");
        let transition = render_stage_transition(&root, "v0.7.0-beta.1")
            .expect("render Alpha to Beta transition");
        assert_eq!(
            stage_transition_report(&root, &transition, false).expect("dry-run report")["writes_performed"],
            false
        );
        assert_eq!(
            fs::read(root.join("Cargo.toml")).expect("read workspace after dry run"),
            workspace_before
        );
        assert_eq!(transition.files.len(), 5);
        for (relative, body) in &transition.files {
            fs::write(root.join(relative), body).expect("apply rendered transition fixture");
        }
        assert!(
            fs::read_to_string(root.join("Cargo.toml"))
                .expect("read transitioned workspace")
                .contains("version = \"0.7.0-beta.1\"")
        );
        assert!(
            fs::read_to_string(root.join("crates/app/Cargo.toml"))
                .expect("read transitioned app")
                .contains("version = \"=0.7.0-beta.1\"")
        );
        let ledger: Value = serde_json::from_slice(
            &fs::read(root.join("release/qualification-ledger.json"))
                .expect("read transitioned ledger"),
        )
        .expect("parse transitioned ledger");
        assert_eq!(ledger["workspace_stage"], "beta");
        assert_eq!(ledger["status"], "beta-qualifying");
        assert_eq!(ledger["release_notes"]["status"], "beta-current");
        for relative in [
            "release/beta-readiness.json",
            "release/beta-contract-freeze.json",
            "release/feedback-snapshot.json",
            "packaging/candidates/alpha/candidate-source.json",
        ] {
            assert_eq!(
                fs::read_to_string(root.join(relative)).expect("read historical fixture"),
                "historical 0.7.0-alpha.1\n"
            );
        }
        fs::remove_dir_all(root).expect("remove transition fixture");
    }

    #[test]
    fn rc_iteration_preserves_existing_qualification_evidence() {
        let root =
            std::env::temp_dir().join(format!("canisend-rc-iteration-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale RC fixture");
        }
        fs::create_dir_all(root.join("crates/app")).expect("create RC app fixture");
        fs::create_dir_all(root.join("crates/contracts")).expect("create RC contracts fixture");
        fs::create_dir_all(root.join("release")).expect("create RC release fixture");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\", \"crates/contracts\"]\n\
             [workspace.package]\nversion = \"0.7.0-rc.1\"\n",
        )
        .expect("write RC workspace fixture");
        fs::write(
            root.join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion.workspace = true\n\
             [dependencies]\ncontracts = { package = \"contracts\", path = \"../contracts\", version = \"=0.7.0-rc.1\" }\n",
        )
        .expect("write RC app fixture");
        fs::write(
            root.join("crates/contracts/Cargo.toml"),
            "[package]\nname = \"contracts\"\nversion.workspace = true\n",
        )
        .expect("write RC contracts fixture");
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.7.0-rc.1\"\n\
             dependencies = [\"contracts\"]\n\n[[package]]\nname = \"contracts\"\nversion = \"0.7.0-rc.1\"\n",
        )
        .expect("write RC lock fixture");
        let ledger = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "rc",
            "status": "rc-qualifying",
            "stable_authorized": false,
            "beta": {"status": "qualified", "tag": "v0.7.0-beta.1"},
            "feature_freeze": {"status": "frozen", "baseline_commit": "7".repeat(40)},
            "release_notes": {"status": "rc-final"},
            "release_candidates": [{"tag": "v0.7.0-rc.1", "status": "success"}]
        });
        write_pretty_json(&root.join("release/qualification-ledger.json"), &ledger)
            .expect("write RC ledger fixture");
        fs::write(
            root.join("release/RELEASE_NOTES.md"),
            "# CanISend 0.7.0-rc.1\n\nFixture notes.\n",
        )
        .expect("write RC notes fixture");

        let ledger_before = fs::read(root.join("release/qualification-ledger.json"))
            .expect("read RC ledger before iteration");
        let transition =
            render_stage_transition(&root, "v0.7.0-rc.2").expect("render sequential RC iteration");
        assert!(
            !transition
                .files
                .contains_key("release/qualification-ledger.json")
        );
        assert_eq!(transition.files.len(), 4);
        for (relative, body) in &transition.files {
            fs::write(root.join(relative), body).expect("apply RC iteration fixture");
        }
        assert_eq!(
            fs::read(root.join("release/qualification-ledger.json"))
                .expect("read RC ledger after iteration"),
            ledger_before
        );
        assert!(
            fs::read_to_string(root.join("Cargo.toml"))
                .expect("read iterated RC workspace")
                .contains("version = \"0.7.0-rc.2\"")
        );
        fs::remove_dir_all(root).expect("remove RC fixture");
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
    fn planned_feature_freeze_cannot_preapprove_source_changes() {
        let freeze = json!({"status": "planned", "baseline_commit": null});
        check_feature_freeze_exceptions(&freeze).expect("planned feature freeze");
        assert!(is_automatic_feature_freeze_path(
            "docs/release/feature-freeze.md"
        ));
        assert!(is_automatic_feature_freeze_path(
            "release/qualification-ledger.json"
        ));
        assert!(!is_automatic_feature_freeze_path(
            "crates/canisend-store/src/lib.rs"
        ));
        assert!(!is_automatic_feature_freeze_path(
            ".github/workflows/release.yml"
        ));
    }

    #[test]
    fn feature_freeze_activation_is_head_bound_and_two_file_only() {
        let root = std::env::temp_dir().join(format!(
            "canisend-feature-freeze-activation-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale activation fixture");
        }
        fs::create_dir_all(root.join("release")).expect("create activation fixture");
        run_git(&root, &["init", "--initial-branch=main"])
            .expect("initialize activation repository");
        run_git(&root, &["config", "user.name", "CanISend qualification"])
            .expect("configure activation name");
        run_git(
            &root,
            &["config", "user.email", "qualification@canisend.invalid"],
        )
        .expect("configure activation email");
        fs::write(root.join("README.md"), "fixture\n").expect("write initial fixture");
        run_git(&root, &["add", "README.md"]).expect("stage initial fixture");
        run_git(&root, &["commit", "-m", "initialize fixture"]).expect("commit initial fixture");
        write_pretty_json(
            &root.join("release/qualification-ledger.json"),
            &json!({
                "schema": RELEASE_QUALIFICATION_SCHEMA,
                "workspace_stage": "beta",
                "status": "beta-qualifying",
                "stable_authorized": false,
                "beta": {"status": "qualified"},
                "feature_freeze": {"status": "planned", "baseline_commit": null}
            }),
        )
        .expect("write activation ledger");
        write_pretty_json(
            &root.join("release/feature-freeze-exceptions.json"),
            &json!({
                "schema": FEATURE_FREEZE_EXCEPTIONS_SCHEMA,
                "status": "planned",
                "baseline_commit": null,
                "exceptions": []
            }),
        )
        .expect("write activation exceptions");
        run_git(&root, &["add", "release"]).expect("stage Beta qualification fixture");
        run_git(&root, &["commit", "-m", "qualify beta"])
            .expect("commit Beta qualification fixture");
        let baseline = run_git_lines(&root, &["rev-parse", "HEAD"])
            .expect("read activation baseline")
            .pop()
            .expect("activation baseline");
        let parent = run_git_lines(&root, &["rev-parse", "HEAD^"])
            .expect("read activation parent")
            .pop()
            .expect("activation parent");
        assert!(render_feature_freeze_activation(&root, &parent).is_err());

        let before = fs::read(root.join("release/qualification-ledger.json"))
            .expect("read ledger before activation");
        let freeze = render_feature_freeze_activation(&root, &baseline)
            .expect("render feature-freeze activation");
        assert_eq!(freeze.files.len(), 2);
        assert_eq!(
            feature_freeze_report(&root, &freeze, false).expect("activation dry-run report")["writes_performed"],
            false
        );
        assert_eq!(
            fs::read(root.join("release/qualification-ledger.json"))
                .expect("read ledger after activation dry run"),
            before
        );
        for (relative, body) in &freeze.files {
            fs::write(root.join(relative), body).expect("apply activation fixture");
        }
        run_git(&root, &["add", "release"]).expect("stage activation fixture");
        run_git(&root, &["commit", "-m", "activate feature freeze"])
            .expect("commit activation fixture");
        validate_feature_freeze_history(&root, &baseline, &[])
            .expect("automatic activation history");
        fs::remove_dir_all(root).expect("remove activation fixture");
    }

    #[test]
    fn frozen_feature_history_requires_exact_commit_paths() {
        let root = std::env::temp_dir().join(format!(
            "canisend-feature-freeze-history-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale feature-freeze fixture");
        }
        fs::create_dir_all(root.join("crates")).expect("create source fixture directory");
        run_git(&root, &["init", "--initial-branch=main"]).expect("initialize fixture repository");
        run_git(&root, &["config", "user.name", "CanISend qualification"])
            .expect("configure fixture name");
        run_git(
            &root,
            &["config", "user.email", "qualification@canisend.invalid"],
        )
        .expect("configure fixture email");
        fs::write(root.join("crates/core.txt"), "baseline\n").expect("write baseline source");
        run_git(&root, &["add", "crates/core.txt"]).expect("stage baseline source");
        run_git(&root, &["commit", "-m", "baseline"]).expect("commit baseline source");
        let baseline = run_git_lines(&root, &["rev-parse", "HEAD"])
            .expect("read baseline commit")
            .pop()
            .expect("baseline commit");

        fs::create_dir_all(root.join("docs")).expect("create docs fixture directory");
        fs::write(root.join("docs/note.md"), "automatic documentation\n")
            .expect("write documentation fixture");
        run_git(&root, &["add", "docs/note.md"]).expect("stage documentation fixture");
        run_git(&root, &["commit", "-m", "document release"])
            .expect("commit documentation fixture");

        fs::write(root.join("crates/core.txt"), "release blocker fix\n")
            .expect("write blocker fixture");
        run_git(&root, &["add", "crates/core.txt"]).expect("stage blocker fixture");
        run_git(&root, &["commit", "-m", "fix release blocker"]).expect("commit blocker fixture");
        let blocker = run_git_lines(&root, &["rev-parse", "HEAD"])
            .expect("read blocker commit")
            .pop()
            .expect("blocker commit");

        let exceptions = vec![json!({
            "commit": blocker,
            "class": "release-blocker",
            "reason": "Correct the owned release implementation before RC qualification.",
            "paths": ["crates/core.txt"]
        })];
        validate_feature_freeze_history(&root, &baseline, &exceptions)
            .expect("exact feature-freeze history");

        let mut wrong = exceptions;
        wrong[0]["paths"] = json!(["crates/other.txt"]);
        assert!(validate_feature_freeze_history(&root, &baseline, &wrong).is_err());
        fs::remove_dir_all(root).expect("remove feature-freeze fixture");
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
    fn upgrade_qualification_policy_is_five_target_and_nonpublishing() {
        check_upgrade_qualification_policy().expect("upgrade qualification policy");
    }

    #[test]
    fn upgrade_evidence_requires_canonical_old_binary_behavior_and_checks() {
        let mut evidence = sample_upgrade_evidence(
            "upgrade-aarch64-apple-darwin",
            "aarch64-apple-darwin",
            "macos-15",
        );
        validate_upgrade_qualification_record(
            &evidence,
            "upgrade-aarch64-apple-darwin",
            "aarch64-apple-darwin",
            "macos-15",
            "v0.7.0-beta.1",
            "v0.7.0-rc.1",
        )
        .expect("canonical upgrade evidence");

        evidence["checks"]["no-publication"] = Value::Bool(false);
        assert!(
            validate_upgrade_qualification_record(
                &evidence,
                "upgrade-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
                "v0.7.0-beta.1",
                "v0.7.0-rc.1",
            )
            .is_err()
        );

        let mut impossible = sample_upgrade_evidence(
            "upgrade-aarch64-apple-darwin",
            "aarch64-apple-darwin",
            "macos-15",
        );
        impossible["old_binary_behavior"] =
            Value::String("future-schema-rejected-without-mutation".to_owned());
        assert!(
            validate_upgrade_qualification_record(
                &impossible,
                "upgrade-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
                "v0.7.0-beta.1",
                "v0.7.0-rc.1",
            )
            .is_err()
        );
    }

    #[test]
    fn upgrade_evidence_directory_binds_one_five_target_run() {
        let root =
            std::env::temp_dir().join(format!("canisend-upgrade-evidence-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale upgrade evidence fixture");
        }
        fs::create_dir_all(&root).expect("create upgrade evidence fixture");
        for (file, record, target, environment) in [
            (
                "upgrade-aarch64-apple-darwin.json",
                "upgrade-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
            ),
            (
                "upgrade-x86_64-apple-darwin.json",
                "upgrade-x86_64-apple-darwin",
                "x86_64-apple-darwin",
                "macos-15-intel",
            ),
            (
                "upgrade-x86_64-unknown-linux-gnu.json",
                "upgrade-x86_64-unknown-linux-gnu",
                "x86_64-unknown-linux-gnu",
                "ubuntu-24.04",
            ),
            (
                "upgrade-x86_64-unknown-linux-musl.json",
                "upgrade-x86_64-unknown-linux-musl",
                "x86_64-unknown-linux-musl",
                "ubuntu-24.04",
            ),
            (
                "upgrade-x86_64-pc-windows-msvc.json",
                "upgrade-x86_64-pc-windows-msvc",
                "x86_64-pc-windows-msvc",
                "windows-2025",
            ),
        ] {
            write_pretty_json(
                &root.join(file),
                &sample_upgrade_evidence(record, target, environment),
            )
            .expect("write upgrade evidence fixture");
        }
        let summary = verify_upgrade_qualification_evidence("v0.7.0-beta.1", "v0.7.0-rc.1", &root)
            .expect("verify five target upgrade evidence");
        assert_eq!(summary.records, 5);

        fs::write(root.join("extra.json"), b"{}\n").expect("write extra evidence fixture");
        assert!(
            verify_upgrade_qualification_evidence("v0.7.0-beta.1", "v0.7.0-rc.1", &root,).is_err()
        );
        fs::remove_dir_all(root).expect("remove upgrade evidence fixture");
    }

    #[test]
    fn upgrade_ledger_promotion_requires_recorded_exact_rc() {
        let ledger = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "rc",
            "status": "rc-qualifying",
            "beta": {"status": "qualified", "tag": "v0.7.0-beta.1"},
            "feature_freeze": {"status": "frozen"},
            "release_candidates": [
                {"status": "success", "tag": "v0.7.0-rc.1"}
            ],
            "stable_authorized": false,
            "upgrade_matrix": {
                "beta_tag": null,
                "evidence": [],
                "rc_tag": null,
                "status": "pending"
            }
        });
        let summary = UpgradeQualificationSummary {
            run_id: 29_650_000_001,
            from_manifest_sha256: "a".repeat(64),
            to_manifest_sha256: "b".repeat(64),
            records: 5,
        };
        let qualified = upgrade_qualified_ledger(&ledger, "v0.7.0-beta.1", "v0.7.0-rc.1", &summary)
            .expect("promote exact RC upgrade evidence");
        assert_eq!(qualified["upgrade_matrix"]["status"], "passed");

        assert!(
            upgrade_qualified_ledger(&ledger, "v0.7.0-beta.1", "v0.7.0-rc.2", &summary,).is_err()
        );
    }

    #[test]
    fn documentation_uninstall_policy_is_same_rc_run_and_five_target() {
        check_documentation_uninstall_policy().expect("documentation/uninstall policy");
    }

    #[test]
    fn documentation_uninstall_record_binds_manifest_archive_and_checks() {
        let digest = "d".repeat(64);
        let mut evidence = sample_documentation_uninstall_evidence(
            "documentation-uninstall-aarch64-apple-darwin",
            "aarch64-apple-darwin",
            "macos-15",
            &digest,
        );
        validate_documentation_uninstall_record(
            &evidence,
            "documentation-uninstall-aarch64-apple-darwin",
            "aarch64-apple-darwin",
            "macos-15",
            "v0.7.0-rc.1",
            &digest,
        )
        .expect("canonical documentation/uninstall record");

        evidence["checks"]["workspace-retained"] = Value::Bool(false);
        assert!(
            validate_documentation_uninstall_record(
                &evidence,
                "documentation-uninstall-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
                "v0.7.0-rc.1",
                &digest,
            )
            .is_err()
        );
        let wrong = sample_documentation_uninstall_evidence(
            "documentation-uninstall-aarch64-apple-darwin",
            "aarch64-apple-darwin",
            "macos-15",
            &digest,
        );
        assert!(
            validate_documentation_uninstall_record(
                &wrong,
                "documentation-uninstall-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
                "v0.7.0-rc.1",
                &"e".repeat(64),
            )
            .is_err()
        );
    }

    #[test]
    fn documentation_uninstall_promotion_requires_same_recorded_rc_run() {
        let ledger = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "rc",
            "status": "rc-qualifying",
            "beta": {"status": "qualified"},
            "feature_freeze": {"status": "frozen"},
            "release_candidates": [{
                "tag": "v0.7.0-rc.1",
                "source_commit": "c".repeat(40),
                "signed_matrix_run": 29_660_000_001_u64,
                "status": "success"
            }],
            "documentation_uninstall": {
                "evidence": ["five-target Alpha preparation"],
                "native_matrix_run": 29_637_471_699_u64,
                "status": "prepared-native"
            },
            "stable_authorized": false
        });
        let summary = DocumentationUninstallSummary {
            run_id: 29_660_000_001,
            records: 5,
        };
        let qualified = documentation_uninstall_qualified_ledger(&ledger, "v0.7.0-rc.1", summary)
            .expect("qualify same RC run documentation evidence");
        assert_eq!(qualified["documentation_uninstall"]["status"], "passed");
        assert_eq!(
            qualified["documentation_uninstall"]["native_matrix_run"],
            summary.run_id
        );

        let wrong_run = DocumentationUninstallSummary {
            run_id: summary.run_id + 1,
            records: 5,
        };
        assert!(
            documentation_uninstall_qualified_ledger(&ledger, "v0.7.0-rc.1", wrong_run,).is_err()
        );
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
    fn package_candidate_source_binds_verified_manifest_bytes() {
        let root = std::env::temp_dir().join(format!(
            "canisend-package-candidate-source-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale package source fixture");
        }
        fs::create_dir_all(&root).expect("create package source fixture");
        let mut source = sample_channel_source();
        let artifacts = source
            .artifacts
            .values()
            .map(|artifact| {
                json!({
                    "target": artifact.target,
                    "archive": artifact.archive,
                    "sha256": artifact.sha256,
                    "size": artifact.size
                })
            })
            .collect::<Vec<_>>();
        let manifest = json!({
            "tag": source.tag,
            "version": source.version,
            "stage": source.stage.as_str(),
            "source": {"commit": source.source_commit},
            "artifacts": artifacts
        });
        let manifest_path = root.join(&source.manifest_file);
        write_pretty_json(&manifest_path, &manifest).expect("write package source manifest");
        source.manifest_sha256 = sha256_file(&manifest_path).expect("manifest fixture hash");
        validate_package_candidate_source_against_assets(
            &source,
            NATIVE_ALPHA_TAG,
            ReleaseStage::Alpha,
            &root,
        )
        .expect("candidate source binding");

        source.manifest_sha256 = "0".repeat(64);
        assert!(
            validate_package_candidate_source_against_assets(
                &source,
                NATIVE_ALPHA_TAG,
                ReleaseStage::Alpha,
                &root,
            )
            .is_err()
        );
        fs::remove_dir_all(root).expect("remove package source fixture");
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
