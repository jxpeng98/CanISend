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
            check_release_contract()
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
        _ => Err(
            "usage: cargo run -p xtask -- schemas <check|write> | <resources|docs> check | \
             release <check|validate-tag TAG|sbom OUTPUT|assemble TAG COMMIT ARTIFACTS OUTPUT|verify TAG DIRECTORY>"
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

fn validate_release_tag(tag: &str) -> Result<ReleaseStage, String> {
    let expected = format!("v{}", env!("CARGO_PKG_VERSION"));
    if tag != expected {
        return Err(format!(
            "release tag `{tag}` does not match workspace version `{expected}`"
        ));
    }
    let version = Version::parse(tag.trim_start_matches('v'))
        .map_err(|error| format!("release tag is not valid SemVer: {error}"))?;
    let prerelease = version.pre.as_str();
    let stage = if prerelease.starts_with("alpha.") {
        ReleaseStage::Alpha
    } else if prerelease.starts_with("beta.") {
        ReleaseStage::Beta
    } else if prerelease.starts_with("rc.") {
        ReleaseStage::ReleaseCandidate
    } else if prerelease.is_empty() {
        ReleaseStage::Stable
    } else {
        return Err(format!(
            "release tag prerelease `{prerelease}` is not alpha, beta, or rc"
        ));
    };
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
    let manifest_targets = manifest["artifacts"]
        .as_array()
        .ok_or_else(|| "release manifest artifacts are missing".to_owned())?
        .iter()
        .filter_map(|entry| entry["target"].as_str())
        .collect::<BTreeSet<_>>();
    let expected_targets = release_targets()?
        .into_iter()
        .map(|target| target.triple)
        .collect::<BTreeSet<_>>();
    if manifest_targets != expected_targets.iter().map(String::as_str).collect() {
        return Err("release manifest does not cover the complete target matrix".to_owned());
    }
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
}
