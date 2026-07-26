#![forbid(unsafe_code)]

mod cli_bridge;
mod components;
mod desktop;
mod i18n;
mod registry;
mod state;
mod theme;
mod worker;

fn main() -> eframe::Result {
    desktop::run()
}
