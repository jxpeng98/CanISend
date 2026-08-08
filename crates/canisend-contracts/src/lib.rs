#![forbid(unsafe_code)]

mod agent;
mod application_v3;
mod association_v4;
mod discovery;
mod domain;
mod operation;
mod primitives;
mod schema;
mod validation;
mod workflow;
mod workflow_pack;
mod workspace;

pub use agent::*;
pub use application_v3::*;
pub use association_v4::*;
pub use discovery::*;
pub use domain::*;
pub use operation::*;
pub use primitives::*;
pub use schema::*;
pub use validation::*;
pub use workflow::*;
pub use workflow_pack::*;
pub use workspace::*;

pub const AGENT_PROTOCOL: &str = "canisend.agent/v2";
pub const WORKSPACE_FORMAT: &str = "canisend.workspace/v2";
pub const WORKSPACE_V4_FORMAT: &str = "canisend.workspace/v4";
pub const RESOURCE_FORMAT: &str = "canisend.resources/v2";
