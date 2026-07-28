#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
fn main() {
    canisend_gui::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("The CanISend desktop application currently supports macOS only.");
}
