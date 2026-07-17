use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use crate::IoAdapterError;

pub const MAX_LOCAL_SOURCE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalTextKind {
    Markdown,
    PlainText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTextDocument {
    pub path: PathBuf,
    pub kind: LocalTextKind,
    pub content_type: &'static str,
    pub original_bytes: Vec<u8>,
    pub normalized_text: String,
}

pub fn read_local_text(path: &Path) -> Result<LocalTextDocument, IoAdapterError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| IoAdapterError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(IoAdapterError::UnsafeLocalFile(path.to_path_buf()));
    }
    if metadata.len() > MAX_LOCAL_SOURCE_BYTES {
        return Err(IoAdapterError::InputTooLarge {
            limit: MAX_LOCAL_SOURCE_BYTES,
        });
    }
    let (kind, content_type) = match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("md") => {
            (LocalTextKind::Markdown, "text/markdown; charset=utf-8")
        }
        Some(extension) if extension.eq_ignore_ascii_case("txt") => {
            (LocalTextKind::PlainText, "text/plain; charset=utf-8")
        }
        _ => return Err(IoAdapterError::UnsupportedLocalType(path.to_path_buf())),
    };
    let mut file = File::open(path).map_err(|source| IoAdapterError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut original_bytes = Vec::with_capacity(usize::try_from(metadata.len()).map_err(|_| {
        IoAdapterError::InputTooLarge {
            limit: MAX_LOCAL_SOURCE_BYTES,
        }
    })?);
    file.by_ref()
        .take(MAX_LOCAL_SOURCE_BYTES + 1)
        .read_to_end(&mut original_bytes)
        .map_err(|source| IoAdapterError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if u64::try_from(original_bytes.len()).expect("vector length fits u64") > MAX_LOCAL_SOURCE_BYTES
    {
        return Err(IoAdapterError::InputTooLarge {
            limit: MAX_LOCAL_SOURCE_BYTES,
        });
    }
    let normalized_text = normalize_utf8_text(&original_bytes)?;
    Ok(LocalTextDocument {
        path: path.to_path_buf(),
        kind,
        content_type,
        original_bytes,
        normalized_text,
    })
}

pub fn normalize_utf8_text(bytes: &[u8]) -> Result<String, IoAdapterError> {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| IoAdapterError::InvalidTextEncoding)?;
    if text.contains('\0')
        || text
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(IoAdapterError::UnsafeTextControlCharacter);
    }
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut output = normalized
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    while output.ends_with('\n') {
        output.pop();
    }
    if output.trim().is_empty() {
        return Err(IoAdapterError::TextUnavailable);
    }
    output.push('\n');
    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{MAX_LOCAL_SOURCE_BYTES, normalize_utf8_text, read_local_text};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn normalization_is_utf8_only_and_deterministic() {
        assert_eq!(
            normalize_utf8_text(b"\xef\xbb\xbfTitle  \r\nBody\r\n").expect("valid text"),
            "Title\nBody\n"
        );
        assert!(normalize_utf8_text(&[0xff, 0xfe]).is_err());
        assert!(normalize_utf8_text(b"safe\0unsafe").is_err());
        assert!(normalize_utf8_text(b" \r\n\t").is_err());
    }

    #[test]
    fn local_reads_require_bounded_regular_supported_files() {
        let root = std::env::temp_dir().join(format!(
            "canisend-local-intake-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test directory");
        let markdown = root.join("advert.MD");
        fs::write(&markdown, b"Job advert\n").expect("markdown fixture");
        assert_eq!(
            read_local_text(&markdown)
                .expect("supported regular file")
                .normalized_text,
            "Job advert\n"
        );

        let unsupported = root.join("advert.html");
        fs::write(&unsupported, b"<p>advert</p>").expect("unsupported fixture");
        assert!(read_local_text(&unsupported).is_err());

        let oversized = root.join("oversized.txt");
        let oversized_file = fs::File::create(&oversized).expect("oversized fixture");
        oversized_file
            .set_len(MAX_LOCAL_SOURCE_BYTES + 1)
            .expect("sparse oversized fixture");
        assert!(read_local_text(&oversized).is_err());

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let linked = root.join("linked.md");
            symlink(&markdown, &linked).expect("symlink fixture");
            assert!(read_local_text(&linked).is_err());
        }
        fs::remove_dir_all(root).expect("remove test directory");
    }
}
