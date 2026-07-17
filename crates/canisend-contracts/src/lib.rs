#![forbid(unsafe_code)]

mod agent;
mod domain;
mod primitives;
mod validation;

pub use agent::*;
pub use domain::*;
pub use primitives::*;
pub use validation::*;

pub const AGENT_PROTOCOL: &str = "canisend.agent/v2";
pub const WORKSPACE_FORMAT: &str = "canisend.workspace/v2";
pub const RESOURCE_FORMAT: &str = "canisend.resources/v2";
