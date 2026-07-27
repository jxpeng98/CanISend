use canisend_app::{ActionReceipt, Application, ApplicationError, DoctorSummary, ProductSummary};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopCommandError {
    code: &'static str,
    message: String,
}

impl DesktopCommandError {
    fn application(error: ApplicationError) -> Self {
        Self {
            code: "application-failure",
            message: error.to_string(),
        }
    }

    #[cfg(target_os = "macos")]
    fn worker(message: String) -> Self {
        Self {
            code: "desktop-worker-failure",
            message,
        }
    }
}

pub(crate) fn product_summary_impl() -> ProductSummary {
    Application::product_summary()
}

pub(crate) fn doctor_impl() -> Result<ActionReceipt<DoctorSummary>, DesktopCommandError> {
    Application::doctor().map_err(DesktopCommandError::application)
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) fn product_summary() -> ProductSummary {
    product_summary_impl()
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn run_doctor() -> Result<ActionReceipt<DoctorSummary>, DesktopCommandError> {
    tauri::async_runtime::spawn_blocking(doctor_impl)
        .await
        .map_err(|error| DesktopCommandError::worker(error.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_summary_exposes_the_shared_rust_contract() {
        let summary = product_summary_impl();

        assert_eq!(summary.product, "canisend");
        assert_eq!(summary.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(summary.protocol, "canisend.agent/v2");
        assert_eq!(summary.workspace_format, "canisend.workspace/v2");
        assert_eq!(summary.resource_format, "canisend.resources/v2");
        assert!(!summary.target_os.is_empty());
        assert!(!summary.target_arch.is_empty());
    }

    #[test]
    fn desktop_errors_are_stable_serializable_envelopes() {
        let error = DesktopCommandError {
            code: "fixture",
            message: "bounded message".to_owned(),
        };
        let json = serde_json::to_value(error).expect("desktop error must serialize");

        assert_eq!(json["code"], "fixture");
        assert_eq!(json["message"], "bounded message");
    }
}
