use std::{io::Read, path::Path};

use canisend_contracts::TaskCompletionRequest;
use canisend_io::{
    read_structured_candidate_file, read_task_completion_file, read_task_completion_stdin,
    write_private_json_new,
};
use serde::Serialize;
use serde_json::Value;

use crate::{Application, ApplicationError, PrivateReadConsent};

impl Application {
    pub fn read_structured_candidate(
        path: &Path,
        _consent: PrivateReadConsent,
    ) -> Result<Value, ApplicationError> {
        Ok(read_structured_candidate_file(path)?)
    }

    pub fn read_task_completion_candidate_file(
        path: &Path,
        _consent: PrivateReadConsent,
    ) -> Result<TaskCompletionRequest, ApplicationError> {
        Ok(read_task_completion_file(path)?)
    }

    pub fn read_task_completion_candidate_stdin<R: Read>(
        reader: R,
    ) -> Result<TaskCompletionRequest, ApplicationError> {
        Ok(read_task_completion_stdin(reader)?)
    }

    pub fn write_private_json_candidate<T: Serialize>(
        path: &Path,
        value: &T,
    ) -> Result<(), ApplicationError> {
        Ok(write_private_json_new(path, value)?)
    }
}
