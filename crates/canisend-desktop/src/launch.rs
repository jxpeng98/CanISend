use std::{ffi::OsString, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaunchMode {
    Gui,
    Cli,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopPlatform {
    MacOs,
    Windows,
    Linux,
}

impl DesktopPlatform {
    pub(crate) const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

pub(crate) fn launch_mode(arguments: &[OsString], platform: DesktopPlatform) -> LaunchMode {
    let executable_is_cli = arguments
        .first()
        .is_some_and(|argument| is_installed_cli_name(argument, platform));
    let explicit = arguments.get(1..).unwrap_or_default();

    match explicit {
        [] if executable_is_cli => LaunchMode::Cli,
        [] => LaunchMode::Gui,
        [argument] if argument == "--gui" => LaunchMode::Gui,
        [argument]
            if platform == DesktopPlatform::MacOs
                && !executable_is_cli
                && argument.to_string_lossy().starts_with("-psn_") =>
        {
            LaunchMode::Gui
        }
        _ => LaunchMode::Cli,
    }
}

fn is_installed_cli_name(argument: &OsString, platform: DesktopPlatform) -> bool {
    if platform == DesktopPlatform::Windows {
        let value = argument.to_string_lossy();
        let name = value.rsplit(['/', '\\']).next().unwrap_or_default();
        name.eq_ignore_ascii_case("canisend.exe")
    } else {
        Path::new(argument)
            .file_name()
            .is_some_and(|name| name == "canisend")
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{DesktopPlatform, LaunchMode, launch_mode};

    fn arguments(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn desktop_hosts_open_the_gui_without_arguments() {
        for (platform, executable) in [
            (
                DesktopPlatform::MacOs,
                "/Applications/CanISend.app/Contents/MacOS/canisend-gui",
            ),
            (
                DesktopPlatform::Windows,
                r"C:\Program Files\CanISend\canisend-gui.exe",
            ),
            (DesktopPlatform::Linux, "/usr/lib/canisend/canisend-gui"),
        ] {
            assert_eq!(
                launch_mode(&arguments(&[executable]), platform),
                LaunchMode::Gui
            );
        }
    }

    #[test]
    fn installed_cli_names_select_cli_mode_without_arguments() {
        for (platform, executable) in [
            (DesktopPlatform::MacOs, "/Users/example/.local/bin/canisend"),
            (
                DesktopPlatform::Windows,
                r"C:\Users\example\AppData\Local\CanISend\bin\CANISEND.EXE",
            ),
            (DesktopPlatform::Linux, "/home/example/.local/bin/canisend"),
        ] {
            assert_eq!(
                launch_mode(&arguments(&[executable]), platform),
                LaunchMode::Cli
            );
        }
    }

    #[test]
    fn explicit_gui_and_finder_launches_select_the_desktop_entrypoint() {
        assert_eq!(
            launch_mode(
                &arguments(&["canisend-gui", "--gui"]),
                DesktopPlatform::Linux
            ),
            LaunchMode::Gui
        );
        assert_eq!(
            launch_mode(
                &arguments(&["canisend-gui", "-psn_0_12345"]),
                DesktopPlatform::MacOs
            ),
            LaunchMode::Gui
        );
        assert_eq!(
            launch_mode(
                &arguments(&["canisend-gui", "-psn_0_12345"]),
                DesktopPlatform::Linux
            ),
            LaunchMode::Cli
        );
    }

    #[test]
    fn every_explicit_cli_or_mcp_command_selects_the_shared_dispatcher() {
        for platform in [
            DesktopPlatform::MacOs,
            DesktopPlatform::Windows,
            DesktopPlatform::Linux,
        ] {
            assert_eq!(
                launch_mode(&arguments(&["canisend-gui", "version", "--json"]), platform),
                LaunchMode::Cli
            );
            assert_eq!(
                launch_mode(
                    &arguments(&[
                        "canisend-gui",
                        "--workspace",
                        "/tmp/workspace",
                        "mcp",
                        "serve",
                    ]),
                    platform
                ),
                LaunchMode::Cli
            );
            assert_eq!(
                launch_mode(&arguments(&["canisend-gui", "--gui", "version"]), platform),
                LaunchMode::Cli,
                "ambiguous extra arguments must fail through Clap"
            );
        }
    }
}
