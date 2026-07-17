#![forbid(unsafe_code)]

use serde::Serialize;
use sha2::{Digest, Sha256};

pub const RESOURCE_VERSION: &str = "canisend.resources/v2";

const GENERIC_AGENT_GUIDE: &[u8] = include_bytes!("../resources/agent/generic/README.md");
const GENERIC_AGENT_GUIDE_SHA256: &str = env!("CANISEND_GENERIC_AGENT_GUIDE_SHA256");

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceDescriptor {
    pub id: &'static str,
    pub path: &'static str,
    pub version: &'static str,
    pub size: usize,
    pub sha256: &'static str,
}

#[must_use]
pub fn manifest() -> Vec<ResourceDescriptor> {
    vec![ResourceDescriptor {
        id: "agent.generic.readme",
        path: "agent/generic/README.md",
        version: "2.0.0",
        size: GENERIC_AGENT_GUIDE.len(),
        sha256: GENERIC_AGENT_GUIDE_SHA256,
    }]
}

pub fn verify() -> Result<(), String> {
    let actual = hex::encode(Sha256::digest(GENERIC_AGENT_GUIDE));
    if actual != GENERIC_AGENT_GUIDE_SHA256 {
        return Err("embedded resource digest mismatch".to_owned());
    }
    Ok(())
}

#[must_use]
pub fn generic_agent_guide() -> &'static [u8] {
    GENERIC_AGENT_GUIDE
}
