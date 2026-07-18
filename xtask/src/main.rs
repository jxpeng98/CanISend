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
            check_internal_dependency_versions()?;
            check_beta_readiness()?;
            check_beta_contract_freeze()?;
            check_channel_candidates()?;
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
        _ => Err(
            "usage: cargo run -p xtask -- schemas <check|write> | <resources|docs> check | \
             release <check|freeze-candidate|validate-tag TAG|sbom OUTPUT|assemble TAG COMMIT ARTIFACTS OUTPUT|verify TAG DIRECTORY|channels TAG ASSETS OUTPUT>"
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
        "troubleshooting.md",
    ];
    for file_name in required {
        let path = guide_root.join(file_name);
        let body = fs::read_to_string(&path)
            .map_err(|error| format!("required guide is missing at {}: {error}", path.display()))?;
        check_local_markdown_links(&root, &path, &body)?;
    }
    for path in [root.join("README.md"), guide_root.join("README.md")] {
        let body = fs::read_to_string(&path).map_err(|error| {
            format!(
                "documentation index is missing at {}: {error}",
                path.display()
            )
        })?;
        check_local_markdown_links(&root, &path, &body)?;
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
            fs::read(schema_root.join(name))
                .map(|bytes| (name.clone(), bytes))
                .map_err(|error| format!("could not read frozen schema `{name}`: {error}"))
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
            fs::read(path)
                .map(|bytes| (name.clone(), bytes))
                .map_err(|error| format!("could not read frozen migration `{name}`: {error}"))
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
        archive_entries.push(json!({
            "archive": file_name,
            "archive_format": target.archive,
            "executable": target.executable,
            "runner": target.runner,
            "sha256": sha256_file(&destination)?,
            "signing_kind": target.signing,
            "size": file_size(&destination)?,
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

    let expected_supplemental = BTreeSet::from([
        "ISSUE_COLLECTION.md".to_owned(),
        "KNOWN_LIMITATIONS.md".to_owned(),
        "RELEASE_NOTES.md".to_owned(),
        "THIRD_PARTY_NOTICES.md".to_owned(),
        format!("canisend-{version}-sbom.cdx.json"),
    ]);
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
    fn channel_candidate_source_cannot_authorize_publication() {
        let mut value = sample_channel_source().to_value();
        value["publication_authorized"] = Value::Bool(true);
        assert!(channel_candidate_source_from_value(&value).is_err());
    }
}
