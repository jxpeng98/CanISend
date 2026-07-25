use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn bundled_cli_path() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|executable| bundled_cli_path_from(&executable))
}

pub fn default_cli_destination() -> PathBuf {
    let executable = if cfg!(windows) {
        "canisend.exe"
    } else {
        "canisend"
    };
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local/bin").join(executable);
    }
    std::env::temp_dir().join("canisend/bin").join(executable)
}

fn bundled_cli_path_from(executable: &Path) -> Option<PathBuf> {
    let parent = executable.parent()?;
    let cli_name = if cfg!(windows) {
        "canisend.exe"
    } else {
        "canisend"
    };
    let mut candidates = Vec::with_capacity(2);
    if parent.file_name().and_then(|name| name.to_str()) == Some("MacOS")
        && let Some(contents) = parent.parent()
    {
        candidates.push(contents.join("Resources/bin").join(cli_name));
    }
    candidates.push(parent.join(cli_name));
    candidates.into_iter().find(|candidate| {
        fs::symlink_metadata(candidate).is_ok_and(|metadata| {
            metadata.is_file() && !metadata.file_type().is_symlink() && candidate != executable
        })
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::bundled_cli_path_from;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-gui-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn finds_sibling_cli_for_development_build() {
        let root = root();
        let executable = root.join("target/release/canisend-gui");
        let cli = root.join("target/release/canisend");
        fs::create_dir_all(executable.parent().expect("parent")).expect("directory");
        fs::write(&executable, b"gui").expect("gui");
        fs::write(&cli, b"cli").expect("cli");

        assert_eq!(bundled_cli_path_from(&executable), Some(cli));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn prefers_app_bundle_resource_cli() {
        let root = root();
        let executable = root.join("CanISend.app/Contents/MacOS/canisend-gui");
        let cli = root.join("CanISend.app/Contents/Resources/bin/canisend");
        fs::create_dir_all(executable.parent().expect("executable parent")).expect("macos");
        fs::create_dir_all(cli.parent().expect("cli parent")).expect("resources");
        fs::write(&executable, b"gui").expect("gui");
        fs::write(&cli, b"cli").expect("cli");

        assert_eq!(bundled_cli_path_from(&executable), Some(cli));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
