use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-env-changed=CANISEND_GIT_SHA");
    let repository = repository_root();
    emit_git_rerun_paths(&repository);

    let git_revision = env::var("CANISEND_GIT_SHA")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| read_git_revision(&repository))
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

fn repository_root() -> PathBuf {
    Path::new(&env::var("CARGO_MANIFEST_DIR").expect("Cargo provides CARGO_MANIFEST_DIR"))
        .join("../..")
}

fn emit_git_rerun_paths(repository: &Path) {
    let Some(git_directory) = git_output(repository, &["rev-parse", "--absolute-git-dir"]) else {
        return;
    };
    println!("cargo:rerun-if-changed={git_directory}/HEAD");
    if let Some(reference) = git_output(repository, &["symbolic-ref", "-q", "HEAD"])
        && let Some(reference_path) =
            git_output(repository, &["rev-parse", "--git-path", &reference])
    {
        let reference_path = Path::new(&reference_path);
        let absolute = if reference_path.is_absolute() {
            reference_path.to_path_buf()
        } else {
            repository.join(reference_path)
        };
        println!("cargo:rerun-if-changed={}", absolute.display());
    }
}

fn read_git_revision(repository: &Path) -> Option<String> {
    git_output(repository, &["rev-parse", "--short=12", "HEAD"])
}

fn git_output(repository: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(repository)
        .args(arguments)
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
