#![forbid(unsafe_code)]

mod agent;
mod discovery;
mod domain;
mod primitives;
mod schema;
mod validation;
mod workspace;

pub use agent::*;
pub use discovery::*;
pub use domain::*;
pub use primitives::*;
pub use schema::*;
pub use validation::*;
pub use workspace::*;

pub const AGENT_PROTOCOL: &str = "canisend.agent/v2";
pub const WORKSPACE_FORMAT: &str = "canisend.workspace/v2";
pub const RESOURCE_FORMAT: &str = "canisend.resources/v2";
