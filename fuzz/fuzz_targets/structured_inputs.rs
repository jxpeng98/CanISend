#![no_main]

use canisend_contracts::{
    ApplicationPlanCandidate, DiscoveryBatch, DocumentCandidate, ReviewCandidate,
    ReviewDispositionCandidate, TaskCompletionRequest,
};
use libfuzzer_sys::fuzz_target;

const MAX_FUZZ_INPUT_BYTES: usize = 1024 * 1024;

fuzz_target!(|input: &[u8]| {
    let input = &input[..input.len().min(MAX_FUZZ_INPUT_BYTES)];
    let _ = serde_json::from_slice::<TaskCompletionRequest>(input);
    let _ = serde_json::from_slice::<DiscoveryBatch>(input);
    let _ = serde_json::from_slice::<ApplicationPlanCandidate>(input);
    let _ = serde_json::from_slice::<DocumentCandidate>(input);
    let _ = serde_json::from_slice::<ReviewCandidate>(input);
    let _ = serde_json::from_slice::<ReviewDispositionCandidate>(input);
});
