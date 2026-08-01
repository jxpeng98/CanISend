use std::{
    fs,
    path::{Path, PathBuf},
};

#[must_use]
pub fn bundled_cli_path() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|executable| bundled_cli_path_from(&executable))
}

#[must_use]
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
    if let Some(candidate) = candidates.into_iter().find(|candidate| {
        fs::symlink_metadata(candidate).is_ok_and(|metadata| {
            metadata.is_file() && !metadata.file_type().is_symlink() && candidate != executable
        })
    }) {
        return Some(candidate);
    }

    is_unified_desktop_executable(executable).then(|| executable.to_path_buf())
}

fn is_unified_desktop_executable(executable: &Path) -> bool {
    let expected_name = if cfg!(windows) {
        "canisend-gui.exe"
    } else {
        "canisend-gui"
    };
    executable.file_name().and_then(|name| name.to_str()) == Some(expected_name)
        && fs::symlink_metadata(executable)
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::bundled_cli_path_from;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn cli_name() -> &'static str {
        if cfg!(windows) {
            "canisend.exe"
        } else {
            "canisend"
        }
    }

    fn root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-desktop-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn finds_sibling_cli_for_development_build() {
        let root = root();
        let executable = root.join("target/release/canisend-gui");
        let cli = root.join("target/release").join(cli_name());
        fs::create_dir_all(executable.parent().expect("parent")).expect("directory");
        fs::write(&executable, b"desktop").expect("desktop executable");
        fs::write(&cli, b"cli").expect("cli executable");

        assert_eq!(bundled_cli_path_from(&executable), Some(cli));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn prefers_app_bundle_resource_cli() {
        let root = root();
        let executable = root.join("CanISend.app/Contents/MacOS/canisend-gui");
        let cli = root
            .join("CanISend.app/Contents/Resources/bin")
            .join(cli_name());
        fs::create_dir_all(executable.parent().expect("executable parent")).expect("macos");
        fs::create_dir_all(cli.parent().expect("cli parent")).expect("resources");
        fs::write(&executable, b"desktop").expect("desktop executable");
        fs::write(&cli, b"cli").expect("cli executable");

        assert_eq!(bundled_cli_path_from(&executable), Some(cli));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn uses_unified_desktop_executable_when_no_separate_cli_exists() {
        let root = root();
        let development_executable = root.join("target/release/canisend-gui");
        fs::create_dir_all(development_executable.parent().expect("parent")).expect("directory");
        fs::write(&development_executable, b"desktop and cli").expect("unified executable");

        assert_eq!(
            bundled_cli_path_from(&development_executable),
            Some(development_executable.clone())
        );

        let app_executable = root.join("CanISend.app/Contents/MacOS/canisend-gui");
        fs::create_dir_all(app_executable.parent().expect("parent")).expect("app directory");
        fs::write(&app_executable, b"desktop and cli").expect("unified app executable");
        assert_eq!(
            bundled_cli_path_from(&app_executable),
            Some(app_executable.clone())
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn does_not_treat_an_arbitrary_or_symlinked_host_as_the_cli() {
        let root = root();
        let arbitrary = root.join("target/release/another-host");
        fs::create_dir_all(arbitrary.parent().expect("parent")).expect("directory");
        fs::write(&arbitrary, b"not CanISend").expect("host executable");
        assert_eq!(bundled_cli_path_from(&arbitrary), None);

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let target = root.join("real-binary");
            let linked_gui = root.join("target/release/canisend-gui");
            fs::write(&target, b"real executable").expect("target executable");
            symlink(&target, &linked_gui).expect("GUI symlink");
            assert_eq!(bundled_cli_path_from(&linked_gui), None);
        }

        fs::remove_dir_all(root).expect("cleanup");
    }
}
