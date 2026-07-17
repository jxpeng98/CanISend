#![forbid(unsafe_code)]

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use canisend_contracts::{generate_public_schemas, verify_public_schemas};

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
        [area, command] if area == "release" && command == "check" => {
            check_schemas()?;
            check_resources()
        }
        _ => Err(
            "usage: cargo run -p xtask -- schemas <check|write> | <resources|release> check"
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
