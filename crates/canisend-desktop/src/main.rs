#![forbid(unsafe_code)]

use std::process::ExitCode;

mod launch;

use launch::{DesktopPlatform, LaunchMode, launch_mode};

fn main() -> ExitCode {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    match launch_mode(&arguments, DesktopPlatform::current()) {
        LaunchMode::Gui => launch_gui(),
        LaunchMode::Cli => canisend_cli::run(),
    }
}

fn launch_gui() -> ExitCode {
    canisend_gui::run();
    ExitCode::SUCCESS
}
