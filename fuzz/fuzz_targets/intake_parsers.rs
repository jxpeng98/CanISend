#![no_main]

use canisend_contracts::UtcTimestamp;
use canisend_io::{
    normalize_html_document, normalize_utf8_text, parse_csv_batch, parse_host_agent_batch,
    parse_json_batch, validate_rendered_pdf,
};
use libfuzzer_sys::fuzz_target;

const MAX_FUZZ_INPUT_BYTES: usize = 256 * 1024;

fuzz_target!(|input: &[u8]| {
    let input = &input[..input.len().min(MAX_FUZZ_INPUT_BYTES)];
    let observed_at =
        UtcTimestamp::try_new("2026-07-18T00:00:00Z").expect("fixed fuzz timestamp is valid");
    let _ = normalize_utf8_text(input);
    let _ = normalize_html_document(input);
    let _ = parse_csv_batch(input, "fuzz-input", None, observed_at);
    let _ = parse_json_batch(input);
    let _ = parse_host_agent_batch(input);
    let _ = validate_rendered_pdf(input);
});
