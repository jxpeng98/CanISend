use std::{
    fs,
    path::{Path, PathBuf},
};

#[must_use]
pub fn desktop_cli_source_path() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|executable| desktop_cli_source_path_from(&executable))
}

#[must_use]
pub fn default_cli_destination() -> PathBuf {
    let executable = if cfg!(windows) {
        "canisend.exe"
    } else {
        "canisend"
    };
    if cfg!(windows)
        && let Some(local_app_data) = std::env::var_os("LOCALAPPDATA")
    {
        return PathBuf::from(local_app_data)
            .join("CanISend/bin")
            .join(executable);
    }
    if let Some(home) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
        return PathBuf::from(home).join(".local/bin").join(executable);
    }
    std::env::temp_dir().join("canisend/bin").join(executable)
}

fn desktop_cli_source_path_from(executable: &Path) -> Option<PathBuf> {
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

    use super::desktop_cli_source_path_from;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-desktop-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn ignores_a_separate_sibling_cli_and_uses_the_unified_host() {
        let root = root();
        let executable = root.join("target/release/canisend-gui");
        let cli = root.join("target/release/canisend");
        fs::create_dir_all(executable.parent().expect("parent")).expect("directory");
        fs::write(&executable, b"desktop").expect("desktop executable");
        fs::write(&cli, b"cli").expect("cli executable");

        assert_eq!(
            desktop_cli_source_path_from(&executable),
            Some(executable.clone())
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn ignores_a_legacy_app_resource_cli_and_uses_the_unified_host() {
        let root = root();
        let executable = root.join("CanISend.app/Contents/MacOS/canisend-gui");
        let cli = root.join("CanISend.app/Contents/Resources/bin/canisend");
        fs::create_dir_all(executable.parent().expect("executable parent")).expect("macos");
        fs::create_dir_all(cli.parent().expect("cli parent")).expect("resources");
        fs::write(&executable, b"desktop").expect("desktop executable");
        fs::write(&cli, b"cli").expect("cli executable");

        assert_eq!(
            desktop_cli_source_path_from(&executable),
            Some(executable.clone())
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn uses_unified_desktop_executable_when_no_separate_cli_exists() {
        let root = root();
        let development_executable = root.join("target/release/canisend-gui");
        fs::create_dir_all(development_executable.parent().expect("parent")).expect("directory");
        fs::write(&development_executable, b"desktop and cli").expect("unified executable");

        assert_eq!(
            desktop_cli_source_path_from(&development_executable),
            Some(development_executable.clone())
        );

        let app_executable = root.join("CanISend.app/Contents/MacOS/canisend-gui");
        fs::create_dir_all(app_executable.parent().expect("parent")).expect("app directory");
        fs::write(&app_executable, b"desktop and cli").expect("unified app executable");
        assert_eq!(
            desktop_cli_source_path_from(&app_executable),
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
        assert_eq!(desktop_cli_source_path_from(&arbitrary), None);

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let target = root.join("real-binary");
            let linked_gui = root.join("target/release/canisend-gui");
            fs::write(&target, b"real executable").expect("target executable");
            symlink(&target, &linked_gui).expect("GUI symlink");
            assert_eq!(desktop_cli_source_path_from(&linked_gui), None);
        }

        fs::remove_dir_all(root).expect("cleanup");
    }
}
