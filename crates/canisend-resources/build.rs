use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fmt::Write as _,
    fs,
    path::{Component, Path, PathBuf},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceDeclaration {
    id: String,
    kind: String,
    path: String,
    version: String,
}

fn main() {
    if let Err(error) = build_manifest() {
        panic!("embedded resource manifest failed: {error}");
    }
}

fn build_manifest() -> Result<(), String> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?);
    let resource_root = manifest_dir.join("resources");
    let declaration_path = resource_root.join("manifest.json");
    println!("cargo:rerun-if-changed={}", resource_root.display());

    let declarations: Vec<ResourceDeclaration> =
        serde_json::from_slice(&fs::read(&declaration_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    if declarations.is_empty() {
        return Err("resource declaration cannot be empty".to_owned());
    }

    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut variants = BTreeSet::new();
    let mut generated = Vec::new();
    for declaration in declarations {
        validate_declaration(&declaration)?;
        if !ids.insert(declaration.id.clone()) {
            return Err(format!("duplicate resource ID: {}", declaration.id));
        }
        if !paths.insert(declaration.path.clone()) {
            return Err(format!("duplicate resource path: {}", declaration.path));
        }
        let variant = variant_name(&declaration.id);
        if !variants.insert(variant.clone()) {
            return Err(format!(
                "resource IDs produce duplicate Rust variant: {variant}"
            ));
        }
        let absolute_path = resource_root.join(&declaration.path);
        let bytes = fs::read(&absolute_path)
            .map_err(|error| format!("cannot read {}: {error}", absolute_path.display()))?;
        let digest = hex::encode(Sha256::digest(&bytes));
        generated.push((declaration, variant, absolute_path, bytes.len(), digest));
    }

    let actual_paths = collect_resource_paths(&resource_root)?;
    if paths != actual_paths {
        let missing = paths.difference(&actual_paths).cloned().collect::<Vec<_>>();
        let undeclared = actual_paths.difference(&paths).cloned().collect::<Vec<_>>();
        return Err(format!(
            "resource declarations differ from files; missing={missing:?}, undeclared={undeclared:?}"
        ));
    }

    generated.sort_unstable_by(|left, right| left.0.id.cmp(&right.0.id));
    let source = render_source(&generated)?;
    let output =
        PathBuf::from(env::var("OUT_DIR").map_err(|e| e.to_string())?).join("resource_manifest.rs");
    fs::write(output, source).map_err(|error| error.to_string())
}

fn validate_declaration(declaration: &ResourceDeclaration) -> Result<(), String> {
    if declaration.id.is_empty()
        || !declaration.id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
    {
        return Err(format!("invalid resource ID: {}", declaration.id));
    }
    if !matches!(
        declaration.kind.as_str(),
        "agent" | "example" | "prompt" | "schema" | "template"
    ) {
        return Err(format!("invalid resource kind: {}", declaration.kind));
    }
    if semver_like(&declaration.version).is_none() {
        return Err(format!("invalid resource version: {}", declaration.version));
    }
    let path = Path::new(&declaration.path);
    if declaration.path.contains('\\')
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("unsafe resource path: {}", declaration.path));
    }
    Ok(())
}

fn semver_like(value: &str) -> Option<()> {
    let parts = value.split('.').collect::<Vec<_>>();
    (parts.len() == 3 && parts.iter().all(|part| part.parse::<u64>().is_ok())).then_some(())
}

fn collect_resource_paths(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    Ok(files
        .into_iter()
        .filter(|path| path != "manifest.json")
        .collect())
}

fn collect_files(root: &Path, directory: &Path, files: &mut Vec<String>) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            return Err(format!(
                "resource tree contains symlink: {}",
                entry.path().display()
            ));
        }
        if file_type.is_dir() {
            collect_files(root, &entry.path(), files)?;
        } else if file_type.is_file() {
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            files.push(relative);
        }
    }
    Ok(())
}

fn variant_name(id: &str) -> String {
    id.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            let first = characters.next().expect("non-empty variant component");
            format!("{}{}", first.to_ascii_uppercase(), characters.as_str())
        })
        .collect()
}

fn render_source(
    resources: &[(ResourceDeclaration, String, PathBuf, usize, String)],
) -> Result<String, String> {
    let mut source = String::new();
    writeln!(
        source,
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]\npub enum ResourceId {{"
    )
    .map_err(|error| error.to_string())?;
    for (_, variant, _, _, _) in resources {
        writeln!(source, "    {variant},").map_err(|error| error.to_string())?;
    }
    writeln!(source, "}}\n\nimpl ResourceId {{").map_err(|error| error.to_string())?;
    writeln!(source, "    pub const ALL: [Self; {}] = [", resources.len())
        .map_err(|error| error.to_string())?;
    for (_, variant, _, _, _) in resources {
        writeln!(source, "        Self::{variant},").map_err(|error| error.to_string())?;
    }
    writeln!(source, "    ];\n\n    #[must_use]\n    pub const fn as_str(self) -> &'static str {{\n        match self {{")
        .map_err(|error| error.to_string())?;
    for (declaration, variant, _, _, _) in resources {
        writeln!(
            source,
            "            Self::{variant} => {:?},",
            declaration.id
        )
        .map_err(|error| error.to_string())?;
    }
    writeln!(source, "        }}\n    }}\n}}").map_err(|error| error.to_string())?;
    writeln!(source, "\nimpl std::fmt::Display for ResourceId {{\n    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n        formatter.write_str(self.as_str())\n    }}\n}}")
        .map_err(|error| error.to_string())?;
    writeln!(source, "\nimpl std::str::FromStr for ResourceId {{\n    type Err = ResourceError;\n\n    fn from_str(value: &str) -> Result<Self, Self::Err> {{\n        match value {{")
        .map_err(|error| error.to_string())?;
    for (declaration, variant, _, _, _) in resources {
        writeln!(
            source,
            "            {:?} => Ok(Self::{variant}),",
            declaration.id
        )
        .map_err(|error| error.to_string())?;
    }
    writeln!(
        source,
        "            _ => Err(ResourceError::UnknownId(value.to_owned())),\n        }}\n    }}\n}}"
    )
    .map_err(|error| error.to_string())?;

    let kind_variants = BTreeMap::from([
        ("agent", "Agent"),
        ("example", "Example"),
        ("prompt", "Prompt"),
        ("schema", "Schema"),
        ("template", "Template"),
    ]);
    writeln!(
        source,
        "\npub(crate) static EMBEDDED_RESOURCES: &[EmbeddedResource] = &["
    )
    .map_err(|error| error.to_string())?;
    for (declaration, variant, path, size, digest) in resources {
        let kind = kind_variants
            .get(declaration.kind.as_str())
            .ok_or_else(|| "validated resource kind disappeared".to_owned())?;
        writeln!(
            source,
            "    EmbeddedResource {{ id: ResourceId::{variant}, descriptor: ResourceDescriptor {{ id: {:?}, kind: ResourceKind::{kind}, path: {:?}, version: {:?}, size: {size}, sha256: {:?} }}, bytes: include_bytes!({:?}) }},",
            declaration.id,
            declaration.path,
            declaration.version,
            digest,
            path
        )
        .map_err(|error| error.to_string())?;
    }
    writeln!(source, "];\n").map_err(|error| error.to_string())?;
    Ok(source)
}
