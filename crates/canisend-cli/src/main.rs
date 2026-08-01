#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    canisend_cli::run()
}
