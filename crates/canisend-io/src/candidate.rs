use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path},
};

use canisend_contracts::TaskCompletionRequest;
use serde::Serialize;
use serde_json::Value;

use crate::IoAdapterError;

pub const MAX_TASK_COMPLETION_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_STRUCTURED_CANDIDATE_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_CRITERIA_BYTES: u64 = MAX_STRUCTURED_CANDIDATE_BYTES;

pub fn read_task_completion_file(path: &Path) -> Result<TaskCompletionRequest, IoAdapterError> {
    read_json_file(path, MAX_TASK_COMPLETION_BYTES)
}

pub fn read_criteria_file(path: &Path) -> Result<Value, IoAdapterError> {
    read_structured_candidate_file(path)
}

pub fn read_structured_candidate_file(path: &Path) -> Result<Value, IoAdapterError> {
    let value: Value = read_json_file(path, MAX_STRUCTURED_CANDIDATE_BYTES)?;
    if !value.is_object() {
        return Err(IoAdapterError::CandidateInput(
            "structured candidate must be one JSON object".to_owned(),
        ));
    }
    Ok(value)
}

pub fn write_private_json_new<T: Serialize>(path: &Path, value: &T) -> Result<(), IoAdapterError> {
    if path.components().any(|component| {
        matches!(component, Component::Normal(name) if name.to_string_lossy().eq_ignore_ascii_case(".canisend"))
    }) {
        return Err(IoAdapterError::UnsafeLocalFile(path.to_path_buf()));
    }
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        return Err(IoAdapterError::UnsupportedLocalType(path.to_path_buf()));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent_metadata = fs::symlink_metadata(parent).map_err(|source| IoAdapterError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err(IoAdapterError::UnsafeLocalFile(path.to_path_buf()));
    }
    let canonical_parent = fs::canonicalize(parent).map_err(|source| IoAdapterError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    if canonical_parent.components().any(|component| {
        matches!(component, Component::Normal(name) if name.to_string_lossy().eq_ignore_ascii_case(".canisend"))
    }) {
        return Err(IoAdapterError::UnsafeLocalFile(path.to_path_buf()));
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|source| IoAdapterError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| IoAdapterError::CandidateInput(error.to_string()))?;
    bytes.push(b'\n');
    file.write_all(&bytes)
        .map_err(|source| IoAdapterError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    file.sync_all().map_err(|source| IoAdapterError::Io {
        path: path.to_path_buf(),
        source,
    })
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
        MAX_TASK_COMPLETION_BYTES, read_criteria_file, read_structured_candidate_file,
        read_task_completion_file, read_task_completion_stdin, write_private_json_new,
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
        assert!(
            read_structured_candidate_file(&path)
                .expect("generic JSON object")
                .is_object()
        );
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

    #[test]
    fn private_json_export_is_create_new_and_rejects_private_state_paths() {
        let root =
            std::env::temp_dir().join(format!("canisend-private-candidate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".canisend")).expect("test directory");
        let path = root.join("candidate.json");
        write_private_json_new(&path, &serde_json::json!({"kind": "generic"}))
            .expect("new candidate export");
        assert!(write_private_json_new(&path, &serde_json::json!({})).is_err());
        assert!(
            write_private_json_new(&root.join(".canisend/private.json"), &serde_json::json!({}))
                .is_err()
        );
        assert_eq!(
            fs::read_to_string(path).expect("candidate body"),
            "{\n  \"kind\": \"generic\"\n}\n"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(root.join("candidate.json"))
                    .expect("candidate metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        fs::remove_dir_all(root).expect("cleanup");
    }
}
