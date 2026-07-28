use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, CliInstallStatus, InspectionCatalogReadModel, NetworkFetchConsent,
    ResourceCatalogExportReadModel, ResourceCatalogExportRequest, ResourceDetailReadModel,
    TerminalInstallConsent, UpdateCheckReadModel, bundled_cli_path, default_cli_destination,
};
use canisend_contracts::SchemaCatalogEntry;
use serde::{Deserialize, Serialize};

use crate::commands::{DesktopCommandError, run_worker};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopCliDefaults {
    bundled_source: Option<PathBuf>,
    destination: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopCliRequest {
    destination: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopCliInstallRequest {
    destination: Option<PathBuf>,
    replace_existing: bool,
    confirmed_terminal_install: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopCliUninstallRequest {
    destination: Option<PathBuf>,
    confirmed_terminal_install: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateRequest {
    confirmed_network_fetch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CatalogDetailRequest {
    query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CatalogExportRequest {
    destination: PathBuf,
}

fn destination_or_default(destination: Option<PathBuf>) -> PathBuf {
    destination
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(default_cli_destination)
}

fn install_cli_impl(
    request: DesktopCliInstallRequest,
) -> Result<ActionReceipt<CliInstallStatus>, DesktopCommandError> {
    if !request.confirmed_terminal_install {
        return Err(DesktopCommandError::consent(
            "Confirm terminal installation before writing or replacing the CanISend CLI.",
        ));
    }
    let source = bundled_cli_path().ok_or_else(|| {
        DesktopCommandError::state(
            "This development application does not contain a bundled CanISend CLI.",
        )
    })?;
    Application::install_cli(
        &source,
        &destination_or_default(request.destination),
        request.replace_existing,
        TerminalInstallConsent::granted_by_user(),
    )
    .map_err(DesktopCommandError::application)
}

fn uninstall_cli_impl(
    request: DesktopCliUninstallRequest,
) -> Result<ActionReceipt<CliInstallStatus>, DesktopCommandError> {
    if !request.confirmed_terminal_install {
        return Err(DesktopCommandError::consent(
            "Confirm terminal uninstall before removing the managed CanISend CLI.",
        ));
    }
    let source = bundled_cli_path();
    Application::uninstall_cli(
        source.as_deref(),
        &destination_or_default(request.destination),
        TerminalInstallConsent::granted_by_user(),
    )
    .map_err(DesktopCommandError::application)
}

fn check_for_updates_impl(
    request: UpdateRequest,
) -> Result<ActionReceipt<UpdateCheckReadModel>, DesktopCommandError> {
    if !request.confirmed_network_fetch {
        return Err(DesktopCommandError::consent(
            "Confirm the bounded GitHub Releases request before checking for updates.",
        ));
    }
    Application::check_for_updates(NetworkFetchConsent::granted_by_user())
        .map_err(DesktopCommandError::application)
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) fn desktop_cli_defaults() -> DesktopCliDefaults {
    DesktopCliDefaults {
        bundled_source: bundled_cli_path(),
        destination: default_cli_destination(),
    }
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn cli_install_status(
    request: DesktopCliRequest,
) -> Result<ActionReceipt<CliInstallStatus>, DesktopCommandError> {
    run_worker(move || {
        let source = bundled_cli_path();
        Application::cli_install_status(
            source.as_deref(),
            &destination_or_default(request.destination),
        )
        .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn install_cli(
    request: DesktopCliInstallRequest,
) -> Result<ActionReceipt<CliInstallStatus>, DesktopCommandError> {
    run_worker(move || install_cli_impl(request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn uninstall_cli(
    request: DesktopCliUninstallRequest,
) -> Result<ActionReceipt<CliInstallStatus>, DesktopCommandError> {
    run_worker(move || uninstall_cli_impl(request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn check_for_updates(
    request: UpdateRequest,
) -> Result<ActionReceipt<UpdateCheckReadModel>, DesktopCommandError> {
    run_worker(move || check_for_updates_impl(request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn inspection_catalog()
-> Result<ActionReceipt<InspectionCatalogReadModel>, DesktopCommandError> {
    run_worker(|| Application::inspection_catalog().map_err(DesktopCommandError::application)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn schema_detail(
    request: CatalogDetailRequest,
) -> Result<ActionReceipt<SchemaCatalogEntry>, DesktopCommandError> {
    run_worker(move || {
        Application::schema_detail(&request.query).map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn resource_detail(
    request: CatalogDetailRequest,
) -> Result<ActionReceipt<ResourceDetailReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::resource_detail(&request.query).map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn export_resource_catalog(
    request: CatalogExportRequest,
) -> Result<ActionReceipt<ResourceCatalogExportReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::export_resource_catalog(&ResourceCatalogExportRequest::new(
            request.destination,
        ))
        .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_and_network_commands_require_explicit_consent() {
        let install = install_cli_impl(DesktopCliInstallRequest {
            destination: Some(PathBuf::from("/missing/canisend")),
            replace_existing: false,
            confirmed_terminal_install: false,
        })
        .expect_err("CLI install needs consent");
        assert_eq!(install.code, "consent-required");

        let uninstall = uninstall_cli_impl(DesktopCliUninstallRequest {
            destination: Some(PathBuf::from("/missing/canisend")),
            confirmed_terminal_install: false,
        })
        .expect_err("CLI uninstall needs consent");
        assert_eq!(uninstall.code, "consent-required");

        let update = check_for_updates_impl(UpdateRequest {
            confirmed_network_fetch: false,
        })
        .expect_err("update check needs consent");
        assert_eq!(update.code, "consent-required");
    }

    #[test]
    fn inspection_catalog_delegates_to_verified_embedded_resources() {
        let receipt = Application::inspection_catalog().expect("inspection catalog");
        assert_eq!(receipt.operation, "inspection.catalog");
        assert!(!receipt.data.schemas.schemas.is_empty());
        assert!(!receipt.data.resources.is_empty());
    }

    #[test]
    fn default_cli_destination_is_an_explicit_executable_path() {
        let destination = default_cli_destination();
        assert_eq!(
            destination.file_name().and_then(|name| name.to_str()),
            Some(if cfg!(windows) {
                "canisend.exe"
            } else {
                "canisend"
            })
        );
    }
}
