use std::path::Path;

use canisend_contracts::{
    AGENT_PROTOCOL, PUBLIC_SCHEMA_VERSION, PublicSchemaId, RESOURCE_FORMAT, WORKSPACE_FORMAT,
};
use canisend_io::{EmbeddedTypstCompiler, render_acceptance_probe};
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
    pub rendered_pdf_bytes: usize,
    pub render_elapsed_millis: u128,
    pub schema_count: usize,
    pub binary_size_bytes: u64,
    pub release_binary_budget_bytes: u64,
    pub system_font_scan: bool,
    pub runtime_package_downloads: bool,
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
        let template = std::str::from_utf8(
            canisend_resources::get(canisend_resources::ResourceId::TemplateCoverLetter).bytes,
        )
        .map_err(|_| {
            ApplicationError::ResourceIntegrity("embedded Typst template is not UTF-8".to_owned())
        })?;
        let probe_source = format!(
            "{template}\n#application_cover_letter([CanISend], [Native self-check], [Embedded rendering verified.])"
        );
        EmbeddedTypstCompiler::new().compile_pdf(&probe_source)?;
        let rendered = render_acceptance_probe()?;
        let binary_size_bytes = std::env::current_exe()
            .and_then(|path| std::fs::metadata(path).map(|metadata| metadata.len()))
            .map_err(|error| {
                ApplicationError::ResourceIntegrity(format!(
                    "cannot measure current executable: {error}"
                ))
            })?;
        let data = DoctorSummary {
            healthy: true,
            embedded_resources: canisend_resources::manifest().len(),
            embedded_renderer: true,
            rendered_pages: rendered.page_count() as usize,
            render_warning_count: rendered.warning_count(),
            rendered_pdf_bytes: rendered.bytes().len(),
            render_elapsed_millis: rendered.elapsed().as_millis(),
            schema_count: PublicSchemaId::ALL.len(),
            binary_size_bytes,
            release_binary_budget_bytes: 67_108_864,
            system_font_scan: false,
            runtime_package_downloads: false,
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

    pub fn configure_cli_path(
        source: Option<&Path>,
        destination: &Path,
        consent: TerminalInstallConsent,
    ) -> Result<ActionReceipt<CliInstallStatus>, ApplicationError> {
        cli_install::configure_path(source, destination, consent)
    }

    pub fn check_for_updates(
        consent: NetworkFetchConsent,
    ) -> Result<ActionReceipt<UpdateCheckReadModel>, ApplicationError> {
        update::check(consent)
    }
}
