use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

use canisend_contracts::TaskCompletionRequest;
use serde_json::Value;

use crate::IoAdapterError;

pub const MAX_TASK_COMPLETION_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_CRITERIA_BYTES: u64 = 4 * 1024 * 1024;

pub fn read_task_completion_file(path: &Path) -> Result<TaskCompletionRequest, IoAdapterError> {
    read_json_file(path, MAX_TASK_COMPLETION_BYTES)
}

pub fn read_criteria_file(path: &Path) -> Result<Value, IoAdapterError> {
    let value: Value = read_json_file(path, MAX_CRITERIA_BYTES)?;
    if !value.is_object() {
        return Err(IoAdapterError::CandidateInput(
            "criteria candidate must be one JSON object".to_owned(),
        ));
    }
    Ok(value)
}

fn read_json_file<T: serde::de::DeserializeOwned>(
    path: &Path,
    limit: u64,
) -> Result<T, IoAdapterError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| IoAdapterError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(IoAdapterError::UnsafeLocalFile(path.to_path_buf()));
    }
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        return Err(IoAdapterError::UnsupportedLocalType(path.to_path_buf()));
    }
    if metadata.len() > limit {
        return Err(IoAdapterError::InputTooLarge { limit });
    }
    let file = File::open(path).map_err(|source| IoAdapterError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_reader(file).map_err(|error| IoAdapterError::CandidateInput(error.to_string()))
}

pub fn read_task_completion_stdin<R: Read>(
    reader: R,
) -> Result<TaskCompletionRequest, IoAdapterError> {
    read_task_completion(reader)
}

fn read_task_completion<R: Read>(mut reader: R) -> Result<TaskCompletionRequest, IoAdapterError> {
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(MAX_TASK_COMPLETION_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| IoAdapterError::Io {
            path: "<stdin-or-candidate>".into(),
            source,
        })?;
    if u64::try_from(bytes.len()).expect("vector length fits u64") > MAX_TASK_COMPLETION_BYTES {
        return Err(IoAdapterError::InputTooLarge {
            limit: MAX_TASK_COMPLETION_BYTES,
        });
    }
    serde_json::from_slice(&bytes)
        .map_err(|error| IoAdapterError::CandidateInput(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Cursor};

    use super::{
        MAX_TASK_COMPLETION_BYTES, read_criteria_file, read_task_completion_file,
        read_task_completion_stdin,
    };

    const VALID: &str = r#"{
      "task_id":"019f2f55-7c00-7000-8000-000000000001",
      "lease_id":"019f2f55-7c00-7000-8000-000000000002",
      "expected_job_revision":1,
      "expected_inputs":[],
      "candidate":{}
    }"#;

    #[test]
    fn reads_bounded_completion_from_stdin() {
        let request = read_task_completion_stdin(Cursor::new(VALID)).expect("completion request");
        assert_eq!(request.expected_job_revision.get(), 1);
        let oversized = vec![b'x'; usize::try_from(MAX_TASK_COMPLETION_BYTES + 1).unwrap()];
        assert!(read_task_completion_stdin(Cursor::new(oversized)).is_err());
    }

    #[test]
    fn file_must_be_regular_json_and_not_a_symlink() {
        let root =
            std::env::temp_dir().join(format!("canisend-task-candidate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("test directory");
        let path = root.join("completion.json");
        fs::write(&path, VALID).expect("fixture");
        read_task_completion_file(&path).expect("regular JSON");
        assert!(read_criteria_file(&path).expect("JSON object").is_object());
        let unsupported = root.join("completion.txt");
        fs::write(&unsupported, VALID).expect("unsupported fixture");
        assert!(read_task_completion_file(&unsupported).is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let linked = root.join("linked.json");
            symlink(&path, &linked).expect("symlink");
            assert!(read_task_completion_file(&linked).is_err());
        }
        fs::remove_dir_all(root).expect("cleanup");
    }
}
