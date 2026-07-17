use std::{env, fs, path::PathBuf};

use sha2::{Digest, Sha256};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let resource = manifest_dir.join("resources/agent/generic/README.md");
    println!("cargo:rerun-if-changed={}", resource.display());

    let bytes = fs::read(&resource).expect("embedded generic agent guide is readable");
    let digest = hex::encode(Sha256::digest(bytes));
    println!("cargo:rustc-env=CANISEND_GENERIC_AGENT_GUIDE_SHA256={digest}");
}
