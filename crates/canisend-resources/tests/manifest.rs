use std::{fs, str::FromStr};

use canisend_resources::{
    AgentHost, AgentPackManifest, AgentSkillsInstallState, AgentSkillsManifest,
    ResourceCatalogManifest, ResourceId, ResourceKind, export_agent_pack, export_all,
    export_catalog, get, install_agent_skills, manifest, verify,
};
use sha2::{Digest, Sha256};

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

#[test]
fn catalog_export_is_complete_integrity_bound_and_create_new() {
    let parent = std::env::temp_dir().join(format!(
        "canisend-resource-catalog-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&parent);
    fs::create_dir(&parent).expect("catalog parent");
    let root = parent.join("catalog");
    let exported = export_catalog(&ResourceId::ALL, &root).expect("catalog export");
    let manifest: ResourceCatalogManifest =
        serde_json::from_slice(&fs::read(&exported.manifest_path).expect("manifest bytes"))
            .expect("manifest JSON");
    assert_eq!(manifest, exported.manifest);
    assert_eq!(manifest.format, "canisend.resource-catalog-export/v1");
    assert_eq!(manifest.resource_format, "canisend.resources/v2");
    assert_eq!(manifest.files.len(), ResourceId::ALL.len());
    for entry in &manifest.files {
        let bytes = fs::read(root.join(&entry.path)).expect("catalog file");
        assert_eq!(bytes.len(), entry.size);
        assert_eq!(hex::encode(Sha256::digest(bytes)), entry.sha256);
    }
    assert!(export_catalog(&ResourceId::ALL, &root).is_err());

    let duplicate_root = parent.join("duplicate");
    fs::create_dir(&duplicate_root).expect("empty duplicate root");
    assert!(
        export_catalog(
            &[
                ResourceId::TemplateCoverLetter,
                ResourceId::TemplateCoverLetter
            ],
            &duplicate_root,
        )
        .is_err()
    );
    assert!(
        fs::read_dir(&duplicate_root)
            .expect("duplicate root")
            .next()
            .is_none()
    );
    assert!(duplicate_root.is_dir());
    assert!(export_catalog(&[], &parent.join("empty-selection")).is_err());
    assert!(!parent.join("empty-selection").exists());
    assert!(export_catalog(&ResourceId::ALL, &parent.join(".canisend/catalog")).is_err());
    assert!(!parent.join(".canisend").exists());

    fs::remove_dir_all(parent).expect("cleanup catalog");
}

#[test]
fn host_packs_are_self_contained_versioned_and_integrity_manifested() {
    let parent =
        std::env::temp_dir().join(format!("canisend-agent-pack-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&parent);
    fs::create_dir(&parent).expect("pack test parent");
    for host in [AgentHost::Codex, AgentHost::Claude, AgentHost::Generic] {
        let root = parent.join(host.as_str());
        let exported = export_agent_pack(host, &root).expect("agent pack");
        let manifest: AgentPackManifest =
            serde_json::from_slice(&fs::read(&exported.manifest_path).expect("manifest bytes"))
                .expect("manifest JSON");
        assert_eq!(manifest, exported.manifest);
        assert_eq!(manifest.format, "canisend.agent-pack/v2");
        assert_eq!(manifest.protocol, "canisend.agent/v2");
        let expected_files = if host == AgentHost::Codex { 39 } else { 35 };
        assert_eq!(manifest.files.len(), expected_files);
        let skill_root = match host {
            AgentHost::Codex => ".agents/skills",
            AgentHost::Claude => ".claude/skills",
            AgentHost::Generic => "skills",
        };
        assert!(
            root.join(skill_root)
                .join("canisend-application/SKILL.md")
                .is_file()
        );
        assert_eq!(
            root.join(skill_root)
                .join("canisend-application/agents/openai.yaml")
                .is_file(),
            host == AgentHost::Codex
        );
        for entry in &manifest.files {
            let bytes = fs::read(root.join(&entry.path)).expect("pack file");
            assert_eq!(bytes.len(), entry.size);
            assert_eq!(hex::encode(Sha256::digest(bytes)), entry.sha256);
        }
        assert!(export_agent_pack(host, &root).is_err());
    }
    assert!(export_agent_pack(AgentHost::Generic, &parent.join(".canisend/pack")).is_err());
    fs::remove_dir_all(parent).expect("cleanup");
}

#[test]
fn agent_skills_install_is_idempotent_upgradeable_and_edit_safe() {
    let root =
        std::env::temp_dir().join(format!("canisend-agent-skills-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).expect("workspace root");

    let installed = install_agent_skills(AgentHost::Codex, &root).expect("install skills");
    assert_eq!(installed.state, AgentSkillsInstallState::Installed);
    assert_eq!(installed.files.len(), 8);
    assert!(
        root.join(".agents/skills/canisend-application/SKILL.md")
            .is_file()
    );
    assert!(
        root.join(".agents/skills/canisend-application/agents/openai.yaml")
            .is_file()
    );

    let unchanged = install_agent_skills(AgentHost::Codex, &root).expect("check skills");
    assert_eq!(unchanged.state, AgentSkillsInstallState::UpToDate);

    let managed_path = root.join(".agents/skills/canisend-job-intake/SKILL.md");
    let old_bytes = b"previous managed skill";
    fs::write(&managed_path, old_bytes).expect("old managed bytes");
    let mut old_manifest: AgentSkillsManifest =
        serde_json::from_slice(&fs::read(&installed.manifest_path).expect("manifest bytes"))
            .expect("manifest JSON");
    let entry = old_manifest
        .files
        .iter_mut()
        .find(|file| file.path.ends_with("canisend-job-intake/SKILL.md"))
        .expect("managed entry");
    entry.sha256 = hex::encode(Sha256::digest(old_bytes));
    fs::write(
        &installed.manifest_path,
        serde_json::to_vec_pretty(&old_manifest).expect("old manifest bytes"),
    )
    .expect("old manifest");
    let updated = install_agent_skills(AgentHost::Codex, &root).expect("upgrade skills");
    assert_eq!(updated.state, AgentSkillsInstallState::Updated);
    assert_ne!(fs::read(&managed_path).expect("updated skill"), old_bytes);

    fs::write(&managed_path, b"user modified skill").expect("user edit");
    assert!(install_agent_skills(AgentHost::Codex, &root).is_err());
    assert_eq!(
        fs::read(&managed_path).expect("preserved user edit"),
        b"user modified skill"
    );

    let claude_root = root.join("claude-workspace");
    fs::create_dir(&claude_root).expect("Claude root");
    let claude =
        install_agent_skills(AgentHost::Claude, &claude_root).expect("Claude skills install");
    assert_eq!(claude.files.len(), 4);
    assert!(
        claude_root
            .join(".claude/skills/canisend-application/SKILL.md")
            .is_file()
    );
    assert!(
        !claude_root
            .join(".claude/skills/canisend-application/agents/openai.yaml")
            .exists()
    );

    fs::remove_dir_all(root).expect("cleanup");
}
