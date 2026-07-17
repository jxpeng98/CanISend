use std::{
    fs::{self, File},
    io::Read,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::{IoAdapterError, MAX_LOCAL_SOURCE_BYTES};

pub const MAX_PDF_PAGES: usize = 100;
const MAX_PDF_TEXT_BYTES: usize = 16 * 1024 * 1024;
const PDF_TIME_BUDGET: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfTextDocument {
    pub original_bytes: Vec<u8>,
    pub normalized_text: String,
    pub page_count: usize,
}

pub fn read_local_pdf(path: &Path) -> Result<PdfTextDocument, IoAdapterError> {
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
    {
        return Err(IoAdapterError::UnsupportedLocalType(path.to_path_buf()));
    }
    let bytes = read_regular_file(path, MAX_LOCAL_SOURCE_BYTES)?;
    extract_pdf_text(bytes)
}

pub fn extract_pdf_text(original_bytes: Vec<u8>) -> Result<PdfTextDocument, IoAdapterError> {
    if original_bytes.len() < 8 || !original_bytes.starts_with(b"%PDF-") {
        return Err(IoAdapterError::PdfMalformed(
            "missing PDF signature".to_owned(),
        ));
    }
    let started = Instant::now();
    let document = lopdf::Document::load_mem(&original_bytes)
        .map_err(|error| IoAdapterError::PdfMalformed(error.to_string()))?;
    if document.is_encrypted() {
        return Err(IoAdapterError::PdfEncrypted);
    }
    let page_count = document.get_pages().len();
    if page_count == 0 {
        return Err(IoAdapterError::PdfMalformed(
            "document contains no pages".to_owned(),
        ));
    }
    if page_count > MAX_PDF_PAGES {
        return Err(IoAdapterError::PdfPageLimit {
            limit: MAX_PDF_PAGES,
            actual: page_count,
        });
    }
    drop(document);

    let extracted = catch_unwind(AssertUnwindSafe(|| {
        pdf_extract::extract_text_from_mem_by_pages(&original_bytes)
    }))
    .map_err(|_| IoAdapterError::PdfMalformed("extractor panicked".to_owned()))?
    .map_err(|error| IoAdapterError::PdfMalformed(error.to_string()))?;
    if started.elapsed() > PDF_TIME_BUDGET {
        return Err(IoAdapterError::PdfTimeBudget);
    }
    if extracted.len() != page_count {
        return Err(IoAdapterError::PdfMalformed(format!(
            "extractor returned {} of {page_count} pages",
            extracted.len()
        )));
    }

    let mut normalized = String::new();
    let mut has_text = false;
    for (index, page) in extracted.into_iter().enumerate() {
        let page = normalize_page(&page);
        has_text |= !page.trim().is_empty();
        use std::fmt::Write as _;
        writeln!(normalized, "--- Page {} ---", index + 1)
            .map_err(|error| IoAdapterError::PdfMalformed(error.to_string()))?;
        if !page.is_empty() {
            normalized.push_str(&page);
            normalized.push('\n');
        }
        normalized.push('\n');
        if normalized.len() > MAX_PDF_TEXT_BYTES {
            return Err(IoAdapterError::InputTooLarge {
                limit: u64::try_from(MAX_PDF_TEXT_BYTES).expect("PDF text limit fits u64"),
            });
        }
    }
    if !has_text {
        return Err(IoAdapterError::PdfTextUnavailable);
    }
    Ok(PdfTextDocument {
        original_bytes,
        normalized_text: normalized,
        page_count,
    })
}

fn normalize_page(page: &str) -> String {
    page.replace("\r\n", "\n")
        .replace(['\r', '\u{000c}'], "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

fn read_regular_file(path: &Path, limit: u64) -> Result<Vec<u8>, IoAdapterError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| IoAdapterError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(IoAdapterError::UnsafeLocalFile(path.to_path_buf()));
    }
    if metadata.len() > limit {
        return Err(IoAdapterError::InputTooLarge { limit });
    }
    let mut bytes = Vec::with_capacity(
        usize::try_from(metadata.len()).map_err(|_| IoAdapterError::InputTooLarge { limit })?,
    );
    File::open(path)
        .map_err(|source| IoAdapterError::Io {
            path: PathBuf::from(path),
            source,
        })?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| IoAdapterError::Io {
            path: PathBuf::from(path),
            source,
        })?;
    if u64::try_from(bytes.len()).expect("vector length fits u64") > limit {
        return Err(IoAdapterError::InputTooLarge { limit });
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use lopdf::{
        Document, EncryptionState, EncryptionVersion, Object, Permissions, Stream,
        content::{Content, Operation},
        dictionary,
    };

    use super::extract_pdf_text;
    use crate::IoAdapterError;

    #[test]
    fn text_pdf_is_extracted_by_page_and_invalid_inputs_are_typed() {
        let pdf = make_pdf(Some("Lecturer in Economics"));
        let extracted = extract_pdf_text(pdf).expect("text PDF");
        assert_eq!(extracted.page_count, 1);
        assert!(extracted.normalized_text.contains("--- Page 1 ---"));
        assert!(extracted.normalized_text.contains("Lecturer in Economics"));

        let layout = extract_pdf_text(make_layout_pdf()).expect("layout PDF");
        for expected in [
            "University X",
            "Essential criteria",
            "Desirable criteria",
            "Teaching",
            "Research",
        ] {
            assert!(layout.normalized_text.contains(expected));
        }

        assert!(matches!(
            extract_pdf_text(b"not a PDF".to_vec()),
            Err(IoAdapterError::PdfMalformed(_))
        ));
        assert!(matches!(
            extract_pdf_text(make_pdf(None)),
            Err(IoAdapterError::PdfTextUnavailable)
        ));
        let mut truncated = make_pdf(Some("truncated"));
        truncated.truncate(truncated.len() / 2);
        assert!(matches!(
            extract_pdf_text(truncated),
            Err(IoAdapterError::PdfMalformed(_))
        ));
        assert!(matches!(
            extract_pdf_text(make_pdf_pages(101, Some("too many pages"))),
            Err(IoAdapterError::PdfPageLimit { .. })
        ));
        assert!(matches!(
            extract_pdf_text(make_encrypted_pdf()),
            Err(IoAdapterError::PdfEncrypted)
        ));
    }

    fn make_pdf(text: Option<&str>) -> Vec<u8> {
        make_pdf_pages(1, text)
    }

    fn make_pdf_pages(page_count: usize, text: Option<&str>) -> Vec<u8> {
        let operations = text.map_or_else(Vec::new, |text| {
            vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new("Td", vec![50.into(), 750.into()]),
                Operation::new("Tj", vec![Object::string_literal(text)]),
                Operation::new("ET", vec![]),
            ]
        });
        make_pdf_with_operations(page_count, operations)
    }

    fn make_layout_pdf() -> Vec<u8> {
        let mut operations = Vec::new();
        for (x, y, text) in [
            (50, 760, "University X"),
            (50, 720, "Essential criteria"),
            (50, 690, "Teaching"),
            (310, 720, "Desirable criteria"),
            (310, 690, "Research"),
        ] {
            operations.extend([
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 11.into()]),
                Operation::new("Td", vec![x.into(), y.into()]),
                Operation::new("Tj", vec![Object::string_literal(text)]),
                Operation::new("ET", vec![]),
            ]);
        }
        make_pdf_with_operations(1, operations)
    }

    fn make_pdf_with_operations(page_count: usize, operations: Vec<Operation>) -> Vec<u8> {
        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
            "Encoding" => "WinAnsiEncoding"
        });
        let resources_id = document.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });
        let content_id = document.add_object(Stream::new(
            dictionary! {},
            Content { operations }.encode().expect("content encoding"),
        ));
        let page_ids = (0..page_count)
            .map(|_| {
                document.add_object(dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                    "Contents" => content_id,
                    "Resources" => resources_id,
                    "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
                })
            })
            .collect::<Vec<_>>();
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => page_ids.into_iter().map(Object::from).collect::<Vec<_>>(),
                "Count" => i64::try_from(page_count).expect("page count fits i64"),
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);
        document.trailer.set(
            "ID",
            Object::Array(vec![
                Object::string_literal(b"CANISEND-PDF-TEST-A"),
                Object::string_literal(b"CANISEND-PDF-TEST-B"),
            ]),
        );
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("PDF serialization");
        bytes
    }

    fn make_encrypted_pdf() -> Vec<u8> {
        let bytes = make_pdf(Some("encrypted"));
        let mut document = Document::load_mem(&bytes).expect("load encryption fixture");
        let state = EncryptionState::try_from(EncryptionVersion::V1 {
            document: &document,
            owner_password: "owner",
            user_password: "user",
            permissions: Permissions::empty(),
        })
        .expect("encryption state");
        document.encrypt(&state).expect("encrypt fixture");
        let mut encrypted = Vec::new();
        document
            .save_to(&mut encrypted)
            .expect("encrypted PDF serialization");
        encrypted
    }
}
