#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
fn main() {
    canisend_desktop::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("The CanISend Svelte desktop preview currently supports macOS only.");
}
