use canisend_resources::{generic_agent_guide, manifest, verify};

#[test]
fn embedded_manifest_matches_resource_bytes() {
    verify().expect("embedded resources verify");
    let resources = manifest();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].size, generic_agent_guide().len());
    assert_eq!(resources[0].sha256.len(), 64);
}
