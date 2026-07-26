use std::path::PathBuf;

use canisend_app::{Application, ApplicationError};

use super::{CommandFailure, CommandResult};

pub(super) fn failure(operation: &'static str, error: ApplicationError) -> Box<CommandFailure> {
    let classified = error.classify();
    let mut failure = CommandFailure::new(
        operation,
        classified.status,
        classified.code,
        classified.message,
        classified.retryable,
    );
    failure.error.details = classified.details;
    failure.error.remediation = classified.remediation;
    failure
}

pub(super) fn workspace_root(
    explicit: Option<PathBuf>,
    operation: &'static str,
) -> CommandResult<PathBuf> {
    Application::resolve_workspace_root(explicit.as_deref())
        .map_err(|error| failure(operation, error))
}
