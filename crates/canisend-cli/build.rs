use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=CANISEND_GIT_SHA");
    println!("cargo:rerun-if-changed=../../.git/HEAD");

    let git_revision = env::var("CANISEND_GIT_SHA")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(read_git_revision)
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=CANISEND_GIT_REVISION={git_revision}");

    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_owned());
    println!("cargo:rustc-env=CANISEND_BUILD_TARGET={target}");

    let rustc = env::var("RUSTC")
        .ok()
        .and_then(|program| Command::new(program).arg("--version").output().ok())
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=CANISEND_RUSTC_VERSION={rustc}");
}

fn read_git_revision() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
