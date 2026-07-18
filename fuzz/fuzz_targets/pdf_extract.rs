#![no_main]

use canisend_io::extract_pdf_text;
use libfuzzer_sys::fuzz_target;

const MAX_FUZZ_INPUT_BYTES: usize = 1024 * 1024;

fuzz_target!(|input: &[u8]| {
    let input = &input[..input.len().min(MAX_FUZZ_INPUT_BYTES)];
    let _ = extract_pdf_text(input.to_vec());
});
