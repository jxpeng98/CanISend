use jsonschema::draft202012;
use rusqlite::{Connection, params};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::json;
use typst_as_lib::{TypstEngine, typst_kit_options::TypstKitFontOptions};

#[derive(JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    name: String,
    score: u8,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    prove_bundled_sqlite()?;
    prove_generated_schema_and_validation()?;
    let (pdf_bytes, pdf_extract_text, lopdf_text, lopdf_recovered_expected_text) =
        prove_embedded_typst_and_pdf_extraction()?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "probe": "canisend-r0-native",
            "ok": true,
            "sqlite_version": rusqlite::version(),
            "pdf_bytes": pdf_bytes,
            "pdf_extract_text": pdf_extract_text.trim(),
            "lopdf_text": lopdf_text.trim(),
            "lopdf_recovered_expected_text": lopdf_recovered_expected_text,
        }))?
    );
    Ok(())
}

fn prove_bundled_sqlite() -> Result<(), Box<dyn std::error::Error>> {
    let connection = Connection::open_in_memory()?;
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE probe (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
    )?;
    connection.execute("INSERT INTO probe (value) VALUES (?1)", params!["ok"])?;
    let value: String =
        connection.query_row("SELECT value FROM probe WHERE id = 1", [], |row| row.get(0))?;
    if value != "ok" {
        return Err("bundled SQLite round trip returned unexpected data".into());
    }
    Ok(())
}

fn prove_generated_schema_and_validation() -> Result<(), Box<dyn std::error::Error>> {
    let schema = schemars::schema_for!(Candidate);
    let schema_value = serde_json::to_value(schema)?;
    if !jsonschema::meta::is_valid(&schema_value) {
        return Err("generated schema did not pass meta-schema validation".into());
    }

    let valid = json!({"name": "synthetic", "score": 7});
    let invalid = json!({"name": "synthetic", "score": "seven"});
    if !draft202012::is_valid(&schema_value, &valid)
        || draft202012::is_valid(&schema_value, &invalid)
    {
        return Err("Draft 2020-12 validation result was incorrect".into());
    }
    Ok(())
}

fn prove_embedded_typst_and_pdf_extraction()
-> Result<(usize, String, String, bool), Box<dyn std::error::Error>> {
    const SOURCE: &str = r#"
#set page(width: 120mm, height: 60mm, margin: 10mm)
#set text(font: "Libertinus Serif", size: 11pt)
= CanISend Rust-native probe
Standalone embedded Typst rendering and PDF extraction succeeded.
"#;

    let engine = TypstEngine::builder()
        .main_file(SOURCE)
        .search_fonts_with(
            TypstKitFontOptions::default()
                .include_system_fonts(false)
                .include_embedded_fonts(true),
        )
        .build();

    let document = engine
        .compile()
        .output
        .map_err(|diagnostics| format!("embedded Typst compilation failed: {diagnostics:?}"))?;
    let pdf = typst_pdf::pdf(&document, &Default::default())
        .map_err(|diagnostics| format!("embedded Typst PDF export failed: {diagnostics:?}"))?;
    if !pdf.starts_with(b"%PDF-") {
        return Err("Typst output did not contain a PDF header".into());
    }

    let extracted = pdf_extract::extract_text_from_mem(&pdf)?;
    if !extracted.contains("CanISend Rust-native probe")
        || !extracted.contains("PDF extraction succeeded")
    {
        return Err("PDF extractor did not recover expected Typst text".into());
    }
    let document = lopdf::Document::load_mem(&pdf)?;
    let page_numbers = document.get_pages().keys().copied().collect::<Vec<_>>();
    let lopdf_text = document.extract_text(&page_numbers)?;
    let lopdf_recovered_expected_text = lopdf_text.contains("CanISend Rust-native probe")
        && lopdf_text.contains("PDF extraction succeeded");
    Ok((
        pdf.len(),
        extracted,
        lopdf_text,
        lopdf_recovered_expected_text,
    ))
}
