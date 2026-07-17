use std::{fs, str::FromStr};

use canisend_resources::{ResourceId, ResourceKind, export_all, get, manifest, verify};

#[test]
fn embedded_manifest_matches_resource_bytes() {
    verify().expect("embedded resources verify");
    let resources = manifest();

    assert_eq!(resources.len(), ResourceId::ALL.len());
    assert!(resources.len() >= 20);
    assert!(resources.iter().all(|resource| resource.sha256.len() == 64));
    assert!(
        resources
            .iter()
            .any(|resource| resource.kind == ResourceKind::Schema)
    );
    for id in ResourceId::ALL {
        assert_eq!(
            ResourceId::from_str(id.as_str()).expect("typed ID parses"),
            id
        );
        assert_eq!(get(id).descriptor.id, id.as_str());
    }
}

#[test]
fn export_all_reproduces_declared_resource_tree() {
    let root = std::env::temp_dir().join(format!("canisend-resource-test-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove prior test directory");
    }
    let paths = export_all(&root).expect("resources export");
    assert_eq!(paths.len(), ResourceId::ALL.len());
    for path in paths {
        assert!(path.is_file(), "missing exported file: {}", path.display());
    }
    fs::remove_dir_all(root).expect("remove test directory");
}
