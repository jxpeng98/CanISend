use std::path::Path;

use canisend_contracts::{
    AGENT_PROTOCOL, PUBLIC_SCHEMA_VERSION, RESOURCE_FORMAT, WORKSPACE_FORMAT,
};
use canisend_io::render_acceptance_probe;
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, CliInstallStatus, NetworkFetchConsent,
    TerminalInstallConsent, UpdateCheckReadModel, cli_install, update,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductSummary {
    pub product: String,
    pub version: String,
    pub protocol: String,
    pub workspace_format: String,
    pub resource_format: String,
    pub public_schema_version: String,
    pub target_os: String,
    pub target_arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctorSummary {
    pub healthy: bool,
    pub embedded_resources: usize,
    pub embedded_renderer: bool,
    pub rendered_pages: usize,
    pub render_warning_count: usize,
    pub python_required: bool,
}

impl Application {
    #[must_use]
    pub fn product_summary() -> ProductSummary {
        ProductSummary {
            product: "canisend".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            protocol: AGENT_PROTOCOL.to_owned(),
            workspace_format: WORKSPACE_FORMAT.to_owned(),
            resource_format: RESOURCE_FORMAT.to_owned(),
            public_schema_version: PUBLIC_SCHEMA_VERSION.to_owned(),
            target_os: std::env::consts::OS.to_owned(),
            target_arch: std::env::consts::ARCH.to_owned(),
        }
    }

    pub fn doctor() -> Result<ActionReceipt<DoctorSummary>, ApplicationError> {
        canisend_resources::verify().map_err(ApplicationError::ResourceIntegrity)?;
        let rendered = render_acceptance_probe()?;
        let data = DoctorSummary {
            healthy: true,
            embedded_resources: canisend_resources::manifest().len(),
            embedded_renderer: true,
            rendered_pages: rendered.page_count() as usize,
            render_warning_count: rendered.warning_count(),
            python_required: false,
        };
        Ok(ActionReceipt::new(
            "product.doctor",
            "healthy",
            "Native resources and embedded PDF renderer verified",
            data,
        ))
    }

    pub fn cli_install_status(
        source: Option<&Path>,
        destination: &Path,
    ) -> Result<ActionReceipt<CliInstallStatus>, ApplicationError> {
        cli_install::inspect(source, destination)
    }

    pub fn install_cli(
        source: &Path,
        destination: &Path,
        replace_existing: bool,
        consent: TerminalInstallConsent,
    ) -> Result<ActionReceipt<CliInstallStatus>, ApplicationError> {
        cli_install::install(source, destination, replace_existing, consent)
    }

    pub fn uninstall_cli(
        source: Option<&Path>,
        destination: &Path,
        consent: TerminalInstallConsent,
    ) -> Result<ActionReceipt<CliInstallStatus>, ApplicationError> {
        cli_install::uninstall(source, destination, consent)
    }

    pub fn check_for_updates(
        consent: NetworkFetchConsent,
    ) -> Result<ActionReceipt<UpdateCheckReadModel>, ApplicationError> {
        update::check(consent)
    }
}
