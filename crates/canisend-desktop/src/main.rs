#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
use std::{
    ffi::{OsStr, OsString},
    process::ExitCode,
};

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaunchMode {
    Gui,
    Cli,
}

#[cfg(target_os = "macos")]
fn launch_mode(arguments: &[OsString]) -> LaunchMode {
    match arguments.get(1..) {
        Some([]) | None => LaunchMode::Gui,
        Some([argument])
            if argument == OsStr::new("--gui")
                || argument.to_string_lossy().starts_with("-psn_") =>
        {
            LaunchMode::Gui
        }
        Some(_) => LaunchMode::Cli,
    }
}

#[cfg(target_os = "macos")]
fn main() -> ExitCode {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    match launch_mode(&arguments) {
        LaunchMode::Gui => {
            canisend_gui::run();
            ExitCode::SUCCESS
        }
        LaunchMode::Cli => canisend_cli::run(),
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("The CanISend desktop application currently supports macOS only.");
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use std::ffi::OsString;

    use super::{LaunchMode, launch_mode};

    fn arguments(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn finder_and_explicit_gui_launches_select_the_desktop_entrypoint() {
        assert_eq!(launch_mode(&arguments(&["canisend-gui"])), LaunchMode::Gui);
        assert_eq!(
            launch_mode(&arguments(&["canisend-gui", "--gui"])),
            LaunchMode::Gui
        );
        assert_eq!(
            launch_mode(&arguments(&["canisend-gui", "-psn_0_12345"])),
            LaunchMode::Gui
        );
    }

    #[test]
    fn every_explicit_cli_or_mcp_command_selects_the_shared_dispatcher() {
        assert_eq!(
            launch_mode(&arguments(&["canisend-gui", "version", "--json"])),
            LaunchMode::Cli
        );
        assert_eq!(
            launch_mode(&arguments(&[
                "canisend-gui",
                "--workspace",
                "/tmp/workspace",
                "mcp",
                "serve",
            ])),
            LaunchMode::Cli
        );
        assert_eq!(
            launch_mode(&arguments(&["canisend-gui", "--gui", "version"])),
            LaunchMode::Cli,
            "ambiguous extra arguments must fail through Clap instead of silently opening the GUI"
        );
    }
}
