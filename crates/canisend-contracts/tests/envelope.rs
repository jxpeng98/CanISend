use canisend_contracts::{AGENT_PROTOCOL, AgentError, AgentResponse, ErrorCode};
use serde_json::json;

#[test]
fn success_envelope_is_body_safe_and_versioned() {
    let response = AgentResponse::success("agent.capabilities", "available", json!({"count": 1}));
    let value = serde_json::to_value(response).expect("response serializes");

    assert_eq!(value["protocol"], AGENT_PROTOCOL);
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["count"], 1);
    assert!(value["error"].is_null());
}

#[test]
fn failure_envelope_has_no_success_data() {
    let response = AgentResponse::failure(
        "workspace.open",
        "not-found",
        AgentError {
            code: ErrorCode::WorkspaceNotFound,
            message: "The workspace does not exist.".to_owned(),
            retryable: false,
            details: None,
            remediation: None,
        },
    );
    let value = serde_json::to_value(response).expect("response serializes");

    assert_eq!(value["ok"], false);
    assert!(value["data"].is_null());
    assert_eq!(value["error"]["code"], "workspace.not_found");
}
