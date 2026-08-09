#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use canisend_contracts::{
    AGENT_PROTOCOL, AGENT_V4_PROTOCOL, AGENT_V4_SCHEMA_VERSION, OperationClass, OperationPackScope,
    OperationRegistry, OperationSurface, PUBLIC_SCHEMA_VERSION, WORKSPACE_FORMAT,
    WORKSPACE_V4_FORMAT, generate_agent_v4_schemas, generate_application_model_schemas,
    generate_public_schemas, generate_workflow_pack_schema, verify_agent_v4_schemas,
    verify_application_model_schemas, verify_public_schemas, verify_workflow_pack_schema,
};
use semver::Version;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use time::{Date, Duration, Month, OffsetDateTime, format_description::well_known::Rfc3339};

const RELEASE_TARGET_SCHEMA: &str = "canisend.release-targets/v1";
const RELEASE_MANIFEST_SCHEMA: &str = "canisend.release-manifest/v1";
const ALPHA_PACKAGE_CONTRACT_V2_SCHEMA: &str = "canisend.alpha-package-contract/v2";
const ALPHA_PACKAGE_CONTRACT_V3_SCHEMA: &str = "canisend.alpha-package-contract/v3";
const NATIVE_TEST_OWNERSHIP_SCHEMA: &str = "canisend.native-test-ownership/v1";
const TYPST_TEMPLATE_CONTRACT_SCHEMA: &str = "canisend.typst-template-contract/v2";
const DESKTOP_PROFILE_RECORD_SCHEMA: &str = "canisend.desktop-profile-record/v1";
const DESKTOP_PROFILE_SUMMARY_SCHEMA: &str = "canisend.desktop-profile-summary/v1";
const SCCACHE_STATS_SCHEMA: &str = "canisend.sccache-stats/v1";
const BETA_READINESS_SCHEMA: &str = "canisend.beta-readiness/v1";
const BETA_CONTRACT_FREEZE_SCHEMA: &str = "canisend.beta-contract-freeze/v1";
const CHANNEL_CANDIDATE_SOURCE_SCHEMA: &str = "canisend.channel-candidate-source/v1";
const STABLE_CHANNEL_PUBLICATION_SCHEMA: &str = "canisend.stable-channel-publication/v1";
const SIGNING_POLICY_SCHEMA: &str = "canisend.signing-policy/v2";
const SUPPORT_POLICY_SCHEMA: &str = "canisend.support-policy/v1";
const FEEDBACK_SNAPSHOT_SCHEMA: &str = "canisend.feedback-snapshot/v1";
const RELEASE_QUALIFICATION_SCHEMA: &str = "canisend.release-qualification/v1";
const RELEASE_STATUS_SCHEMA: &str = "canisend.release-status/v1";
const RELEASE_HISTORY_SCHEMA: &str = "canisend.release-history/v1";
const RELEASE_LINE_POLICY_SCHEMA: &str = "canisend.release-line-policy/v1";
const RELEASE_LINE_PLAN_SCHEMA: &str = "canisend.release-line-plan/v1";
const SVELTE_PARITY_SCHEMA: &str = "canisend.svelte-parity/v1";
const WORKSPACE_DEPENDENCY_POLICY_SCHEMA: &str = "canisend.workspace-dependency-policy/v1";
const DEPENDENCY_EXCEPTION_POLICY_SCHEMA: &str = "canisend.dependency-advisory-exceptions/v1";
const THIRD_PARTY_LOCK_FINGERPRINT_SCHEMA: &str = "canisend.third-party-lock-fingerprint/v1";
const SEMANTIC_PARITY_SCHEMA: &str = "canisend.semantic-parity/v1";
const STAGE_TRANSITION_POLICY_SCHEMA: &str = "canisend.stage-transition-policy/v1";
const STAGE_TRANSITION_PLAN_SCHEMA: &str = "canisend.stage-transition-plan/v1";
const FEATURE_FREEZE_PLAN_SCHEMA: &str = "canisend.feature-freeze-plan/v1";
const BETA_QUALIFICATION_PLAN_SCHEMA: &str = "canisend.beta-qualification-plan/v1";
const RC_QUALIFICATION_PLAN_SCHEMA: &str = "canisend.rc-qualification-plan/v1";
const FEATURE_FREEZE_EXCEPTIONS_SCHEMA: &str = "canisend.feature-freeze-exceptions/v1";
const PACKAGE_MANAGER_QUALIFICATION_POLICY_SCHEMA: &str =
    "canisend.package-manager-qualification-policy/v1";
const PACKAGE_MANAGER_QUALIFICATION_SCHEMA: &str = "canisend.package-manager-qualification/v1";
const PACKAGE_MANAGER_QUALIFICATION_PLAN_SCHEMA: &str =
    "canisend.package-manager-qualification-plan/v1";
const UPGRADE_QUALIFICATION_POLICY_SCHEMA: &str = "canisend.upgrade-qualification-policy/v1";
const UPGRADE_QUALIFICATION_SCHEMA: &str = "canisend.upgrade-qualification/v1";
const UPGRADE_QUALIFICATION_PLAN_SCHEMA: &str = "canisend.upgrade-qualification-plan/v1";
const DOCUMENTATION_UNINSTALL_POLICY_SCHEMA: &str = "canisend.documentation-uninstall-policy/v1";
const DOCUMENTATION_UNINSTALL_SCHEMA: &str = "canisend.documentation-uninstall/v1";
const DOCUMENTATION_UNINSTALL_PLAN_SCHEMA: &str = "canisend.documentation-uninstall-plan/v1";
const RELEASE_NOTES_POLICY_SCHEMA: &str = "canisend.release-notes-policy/v1";
const RELEASE_NOTES_QUALIFICATION_PLAN_SCHEMA: &str =
    "canisend.release-notes-qualification-plan/v1";
const CODE_SIGNING_EVIDENCE_SCHEMA: &str = "canisend.code-signing-evidence/v2";
const DOMAIN_COUPLING_INVENTORY_SCHEMA: &str = "canisend.domain-coupling-inventory/v1";
const DECLARED_RUST_VERSION: &str = "1.97";
const PINNED_RUST_TOOLCHAIN: &str = "1.97.0";
const FUZZ_TOOLCHAIN: &str = "nightly-2026-07-01";
const CARGO_FUZZ_VERSION: &str = "0.13.2";
const WINGET_MANIFEST_VERSION: &str = "1.10.0";
const GPL_LICENSE: &str = "GPL-3.0-only";
const FIRST_GPL_PUBLIC_VERSION: &str = "1.0.0-alpha.6";
const FIRST_ALPHA_PACKAGE_V3_VERSION: &str = "1.0.0-alpha.6";
const BETA_READINESS_MAX_AGE_HOURS: i64 = 24;
const NATIVE_ALPHA_TAG: &str = "v0.7.0-alpha.1";
const NATIVE_ALPHA_SOURCE: &str = "4cec4ec48cc2e96f3798dde0b438d3aaa617a2f8";
const FROZEN_MIGRATIONS_THROUGH: u32 = 13;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<String>) -> Result<(), String> {
    match arguments.as_slice() {
        [area, command] if area == "schemas" && command == "check" => check_schemas(),
        [area, command] if area == "schemas" && command == "write" => write_schemas(),
        [area, command] if area == "resources" && command == "check" => check_resources(),
        [area, command] if area == "docs" && command == "check" => check_documentation(),
        [area, command] if area == "scope" && command == "check" => {
            check_domain_coupling_inventory()
        }
        [area, command, json_flag]
            if area == "scope" && command == "inventory" && json_flag == "--json" =>
        {
            print_domain_coupling_inventory()
        }
        [area, command] if area == "desktop" && command == "parity" => check_svelte_parity(),
        [area, command] if area == "operations" && command == "check" => {
            check_operation_registry()
        }
        [area, command] if area == "approvals" && command == "check" => check_approval_broker(),
        [area, command] if area == "semantics" && command == "check" => check_semantic_parity(),
        [area, command] if area == "semantics" && command == "uncovered" => {
            list_uncovered_semantic_bindings()
        }
        [area, command] if area == "architecture" && command == "graph-check" => {
            check_workspace_dependency_graph()
        }
        [area, command] if area == "desktop" && command == "template-audit" => {
            check_typst_template_contract()
        }
        [area, command] if area == "release" && command == "alpha-package-bindings" => {
            print_alpha_package_contract_bindings()
        }
        [area, command, target, candidate, opt_level, lto, host, output]
            if area == "desktop" && command == "profile-record" =>
        {
            write_desktop_profile_record(
                target,
                candidate,
                opt_level,
                lto,
                Path::new(host),
                Path::new(output),
            )
        }
        [area, command, release, size_s, size_z, size_z_fat, output]
            if area == "desktop" && command == "profile-summary" =>
        {
            write_desktop_profile_summary(
                [release, size_s, size_z, size_z_fat].map(Path::new),
                Path::new(output),
            )
        }
        [area, command, target, profile, package_format, host, payload, frontend, artifact, output]
            if area == "desktop" && command == "size-record" =>
        {
            write_desktop_size_record(
                target,
                profile,
                package_format,
                Path::new(host),
                Path::new(payload),
                (frontend != "-").then(|| Path::new(frontend)),
                (artifact != "-").then(|| Path::new(artifact)),
                Path::new(output),
            )
        }
        [area, command] if area == "release" && command == "check" => {
            check_schemas()?;
            check_resources()?;
            check_domain_coupling_inventory()?;
            check_documentation()?;
            check_release_notes_policy()?;
            check_property_test_policy()?;
            check_fuzz_policy()?;
            check_internal_dependency_versions()?;
            check_dependency_assurance()?;
            check_rust_toolchain_alignment()?;
            check_desktop_distribution_versions()?;
            check_beta_readiness()?;
            check_beta_contract_freeze()?;
            check_channel_candidates()?;
            check_package_manager_qualification_policy()?;
            check_upgrade_qualification_policy()?;
            check_documentation_uninstall_policy()?;
            check_signing_policy()?;
            check_release_line_history()?;
            check_stage_transition_policy()?;
            check_support_policy()?;
            check_release_feedback()?;
            check_release_qualification()?;
            check_release_status()?;
            check_workspace_dependency_graph()?;
            check_operation_registry()?;
            check_approval_broker()?;
            check_semantic_parity()?;
            check_cli_gui_parity()?;
            check_svelte_parity()?;
            check_alpha_package_contract()?;
            check_native_test_ownership()?;
            check_release_contract()
        }
        [area, command] if area == "dependencies" && command == "check" => {
            check_dependency_assurance()
        }
        [area, command] if area == "dependencies" && command == "fingerprint" => {
            print_third_party_lock_fingerprint()
        }
        [area, command, json_flag]
            if area == "release" && command == "status" && json_flag == "--json" =>
        {
            print_release_status()
        }
        [area, command] if area == "release" && command == "freeze-candidate" => {
            let candidate = build_beta_contract_freeze()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&candidate)
                    .map_err(|error| format!("could not serialize freeze candidate: {error}"))?
            );
            Ok(())
        }
        [area, command, tag] if area == "release" && command == "validate-tag" => {
            validate_release_tag(tag).map(|_| ())
        }
        [area, command, path] if area == "release" && command == "verify-beta-readiness" => {
            check_beta_readiness_file(Path::new(path))
        }
        [area, command, snapshot, roadmap]
            if area == "release" && command == "verify-feedback-candidate" =>
        {
            check_release_feedback_files(Path::new(snapshot), Path::new(roadmap))
        }
        [area, command, tag] if area == "release" && command == "prepare-stage" => {
            prepare_stage_transition(tag, false)
        }
        [area, command, tag, write]
            if area == "release" && command == "prepare-stage" && write == "--write" =>
        {
            prepare_stage_transition(tag, true)
        }
        [area, command, tag] if area == "release" && command == "activate-line" => {
            activate_release_line(tag, false)
        }
        [area, command, tag, write]
            if area == "release" && command == "activate-line" && write == "--write" =>
        {
            activate_release_line(tag, true)
        }
        [area, command, baseline]
            if area == "release" && command == "activate-feature-freeze" =>
        {
            activate_feature_freeze(baseline, false)
        }
        [area, command, baseline, write]
            if area == "release" && command == "activate-feature-freeze" && write == "--write" =>
        {
            activate_feature_freeze(baseline, true)
        }
        [area, command, tag, run_id, assets]
            if area == "release" && command == "record-beta-qualification" =>
        {
            record_beta_qualification(tag, run_id, Path::new(assets), false)
        }
        [area, command, tag, run_id, assets, write]
            if area == "release"
                && command == "record-beta-qualification"
                && write == "--write" =>
        {
            record_beta_qualification(tag, run_id, Path::new(assets), true)
        }
        [area, command, tag, run_id, assets]
            if area == "release" && command == "record-rc-qualification" =>
        {
            record_rc_qualification(tag, run_id, Path::new(assets), false)
        }
        [area, command, tag, run_id, assets, write]
            if area == "release"
                && command == "record-rc-qualification"
                && write == "--write" =>
        {
            record_rc_qualification(tag, run_id, Path::new(assets), true)
        }
        [area, command, tag, assets, reviewer]
            if area == "release" && command == "record-release-notes-qualification" =>
        {
            record_release_notes_qualification(
                tag,
                Path::new(assets),
                reviewer,
                false,
            )
        }
        [area, command, tag, assets, reviewer, write]
            if area == "release"
                && command == "record-release-notes-qualification"
                && write == "--write" =>
        {
            record_release_notes_qualification(tag, Path::new(assets), reviewer, true)
        }
        [area, command, output] if area == "release" && command == "sbom" => {
            write_release_sbom(Path::new(output))
        }
        [area, command, tag, commit, artifacts, output]
            if area == "release" && command == "assemble" =>
        {
            assemble_release(tag, commit, Path::new(artifacts), Path::new(output))
        }
        [area, command, tag, directory] if area == "release" && command == "verify" => {
            verify_release(tag, Path::new(directory))
        }
        [area, command, tag, commit, directory]
            if area == "release" && command == "verify-candidate" =>
        {
            verify_release_candidate(tag, commit, Path::new(directory))
        }
        [area, command, tag, assets, output]
            if area == "release" && command == "channels" =>
        {
            write_channel_candidates(tag, Path::new(assets), Path::new(output))
        }
        [area, command, tag, target, evidence, binary, archive]
            if area == "release" && command == "bind-signing-evidence" =>
        {
            bind_signing_evidence(
                tag,
                target,
                Path::new(evidence),
                Path::new(binary),
                Path::new(archive),
            )
        }
        [area, command, from_tag, to_tag, evidence]
            if area == "release" && command == "verify-package-evidence" =>
        {
            verify_package_manager_evidence(from_tag, to_tag, Path::new(evidence)).map(|_| ())
        }
        [area, command, from_tag, to_tag, evidence]
            if area == "release" && command == "record-package-qualification" =>
        {
            record_package_manager_qualification(
                from_tag,
                to_tag,
                Path::new(evidence),
                false,
            )
        }
        [area, command, from_tag, to_tag, evidence, write]
            if area == "release"
                && command == "record-package-qualification"
                && write == "--write" =>
        {
            record_package_manager_qualification(
                from_tag,
                to_tag,
                Path::new(evidence),
                true,
            )
        }
        [area, command, from_tag, to_tag, evidence]
            if area == "release" && command == "verify-upgrade-evidence" =>
        {
            verify_upgrade_qualification_evidence(from_tag, to_tag, Path::new(evidence))
                .map(|_| ())
        }
        [area, command, tag, assets, evidence]
            if area == "release" && command == "verify-documentation-evidence" =>
        {
            verify_documentation_uninstall_evidence(
                tag,
                Path::new(assets),
                Path::new(evidence),
            )
            .map(|_| ())
        }
        [area, command, tag, assets, evidence]
            if area == "release" && command == "record-documentation-qualification" =>
        {
            record_documentation_uninstall_qualification(
                tag,
                Path::new(assets),
                Path::new(evidence),
                false,
            )
        }
        [area, command, tag, assets, evidence, write]
            if area == "release"
                && command == "record-documentation-qualification"
                && write == "--write" =>
        {
            record_documentation_uninstall_qualification(
                tag,
                Path::new(assets),
                Path::new(evidence),
                true,
            )
        }
        [area, command, from_tag, to_tag, evidence]
            if area == "release" && command == "record-upgrade-qualification" =>
        {
            record_upgrade_qualification(
                from_tag,
                to_tag,
                Path::new(evidence),
                false,
            )
        }
        [area, command, from_tag, to_tag, evidence, write]
            if area == "release"
                && command == "record-upgrade-qualification"
                && write == "--write" =>
        {
            record_upgrade_qualification(from_tag, to_tag, Path::new(evidence), true)
        }
        [area, command, from_tag, from_assets, to_tag, to_assets]
            if area == "release" && command == "verify-package-candidates" =>
        {
            verify_package_candidate_pair(
                from_tag,
                Path::new(from_assets),
                to_tag,
                Path::new(to_assets),
            )
        }
        _ => Err(
            "usage: cargo run -p xtask -- schemas <check|write> | <resources|docs> check | scope <check|inventory --json> | desktop <parity|template-audit|profile-record TARGET CANDIDATE OPT_LEVEL LTO HOST OUTPUT|profile-summary RELEASE SIZE_S SIZE_Z SIZE_Z_FAT OUTPUT|size-record TARGET PROFILE FORMAT HOST PAYLOAD FRONTEND|- ARTIFACT|- OUTPUT> | \
             release <check|status --json|freeze-candidate|validate-tag TAG|verify-beta-readiness FILE|verify-feedback-candidate SNAPSHOT ROADMAP|prepare-stage TAG [--write]|activate-feature-freeze COMMIT [--write]|record-beta-qualification TAG RUN_ID ASSETS [--write]|record-rc-qualification TAG RUN_ID ASSETS [--write]|record-release-notes-qualification TAG ASSETS REVIEWER [--write]|record-upgrade-qualification FROM_TAG TO_TAG EVIDENCE [--write]|record-documentation-qualification TAG ASSETS EVIDENCE [--write]|record-package-qualification FROM_TAG TO_TAG EVIDENCE [--write]|sbom OUTPUT|assemble TAG COMMIT ARTIFACTS OUTPUT|verify TAG DIRECTORY|verify-candidate TAG COMMIT DIRECTORY|channels TAG ASSETS OUTPUT|bind-signing-evidence TAG TARGET EVIDENCE BINARY ARCHIVE|verify-package-candidates FROM_TAG FROM_ASSETS TO_TAG TO_ASSETS|verify-package-evidence FROM_TAG TO_TAG DIRECTORY|verify-upgrade-evidence FROM_TAG TO_TAG DIRECTORY|verify-documentation-evidence TAG ASSETS EVIDENCE>"
                .to_owned(),
        ),
    }
}

fn check_schemas() -> Result<(), String> {
    verify_public_schemas()?;
    verify_agent_v4_schemas()?;
    verify_application_model_schemas()?;
    verify_workflow_pack_schema()?;
    let expected = generate_public_schemas();
    let directory = schema_directory();
    let mut expected_names = BTreeSet::new();
    for schema in expected {
        let file_name = schema.id.file_name();
        expected_names.insert(file_name.clone());
        let path = directory.join(file_name);
        let actual = fs::read_to_string(&path).map_err(|error| {
            format!("generated schema is missing at {}: {error}", path.display())
        })?;
        if actual != schema.canonical_json() {
            return Err(format!(
                "generated schema drift at {}; run `cargo run -p xtask -- schemas write`",
                path.display()
            ));
        }
    }
    let actual_names = json_files(&directory)?;
    if actual_names != expected_names {
        return Err(format!(
            "generated schema file set differs: expected {expected_names:?}, found {actual_names:?}"
        ));
    }
    let agent_v4 = generate_agent_v4_schemas();
    let agent_v4_directory = agent_v4_schema_directory();
    let mut expected_agent_v4_names = BTreeSet::new();
    for schema in agent_v4 {
        let file_name = schema.id.file_name();
        expected_agent_v4_names.insert(file_name.clone());
        let path = agent_v4_directory.join(file_name);
        let actual = fs::read_to_string(&path).map_err(|error| {
            format!(
                "generated Agent v4 schema is missing at {}: {error}",
                path.display()
            )
        })?;
        if actual != schema.canonical_json() {
            return Err(format!(
                "generated Agent v4 schema drift at {}; run `cargo run -p xtask -- schemas write`",
                path.display()
            ));
        }
    }
    let actual_agent_v4_names = json_files(&agent_v4_directory)?;
    if actual_agent_v4_names != expected_agent_v4_names {
        return Err(format!(
            "generated Agent v4 schema file set differs: expected {expected_agent_v4_names:?}, found {actual_agent_v4_names:?}"
        ));
    }
    let application_model = generate_application_model_schemas();
    let application_model_directory = application_model_schema_directory();
    let mut expected_application_model_names = BTreeSet::new();
    for schema in application_model {
        let file_name = schema.id.file_name();
        expected_application_model_names.insert(file_name.clone());
        let path = application_model_directory.join(file_name);
        let actual = fs::read_to_string(&path).map_err(|error| {
            format!(
                "generated application-model schema is missing at {}: {error}",
                path.display()
            )
        })?;
        if actual != schema.canonical_json() {
            return Err(format!(
                "generated application-model schema drift at {}; run `cargo run -p xtask -- schemas write`",
                path.display()
            ));
        }
    }
    let actual_application_model_names = json_files(&application_model_directory)?;
    if actual_application_model_names != expected_application_model_names {
        return Err(format!(
            "generated application-model schema file set differs: expected {expected_application_model_names:?}, found {actual_application_model_names:?}"
        ));
    }
    let workflow_pack = generate_workflow_pack_schema();
    let workflow_pack_directory = workflow_pack_schema_directory();
    let workflow_pack_path = workflow_pack_directory.join(workflow_pack.file_name());
    let workflow_pack_actual = fs::read_to_string(&workflow_pack_path).map_err(|error| {
        format!(
            "generated workflow-pack schema is missing at {}: {error}",
            workflow_pack_path.display()
        )
    })?;
    if workflow_pack_actual != workflow_pack.canonical_json() {
        return Err(format!(
            "generated workflow-pack schema drift at {}; run `cargo run -p xtask -- schemas write`",
            workflow_pack_path.display()
        ));
    }
    let workflow_pack_names = json_files(&workflow_pack_directory)?;
    let expected_workflow_pack_names = BTreeSet::from([workflow_pack.file_name().to_owned()]);
    if workflow_pack_names != expected_workflow_pack_names {
        return Err(format!(
            "generated workflow-pack schema file set differs: expected {expected_workflow_pack_names:?}, found {workflow_pack_names:?}"
        ));
    }
    println!(
        "schemas: ok ({} public + {} Agent v4 + {} application-v3 + 1 workflow-pack)",
        expected_names.len(),
        expected_agent_v4_names.len(),
        expected_application_model_names.len()
    );
    Ok(())
}

fn write_schemas() -> Result<(), String> {
    verify_public_schemas()?;
    verify_agent_v4_schemas()?;
    verify_application_model_schemas()?;
    verify_workflow_pack_schema()?;
    let directory = schema_directory();
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let schemas = generate_public_schemas();
    let expected_names = schemas
        .iter()
        .map(|schema| schema.id.file_name())
        .collect::<BTreeSet<_>>();
    for existing in json_files(&directory)? {
        if !expected_names.contains(&existing) {
            fs::remove_file(directory.join(existing)).map_err(|error| error.to_string())?;
        }
    }
    for schema in schemas {
        let path = directory.join(schema.id.file_name());
        fs::write(&path, schema.canonical_json()).map_err(|error| error.to_string())?;
    }
    let agent_v4_directory = agent_v4_schema_directory();
    fs::create_dir_all(&agent_v4_directory).map_err(|error| error.to_string())?;
    let agent_v4 = generate_agent_v4_schemas();
    let expected_agent_v4_names = agent_v4
        .iter()
        .map(|schema| schema.id.file_name())
        .collect::<BTreeSet<_>>();
    for existing in json_files(&agent_v4_directory)? {
        if !expected_agent_v4_names.contains(&existing) {
            fs::remove_file(agent_v4_directory.join(existing))
                .map_err(|error| error.to_string())?;
        }
    }
    for schema in agent_v4 {
        let path = agent_v4_directory.join(schema.id.file_name());
        fs::write(&path, schema.canonical_json()).map_err(|error| error.to_string())?;
    }
    let application_model_directory = application_model_schema_directory();
    fs::create_dir_all(&application_model_directory).map_err(|error| error.to_string())?;
    let application_model = generate_application_model_schemas();
    let expected_application_model_names = application_model
        .iter()
        .map(|schema| schema.id.file_name())
        .collect::<BTreeSet<_>>();
    for existing in json_files(&application_model_directory)? {
        if !expected_application_model_names.contains(&existing) {
            fs::remove_file(application_model_directory.join(existing))
                .map_err(|error| error.to_string())?;
        }
    }
    for schema in application_model {
        let path = application_model_directory.join(schema.id.file_name());
        fs::write(&path, schema.canonical_json()).map_err(|error| error.to_string())?;
    }
    let workflow_pack = generate_workflow_pack_schema();
    let workflow_pack_directory = workflow_pack_schema_directory();
    fs::create_dir_all(&workflow_pack_directory).map_err(|error| error.to_string())?;
    for existing in json_files(&workflow_pack_directory)? {
        if existing != workflow_pack.file_name() {
            fs::remove_file(workflow_pack_directory.join(existing))
                .map_err(|error| error.to_string())?;
        }
    }
    fs::write(
        workflow_pack_directory.join(workflow_pack.file_name()),
        workflow_pack.canonical_json(),
    )
    .map_err(|error| error.to_string())?;
    println!(
        "schemas: wrote {} public + {} Agent v4 + {} application-v3 + 1 workflow-pack",
        expected_names.len(),
        expected_agent_v4_names.len(),
        expected_application_model_names.len()
    );
    Ok(())
}

fn schema_directory() -> PathBuf {
    repository_root().join("crates/canisend-resources/resources/schemas/v2")
}

fn application_model_schema_directory() -> PathBuf {
    repository_root().join("crates/canisend-resources/resources/schemas/v3")
}

fn agent_v4_schema_directory() -> PathBuf {
    repository_root().join("crates/canisend-resources/resources/schemas/agent/v4")
}

fn workflow_pack_schema_directory() -> PathBuf {
    repository_root().join("crates/canisend-resources/resources/schemas/workflow-pack/v1")
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is inside repository root")
        .to_path_buf()
}

const DESKTOP_HOST_BUDGET_BYTES: u64 = 67_108_864;
const DESKTOP_PAYLOAD_BUDGET_BYTES: u64 = 75_497_472;
const DESKTOP_FRONTEND_BUDGET_BYTES: u64 = 1_572_864;
const DESKTOP_RUNTIME_PAYLOAD_BUDGET_BYTES: u64 = 402_653_184;
const DESKTOP_APPIMAGE_BUDGET_BYTES: u64 = 134_217_728;
const DESKTOP_OFFLINE_INSTALLER_BUDGET_BYTES: u64 = 268_435_456;
const LARGE_NATIVE_FILE_BYTES: u64 = 10 * 1024 * 1024;
const DESKTOP_PROFILE_MINIMUM_SAVING_BYTES: u64 = 1024 * 1024;
const DESKTOP_PROFILE_MINIMUM_SAVING_PERCENT: u64 = 2;

fn desktop_profile_configuration(candidate: &str) -> Option<(&'static str, &'static str)> {
    match candidate {
        "release" => Some(("3", "thin")),
        "size-s-thin" => Some(("s", "thin")),
        "size-z-thin" => Some(("z", "thin")),
        "size-z-fat" => Some(("z", "fat")),
        _ => None,
    }
}

fn command_stdout(program: &str, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not execute `{program}`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "`{program} {}` failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| format!("`{program}` output is not UTF-8"))
}

fn desktop_source_identity() -> Result<Value, String> {
    let root = repository_root();
    let commit_lines = run_git_lines(&root, &["rev-parse", "HEAD"])?;
    let commit = commit_lines
        .first()
        .filter(|value| value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| "Git HEAD is not a full commit digest".to_owned())?;
    let dirty =
        !run_git_lines(&root, &["status", "--porcelain", "--untracked-files=all"])?.is_empty();
    let cargo_lock = root.join("Cargo.lock");
    let pnpm_lock = root.join("apps/canisend-desktop/pnpm-lock.yaml");
    let template_contract = typst_template_contract_path();
    Ok(json!({
        "commit": commit,
        "dirty": dirty,
        "cargo_lock_sha256": sha256_file(&cargo_lock)?,
        "pnpm_lock_sha256": sha256_file(&pnpm_lock)?,
        "template_contract_sha256": sha256_file(&template_contract)?
    }))
}

fn write_desktop_profile_record(
    target: &str,
    candidate: &str,
    opt_level: &str,
    lto: &str,
    host: &Path,
    output: &Path,
) -> Result<(), String> {
    check_typst_template_contract()?;
    if !release_targets()?
        .iter()
        .any(|release_target| release_target.triple == target)
    {
        return Err(format!(
            "desktop profile record has unsupported target `{target}`"
        ));
    }
    let expected = desktop_profile_configuration(candidate)
        .ok_or_else(|| format!("unsupported desktop profile candidate `{candidate}`"))?;
    if (opt_level, lto) != expected {
        return Err(format!(
            "desktop profile candidate `{candidate}` requires opt-level `{}` and LTO `{}`",
            expected.0, expected.1
        ));
    }
    if output.exists() || output.is_symlink() {
        return Err(format!(
            "desktop profile record output must not exist: {}",
            output.display()
        ));
    }
    reject_symlink(host)?;
    let host = host
        .canonicalize()
        .map_err(|error| format!("could not resolve desktop profile host: {error}"))?;
    let host_file_name = host
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "desktop profile host file name is not UTF-8".to_owned())?;
    let logical_host_path = format!("profile-matrix/{candidate}/{target}/release/{host_file_name}");
    let host_bytes = file_size(&host)?;
    let recorded_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| format!("could not format desktop profile timestamp: {error}"))?;
    let contract = expected_typst_template_contract()?;
    let record = json!({
        "schema": DESKTOP_PROFILE_RECORD_SCHEMA,
        "authoritative_release_evidence": false,
        "target": target,
        "candidate": candidate,
        "recorded_at": recorded_at,
        "source": desktop_source_identity()?,
        "toolchain": {
            "cargo": command_stdout("cargo", &["--version"] )?,
            "rustc_verbose": command_stdout("rustc", &["--version", "--verbose"] )?
        },
        "build": {
            "profile": "release",
            "opt_level": opt_level,
            "lto": lto,
            "codegen_units": 1,
            "panic": "abort",
            "strip": "symbols"
        },
        "templates": contract["templates"].clone(),
        "host": {
            "path": logical_host_path,
            "bytes": host_bytes,
            "sha256": sha256_file(&host)?,
            "budget_bytes": DESKTOP_HOST_BUDGET_BYTES,
            "passed_budget": host_bytes <= DESKTOP_HOST_BUDGET_BYTES
        }
    });
    write_pretty_json(output, &record)?;
    let budget_status = if host_bytes <= DESKTOP_HOST_BUDGET_BYTES {
        "within budget"
    } else {
        "over budget"
    };
    println!("desktop profile: {target} {candidate} = {host_bytes} bytes ({budget_status})");
    Ok(())
}

fn read_desktop_profile_record(path: &Path) -> Result<Value, String> {
    let body = fs::read_to_string(path)
        .map_err(|error| format!("could not read profile record {}: {error}", path.display()))?;
    let record: Value = serde_json::from_str(&body)
        .map_err(|error| format!("profile record {} is invalid JSON: {error}", path.display()))?;
    if record["schema"] != DESKTOP_PROFILE_RECORD_SCHEMA {
        return Err(format!(
            "profile record {} has an unsupported schema",
            path.display()
        ));
    }
    Ok(record)
}

fn write_desktop_profile_summary(records: [&Path; 4], output: &Path) -> Result<(), String> {
    if output.exists() || output.is_symlink() {
        return Err(format!(
            "desktop profile summary output must not exist: {}",
            output.display()
        ));
    }
    let mut by_candidate = BTreeMap::new();
    for path in records {
        let record = read_desktop_profile_record(path)?;
        let candidate = record["candidate"]
            .as_str()
            .ok_or_else(|| format!("profile record {} has no candidate", path.display()))?
            .to_owned();
        if by_candidate.insert(candidate.clone(), record).is_some() {
            return Err(format!("duplicate desktop profile candidate `{candidate}`"));
        }
    }
    let expected_candidates = ["release", "size-s-thin", "size-z-thin", "size-z-fat"];
    if by_candidate
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected_candidates.into_iter().collect()
    {
        return Err("desktop profile summary requires the exact four candidate records".to_owned());
    }
    let reference = &by_candidate["release"];
    let target = reference["target"]
        .as_str()
        .ok_or_else(|| "release profile record has no target".to_owned())?;
    let source = &reference["source"];
    let reference_bytes = reference["host"]["bytes"]
        .as_u64()
        .ok_or_else(|| "release profile record has no host byte count".to_owned())?;
    let material_threshold = DESKTOP_PROFILE_MINIMUM_SAVING_BYTES
        .max(reference_bytes.saturating_mul(DESKTOP_PROFILE_MINIMUM_SAVING_PERCENT) / 100);
    let mut results = Vec::with_capacity(expected_candidates.len());
    let mut winner: Option<(&str, u64)> = None;
    for candidate in expected_candidates {
        let record = &by_candidate[candidate];
        if record["target"] != target || record["source"] != *source {
            return Err(format!(
                "desktop profile candidate `{candidate}` was not built from the same target and source identity"
            ));
        }
        let bytes = record["host"]["bytes"].as_u64().ok_or_else(|| {
            format!("desktop profile candidate `{candidate}` has no host byte count")
        })?;
        let passed_budget = record["host"]["passed_budget"].as_bool() == Some(true);
        let saving = reference_bytes.saturating_sub(bytes);
        let saving_basis_points = saving
            .saturating_mul(10_000)
            .checked_div(reference_bytes)
            .unwrap_or(0);
        if passed_budget && winner.is_none_or(|(_, winner_bytes)| bytes < winner_bytes) {
            winner = Some((candidate, bytes));
        }
        results.push(json!({
            "candidate": candidate,
            "bytes": bytes,
            "saving_bytes_vs_release": saving,
            "saving_basis_points_vs_release": saving_basis_points,
            "passed_budget": passed_budget
        }));
    }
    let (winner_candidate, winner_bytes) = winner.ok_or_else(|| {
        "desktop profile matrix has no candidate within the host budget".to_owned()
    })?;
    let winner_saving = reference_bytes.saturating_sub(winner_bytes);
    let material_size_candidate =
        winner_candidate != "release" && winner_saving >= material_threshold;
    let summary = json!({
        "schema": DESKTOP_PROFILE_SUMMARY_SCHEMA,
        "authoritative_release_evidence": false,
        "target": target,
        "source": source,
        "reference_bytes": reference_bytes,
        "materiality": {
            "minimum_saving_bytes": material_threshold,
            "minimum_percent": DESKTOP_PROFILE_MINIMUM_SAVING_PERCENT
        },
        "winner": {
            "candidate": winner_candidate,
            "bytes": winner_bytes,
            "saving_bytes_vs_release": winner_saving,
            "material_size_candidate": material_size_candidate,
            "requires_native_functional_qualification": winner_candidate != "release"
        },
        "candidates": results
    });
    write_pretty_json(output, &summary)?;
    println!(
        "desktop profile summary: {target} winner {winner_candidate} at {winner_bytes} bytes (saving {winner_saving})"
    );
    Ok(())
}

fn desktop_package_class(package_format: &str) -> (&'static str, bool) {
    match package_format {
        "appimage" => ("portable", true),
        "nsis-offline" => ("offline", true),
        _ => ("standard", false),
    }
}

#[allow(clippy::too_many_arguments)]
fn write_desktop_size_record(
    target: &str,
    profile: &str,
    package_format: &str,
    host: &Path,
    payload: &Path,
    frontend: Option<&Path>,
    artifact: Option<&Path>,
    output: &Path,
) -> Result<(), String> {
    check_typst_template_contract()?;
    if !release_targets()?
        .iter()
        .any(|candidate| candidate.triple == target)
    {
        return Err(format!(
            "desktop size record has unsupported target `{target}`"
        ));
    }
    if !matches!(profile, "release-alpha" | "release") {
        return Err("desktop size record profile must be release-alpha or release".to_owned());
    }
    if !matches!(
        package_format,
        "app" | "dmg" | "zip" | "nsis" | "nsis-offline" | "msi" | "deb" | "rpm" | "appimage"
    ) {
        return Err(format!(
            "desktop size record has unsupported package format `{package_format}`"
        ));
    }
    if output.exists() || output.is_symlink() {
        return Err(format!(
            "desktop size record output must not exist: {}",
            output.display()
        ));
    }
    reject_symlink(host)?;
    let payload = canonical_regular_directory(payload, "desktop application payload")?;
    let host = host
        .canonicalize()
        .map_err(|error| format!("could not resolve unified host: {error}"))?;
    if !host.starts_with(&payload) {
        return Err("desktop unified host must be inside the measured payload".to_owned());
    }

    let (package_class, runtime_inclusive) = desktop_package_class(package_format);
    let (payload_bytes, native_hosts) = if package_format == "appimage" {
        inspect_portable_desktop_payload(&payload)?
    } else {
        inspect_desktop_payload(&payload)?
    };
    let host_bytes = file_size(&host)?;
    let host_relative = relative_slash_path(&payload, &host)?;
    if native_hosts.len() != 1 || native_hosts[0].0 != host_relative {
        return Err(format!(
            "desktop payload must contain exactly the declared unified host; found {native_hosts:?}"
        ));
    }

    let frontend_bytes = frontend
        .map(|path| {
            let path = canonical_regular_directory(path, "desktop frontend")?;
            inspect_desktop_payload(&path).map(|(bytes, _)| bytes)
        })
        .transpose()?;
    if let Some(artifact) = artifact {
        reject_symlink(artifact)?;
    }
    let artifact_bytes = artifact.map(file_size).transpose()?;
    let artifact_sha256 = artifact.map(sha256_file).transpose()?;
    let payload_budget = if package_format == "appimage" {
        DESKTOP_RUNTIME_PAYLOAD_BUDGET_BYTES
    } else {
        DESKTOP_PAYLOAD_BUDGET_BYTES
    };
    let artifact_budget = match package_format {
        "appimage" => Some(DESKTOP_APPIMAGE_BUDGET_BYTES),
        "nsis-offline" => Some(DESKTOP_OFFLINE_INSTALLER_BUDGET_BYTES),
        _ => None,
    };
    let passed = host_bytes <= DESKTOP_HOST_BUDGET_BYTES
        && payload_bytes <= payload_budget
        && frontend_bytes.is_none_or(|bytes| bytes <= DESKTOP_FRONTEND_BUDGET_BYTES)
        && artifact_budget.is_none_or(|budget| artifact_bytes.is_some_and(|bytes| bytes <= budget));
    let recorded_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| format!("could not format desktop size timestamp: {error}"))?;
    let rust_opt_level = std::env::var("CARGO_PROFILE_RELEASE_OPT_LEVEL")
        .unwrap_or_else(|_| "profile-default".to_owned());
    let rust_lto =
        std::env::var("CARGO_PROFILE_RELEASE_LTO").unwrap_or_else(|_| "profile-default".to_owned());
    let record = json!({
        "schema": "canisend.desktop-size/v1",
        "target": target,
        "profile": profile,
        "source": desktop_source_identity()?,
        "build_optimization": {
            "rust_opt_level": rust_opt_level,
            "rust_lto": rust_lto,
            "codegen_units": 1,
            "panic": "abort",
            "strip": "symbols"
        },
        "templates": expected_typst_template_contract()?["templates"].clone(),
        "package_format": package_format,
        "package_class": package_class,
        "runtime_inclusive": runtime_inclusive,
        "recorded_at": recorded_at,
        "budgets": {
            "unified_host_bytes": DESKTOP_HOST_BUDGET_BYTES,
            "application_payload_bytes": payload_budget,
            "frontend_bytes": DESKTOP_FRONTEND_BUDGET_BYTES,
            "download_artifact_bytes": artifact_budget,
            "full_native_host_count": 1
        },
        "bytes": {
            "unified_host": host_bytes,
            "application_payload": payload_bytes,
            "frontend": frontend_bytes,
            "download_artifact": artifact_bytes
        },
        "sha256": {
            "unified_host": sha256_file(&host)?,
            "download_artifact": artifact_sha256
        },
        "native_hosts": native_hosts
            .iter()
            .map(|(path, bytes)| json!({"path": path, "bytes": bytes}))
            .collect::<Vec<_>>(),
        "passed": passed
    });
    write_pretty_json(output, &record)?;
    if !passed {
        return Err(format!(
            "desktop size budget exceeded; inspect {}",
            output.display()
        ));
    }
    println!(
        "desktop size: {target} {package_format} has one {host_bytes}-byte host in a {payload_bytes}-byte payload"
    );
    Ok(())
}

fn canonical_regular_directory(path: &Path, context: &str) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("could not inspect {context} {}: {error}", path.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(format!(
            "{context} is not a regular directory: {}",
            path.display()
        ));
    }
    path.canonicalize()
        .map_err(|error| format!("could not resolve {context} {}: {error}", path.display()))
}

fn inspect_desktop_payload(root: &Path) -> Result<(u64, Vec<(String, u64)>), String> {
    let mut bytes = 0_u64;
    let mut native_hosts = Vec::new();
    inspect_desktop_payload_directory(root, root, &mut bytes, &mut native_hosts)?;
    native_hosts.sort();
    Ok((bytes, native_hosts))
}

fn inspect_portable_desktop_payload(root: &Path) -> Result<(u64, Vec<(String, u64)>), String> {
    let mut bytes = 0_u64;
    let mut native_hosts = Vec::new();
    inspect_portable_desktop_payload_directory(root, root, &mut bytes, &mut native_hosts)?;
    native_hosts.sort();
    Ok((bytes, native_hosts))
}

fn inspect_portable_desktop_payload_directory(
    root: &Path,
    directory: &Path,
    bytes: &mut u64,
    native_hosts: &mut Vec<(String, u64)>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("could not inspect {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("could not inspect payload entry: {error}"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            let target = path.canonicalize().map_err(|error| {
                format!(
                    "portable payload symlink does not resolve {}: {error}",
                    path.display()
                )
            })?;
            if !target.starts_with(root) {
                return Err(format!(
                    "portable payload symlink escapes the extracted root: {}",
                    path.display()
                ));
            }
            *bytes = bytes
                .checked_add(metadata.len())
                .ok_or_else(|| "desktop payload byte count overflowed".to_owned())?;
        } else if metadata.is_dir() {
            inspect_portable_desktop_payload_directory(root, &path, bytes, native_hosts)?;
        } else if metadata.is_file() {
            *bytes = bytes
                .checked_add(metadata.len())
                .ok_or_else(|| "desktop payload byte count overflowed".to_owned())?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if matches!(
                name.as_ref(),
                "canisend" | "canisend-gui" | "canisend.exe" | "canisend-gui.exe"
            ) && has_native_executable_magic(&path)?
            {
                native_hosts.push((relative_slash_path(root, &path)?, metadata.len()));
            }
        } else {
            return Err(format!(
                "portable desktop payload contains a non-regular entry: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn inspect_desktop_payload_directory(
    root: &Path,
    directory: &Path,
    bytes: &mut u64,
    native_hosts: &mut Vec<(String, u64)>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("could not inspect {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("could not inspect payload entry: {error}"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "desktop application payload contains a symlink: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            inspect_desktop_payload_directory(root, &path, bytes, native_hosts)?;
        } else if metadata.is_file() {
            *bytes = bytes
                .checked_add(metadata.len())
                .ok_or_else(|| "desktop payload byte count overflowed".to_owned())?;
            if metadata.len() >= LARGE_NATIVE_FILE_BYTES && has_native_executable_magic(&path)? {
                native_hosts.push((relative_slash_path(root, &path)?, metadata.len()));
            }
        } else {
            return Err(format!(
                "desktop application payload contains a non-regular entry: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn relative_slash_path(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .map_err(|error| format!("could not relativize desktop payload path: {error}"))
}

fn has_native_executable_magic(path: &Path) -> Result<bool, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("could not inspect native file {}: {error}", path.display()))?;
    let mut magic = [0_u8; 4];
    let read = file
        .read(&mut magic)
        .map_err(|error| format!("could not inspect native file {}: {error}", path.display()))?;
    if read < 2 {
        return Ok(false);
    }
    Ok(magic.starts_with(b"MZ")
        || magic == [0x7f, b'E', b'L', b'F']
        || matches!(
            magic,
            [0xfe, 0xed, 0xfa, 0xce]
                | [0xce, 0xfa, 0xed, 0xfe]
                | [0xfe, 0xed, 0xfa, 0xcf]
                | [0xcf, 0xfa, 0xed, 0xfe]
                | [0xca, 0xfe, 0xba, 0xbe]
                | [0xbe, 0xba, 0xfe, 0xca]
                | [0xca, 0xfe, 0xba, 0xbf]
                | [0xbf, 0xba, 0xfe, 0xca]
        ))
}

fn json_files(directory: &Path) -> Result<BTreeSet<String>, String> {
    if !directory.exists() {
        return Ok(BTreeSet::new());
    }
    fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(|entry| match entry {
            Ok(entry)
                if entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "json") =>
            {
                Some(Ok(entry.file_name().to_string_lossy().into_owned()))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error.to_string())),
        })
        .collect()
}

fn check_resources() -> Result<(), String> {
    canisend_resources::verify()?;
    if canisend_resources::manifest().is_empty() {
        return Err("embedded resource manifest is empty".to_owned());
    }
    check_typst_template_contract()?;
    println!("resources: ok");
    Ok(())
}

fn typst_template_contract_path() -> PathBuf {
    repository_root().join("release/typst-template-contract.json")
}

fn locked_package_version(package_name: &str) -> Result<String, String> {
    let lock_path = repository_root().join("Cargo.lock");
    let body = fs::read_to_string(&lock_path)
        .map_err(|error| format!("could not read {}: {error}", lock_path.display()))?;
    let lock: toml::Value =
        toml::from_str(&body).map_err(|error| format!("Cargo.lock is invalid TOML: {error}"))?;
    let packages = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| "Cargo.lock does not contain packages".to_owned())?;
    let versions = packages
        .iter()
        .filter(|package| package.get("name").and_then(toml::Value::as_str) == Some(package_name))
        .filter_map(|package| package.get("version").and_then(toml::Value::as_str))
        .collect::<BTreeSet<_>>();
    if versions.len() != 1 {
        return Err(format!(
            "Cargo.lock must contain exactly one `{package_name}` version, found {versions:?}"
        ));
    }
    Ok(versions
        .into_iter()
        .next()
        .expect("one locked version")
        .to_owned())
}

fn typst_template_descriptor_value(
    descriptor: canisend_resources::ResourceDescriptor,
) -> Result<Value, String> {
    let (bundle_id, document_kinds, fallback_for, adapter_revision, upstream, font_contract) =
        match descriptor.id {
            "template.application-document" => (
                "canisend-default",
                json!([]),
                json!(["cv", "research-statement", "teaching-statement"]),
                0,
                Value::Null,
                json!({
                    "families": ["Libertinus Serif"],
                    "inherits_renderer_default": false,
                    "styles": ["normal"],
                    "weights": ["regular", "semibold"]
                }),
            ),
            "template.cover-letter" => (
                "canisend-default",
                json!([]),
                json!(["cover-letter"]),
                0,
                Value::Null,
                json!({
                    "families": ["Libertinus Serif"],
                    "inherits_renderer_default": false,
                    "styles": ["normal"],
                    "weights": ["regular", "semibold"]
                }),
            ),
            "template.modernpro-cv" => (
                "modernpro-cv",
                json!(["cv"]),
                json!([]),
                2,
                json!({
                    "archive_sha256": "1d108f538571e804f96b59dc1f3c0b0e0dc275b3eb35c6368fd7cc89775851f0",
                    "archive_url": "https://packages.typst.org/preview/modernpro-cv-2.0.0.tar.gz",
                    "license": "MIT",
                    "package": "modernpro-cv",
                    "repository": "https://github.com/jxpeng98/Typst-CV-Resume",
                    "source_entrypoint": "modernpro-cv.typ",
                    "source_patches": [{
                        "id": "prefer-explicit-configuration",
                        "reason": "Honor the configured embedded font before the unavailable upstream fallback"
                    }],
                    "version": "2.0.0"
                }),
                json!({
                    "families": ["Libertinus Serif"],
                    "inherits_renderer_default": false,
                    "styles": ["normal", "italic"],
                    "upstream_default_families": ["PT Serif", "Libertinus Serif"],
                    "weights": ["light", "regular", "medium", "semibold", "bold"]
                }),
            ),
            "template.modernpro-coverletter" => (
                "modernpro-coverletter",
                json!(["cover-letter", "research-statement", "teaching-statement"]),
                json!([]),
                2,
                json!({
                    "archive_sha256": "d3c5e8031e8a74ab4ae6e3163b0f37d6ecebc972dd7a4b3b41fc99ff07585130",
                    "archive_url": "https://packages.typst.org/preview/modernpro-coverletter-1.0.0.tar.gz",
                    "license": "MIT",
                    "package": "modernpro-coverletter",
                    "repository": "https://github.com/jxpeng98/typst-coverletter",
                    "source_entrypoint": "modernpro-coverletter.typ",
                    "source_patches": [{
                        "id": "prefer-explicit-configuration",
                        "reason": "Honor the configured embedded font before the unavailable upstream fallback"
                    }],
                    "version": "1.0.0"
                }),
                json!({
                    "families": ["Libertinus Serif"],
                    "inherits_renderer_default": false,
                    "styles": ["normal"],
                    "upstream_default_families": ["PT Serif", "Libertinus Serif"],
                    "weights": ["light", "regular", "bold"]
                }),
            ),
            other => return Err(format!("uncontracted embedded Typst template `{other}`")),
        };
    Ok(json!({
        "adapter_revision": adapter_revision,
        "bundle_id": bundle_id,
        "document_kinds": document_kinds,
        "entrypoint": "canisend_render_document",
        "fallback_for": fallback_for,
        "resource_id": descriptor.id,
        "path": descriptor.path,
        "resource_version": descriptor.version,
        "bytes": descriptor.size,
        "sha256": descriptor.sha256,
        "font_contract": font_contract,
        "upstream": upstream
    }))
}

fn expected_typst_template_contract() -> Result<Value, String> {
    let mut templates = canisend_resources::manifest()
        .into_iter()
        .filter(|descriptor| descriptor.kind == canisend_resources::ResourceKind::Template)
        .map(typst_template_descriptor_value)
        .collect::<Result<Vec<_>, _>>()?;
    templates.sort_by(|left, right| {
        left["resource_id"]
            .as_str()
            .cmp(&right["resource_id"].as_str())
    });
    Ok(json!({
        "schema": TYPST_TEMPLATE_CONTRACT_SCHEMA,
        "contract_version": 2,
        "baseline": "modernpro-universe-pinned-v2",
        "renderer": {
            "typst_as_lib": locked_package_version("typst-as-lib")?,
            "typst_assets": locked_package_version("typst-assets")?,
            "typst_pdf": locked_package_version("typst-pdf")?,
            "font_pack": "typst-assets-full",
            "font_source": "typst_assets::fonts()",
            "system_font_discovery": false,
            "external_file_access": false,
            "external_package_resolution": false,
            "network_access": false
        },
        "coverage": {
            "scripts": ["latin", "greek", "cyrillic"],
            "math": true,
            "symbols": true,
            "urls": true,
            "lists": true,
            "tables": true,
            "explicit_page_breaks": true,
            "media": []
        },
        "fixtures": [
            "cover-letter-template",
            "all-document-kinds",
            "all-document-section-kinds",
            "unicode-math-url-list-table-two-page",
            "missing-font-bounded-fallback",
            "restricted-world-file-and-package-rejection"
        ],
        "warning_policy": "zero-for-product-template-fixtures",
        "templates": templates
    }))
}

fn check_typst_template_contract() -> Result<(), String> {
    canisend_resources::verify()?;
    let path = typst_template_contract_path();
    let body = fs::read_to_string(&path).map_err(|error| {
        format!(
            "Typst template contract is missing at {}: {error}",
            path.display()
        )
    })?;
    let actual: Value = serde_json::from_str(&body)
        .map_err(|error| format!("Typst template contract is invalid JSON: {error}"))?;
    let expected = expected_typst_template_contract()?;
    if actual != expected {
        return Err(format!(
            "Typst template contract differs from embedded resources, locked renderer versions, or the committed capability policy: {}",
            path.display()
        ));
    }
    println!(
        "desktop templates: ok ({} template resources, contract {})",
        actual["templates"].as_array().map_or(0, Vec::len),
        sha256(body.as_bytes())
    );
    Ok(())
}

const DOMAIN_COUPLING_SCAN_ROOTS: &[&str] = &[
    "README.md",
    "crates",
    "xtask/src",
    "apps/canisend-desktop/src",
    "apps/canisend-desktop/tests",
    "docs/contracts",
    "docs/guides",
    "fixtures",
];
const DOMAIN_COUPLING_EXCLUDED_PATHS: &[&str] =
    &["docs/contracts/domain-coupling-inventory-v1.json"];
const DOMAIN_COUPLING_REQUIRED_AREAS: &[&str] = &[
    "rust",
    "sql",
    "schemas",
    "resources",
    "ui",
    "guides",
    "tests",
    "projections",
];
const DOMAIN_COUPLING_CLASSIFICATIONS: &[&str] = &[
    "kernel",
    "academic-pack",
    "optional-adapter",
    "compatibility-surface",
    "removal",
];
const DOMAIN_COUPLING_FAMILIES: &[(&str, &[&str])] = &[
    (
        "legacy-job-surface",
        &[
            "job_id",
            "JobId",
            "jobs/JOB_ID",
            "canisend_job",
            "canisend-job",
            "parsed-job",
            "job-parse",
            "job advert",
            "Job advert",
        ],
    ),
    (
        "fixed-academic-deliverables",
        &[
            "CoverLetter",
            "CurriculumVitae",
            "ResearchStatement",
            "TeachingStatement",
            "cover-letter",
            "curriculum-vitae",
            "research-statement",
            "teaching-statement",
        ],
    ),
    ("fixed-workflow-stage", &["WorkflowStage"]),
    (
        "academic-vocabulary",
        &["academic", "Academic", "faculty", "Faculty", "jobs.ac.uk"],
    ),
];

fn print_domain_coupling_inventory() -> Result<(), String> {
    let inventory = build_domain_coupling_inventory(&repository_root())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&inventory)
            .map_err(|error| format!("could not serialize domain-coupling inventory: {error}"))?
    );
    Ok(())
}

fn check_domain_coupling_inventory() -> Result<(), String> {
    let root = repository_root();
    let inventory = build_domain_coupling_inventory(&root)?;
    let contract_path = root.join("docs/contracts/domain-coupling-inventory-v1.json");
    let contract: Value = serde_json::from_slice(&fs::read(&contract_path).map_err(|error| {
        format!(
            "domain-coupling inventory contract is missing at {}: {error}",
            contract_path.display()
        )
    })?)
    .map_err(|error| format!("domain-coupling inventory contract is invalid JSON: {error}"))?;
    validate_domain_coupling_contract(&contract, &inventory)?;
    let summary = &inventory["summary"];
    println!(
        "domain coupling: ok ({} files, {} classifications, {} required areas)",
        summary["matching_files"].as_u64().unwrap_or_default(),
        summary["classification_counts"]
            .as_object()
            .map_or(0, Map::len),
        DOMAIN_COUPLING_REQUIRED_AREAS.len()
    );
    Ok(())
}

fn validate_domain_coupling_contract(contract: &Value, inventory: &Value) -> Result<(), String> {
    if contract["schema"] != DOMAIN_COUPLING_INVENTORY_SCHEMA {
        return Err(format!(
            "domain-coupling inventory contract must use schema `{DOMAIN_COUPLING_INVENTORY_SCHEMA}`"
        ));
    }
    for field in [
        "scan_roots",
        "excluded_paths",
        "pattern_families",
        "required_areas",
        "allowed_classifications",
    ] {
        if contract[field] != inventory[field] {
            return Err(format!(
                "domain-coupling inventory contract field `{field}` drifted from the scanner"
            ));
        }
    }
    if contract["expected"] != inventory["summary"] {
        return Err(
            "domain-coupling inventory changed; inspect `cargo run -p xtask --locked -- scope inventory --json`, classify the change, and update the checked contract"
                .to_owned(),
        );
    }
    Ok(())
}

fn build_domain_coupling_inventory(root: &Path) -> Result<Value, String> {
    let mut files = BTreeSet::new();
    for relative in DOMAIN_COUPLING_SCAN_ROOTS {
        let path = root.join(relative);
        if !path.exists() {
            return Err(format!(
                "domain-coupling scan root is missing: {}",
                path.display()
            ));
        }
        collect_domain_coupling_files(root, &path, &mut files)?;
    }

    let mut entries = Vec::new();
    let mut area_counts = BTreeMap::<String, u64>::new();
    let mut classification_counts = BTreeMap::<String, u64>::new();
    let mut family_counts = BTreeMap::<String, u64>::new();
    for relative in files {
        if DOMAIN_COUPLING_EXCLUDED_PATHS.contains(&relative.as_str()) {
            continue;
        }
        let body = fs::read_to_string(root.join(&relative)).map_err(|error| {
            format!("could not read domain-coupling input `{relative}`: {error}")
        })?;
        let families = domain_coupling_families(&body);
        if families.is_empty() {
            continue;
        }
        let areas = domain_coupling_areas(&relative, &body);
        if areas.is_empty() {
            return Err(format!(
                "domain-coupling finding `{relative}` has no classified repository area"
            ));
        }
        let classification = classify_domain_coupling(&relative, &families)?;
        for area in &areas {
            *area_counts.entry(area.clone()).or_default() += 1;
        }
        *classification_counts
            .entry(classification.to_owned())
            .or_default() += 1;
        for family in &families {
            *family_counts.entry(family.clone()).or_default() += 1;
        }
        entries.push(json!({
            "path": relative,
            "areas": areas,
            "families": families,
            "classification": classification,
        }));
    }

    for area in DOMAIN_COUPLING_REQUIRED_AREAS {
        if !area_counts.contains_key(*area) {
            return Err(format!(
                "domain-coupling inventory does not cover required area `{area}`"
            ));
        }
    }
    for classification in classification_counts.keys() {
        if !DOMAIN_COUPLING_CLASSIFICATIONS.contains(&classification.as_str()) {
            return Err(format!(
                "domain-coupling inventory produced unsupported classification `{classification}`"
            ));
        }
    }
    let entries_bytes = serde_json::to_vec(&entries)
        .map_err(|error| format!("could not hash domain-coupling inventory: {error}"))?;
    let pattern_families = DOMAIN_COUPLING_FAMILIES
        .iter()
        .map(|(id, needles)| json!({"id": id, "needles": needles}))
        .collect::<Vec<_>>();
    let summary = json!({
        "matching_files": entries.len(),
        "inventory_sha256": sha256(&entries_bytes),
        "area_counts": area_counts,
        "classification_counts": classification_counts,
        "family_counts": family_counts,
    });
    Ok(json!({
        "schema": DOMAIN_COUPLING_INVENTORY_SCHEMA,
        "scan_roots": DOMAIN_COUPLING_SCAN_ROOTS,
        "excluded_paths": DOMAIN_COUPLING_EXCLUDED_PATHS,
        "pattern_families": pattern_families,
        "required_areas": DOMAIN_COUPLING_REQUIRED_AREAS,
        "allowed_classifications": DOMAIN_COUPLING_CLASSIFICATIONS,
        "entries": entries,
        "summary": summary,
    }))
}

fn collect_domain_coupling_files(
    root: &Path,
    path: &Path,
    files: &mut BTreeSet<String>,
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "domain-coupling scan root contains a symlink: {}",
            path.display()
        ));
    }
    if metadata.is_file() {
        if domain_coupling_text_file(path) {
            files.insert(relative_slash_path(root, path)?);
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(format!(
            "domain-coupling scan input is not a regular file or directory: {}",
            path.display()
        ));
    }
    let mut entries = fs::read_dir(path)
        .map_err(|error| format!("could not inspect {}: {error}", path.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        collect_domain_coupling_files(root, &entry.path(), files)?;
    }
    Ok(())
}

fn domain_coupling_text_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension,
                "rs" | "sql"
                    | "json"
                    | "md"
                    | "toml"
                    | "ts"
                    | "svelte"
                    | "sh"
                    | "csv"
                    | "xml"
                    | "typ"
                    | "yaml"
                    | "yml"
            )
        })
}

fn domain_coupling_families(body: &str) -> BTreeSet<String> {
    DOMAIN_COUPLING_FAMILIES
        .iter()
        .filter(|(_, needles)| needles.iter().any(|needle| body.contains(needle)))
        .map(|(id, _)| (*id).to_owned())
        .collect()
}

fn domain_coupling_areas(path: &str, body: &str) -> BTreeSet<String> {
    let mut areas = BTreeSet::new();
    let extension = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if extension == "rs" {
        areas.insert("rust".to_owned());
    }
    if extension == "sql" {
        areas.insert("sql".to_owned());
    }
    if path.contains("/schemas/") && extension == "json" {
        areas.insert("schemas".to_owned());
    }
    if path.contains("/resources/") {
        areas.insert("resources".to_owned());
    }
    if path.starts_with("apps/canisend-desktop/") || path.starts_with("crates/canisend-desktop/") {
        areas.insert("ui".to_owned());
    }
    if path == "README.md" || path.starts_with("docs/guides/") {
        areas.insert("guides".to_owned());
    }
    if path.starts_with("docs/contracts/") || path.starts_with("crates/canisend-contracts/") {
        areas.insert("contracts".to_owned());
    }
    if path.contains("/tests/")
        || path.starts_with("fixtures/")
        || path.contains(".test.")
        || body.contains("#[cfg(test)]")
    {
        areas.insert("tests".to_owned());
    }
    if path.to_ascii_lowercase().contains("projection")
        || body.contains("jobs/JOB_ID")
        || body.contains("applications/APPLICATION_ID")
    {
        areas.insert("projections".to_owned());
    }
    areas
}

fn classify_domain_coupling(
    path: &str,
    families: &BTreeSet<String>,
) -> Result<&'static str, String> {
    if path.starts_with("fixtures/v2-spec/")
        || path.contains("/resources/schemas/v2/")
        || path.starts_with("crates/canisend-store/migrations/")
        || path == "docs/contracts/academic-v2-compatibility-v1.md"
    {
        return Ok("compatibility-surface");
    }
    if path.starts_with("crates/canisend-io/src/discovery")
        || path == "docs/contracts/opportunity-source-adapters-v1.md"
    {
        return Ok("optional-adapter");
    }
    if path.contains("workflow-packs/org.canisend.academic-job")
        || path == "docs/contracts/academic-job-workflow-pack-v1.md"
        || path.contains("/skills/canisend-job-intake/")
        || path.ends_with("/prompts/job-parse.md")
        || path.ends_with("/templates/cover-letter.typ")
        || path.contains("/templates/modernpro-")
        || (path.contains("/resources/") && families.contains("fixed-academic-deliverables"))
    {
        return Ok("academic-pack");
    }
    if families.contains("legacy-job-surface")
        || families.contains("fixed-academic-deliverables")
        || families.contains("fixed-workflow-stage")
    {
        return Ok("compatibility-surface");
    }
    if families.contains("academic-vocabulary") {
        return Ok("kernel");
    }
    Err(format!(
        "domain-coupling finding `{path}` has no ownership classification"
    ))
}

fn check_documentation() -> Result<(), String> {
    let root = repository_root();
    let guide_root = root.join("docs/guides");
    let required = [
        "installation.md",
        "release-verification.md",
        "quick-start.md",
        "agent-integration.md",
        "privacy-and-consent.md",
        "backup-and-recovery.md",
        "upgrade-and-rollback.md",
        "troubleshooting.md",
        "desktop-gui.md",
        "known-limitations.md",
    ];
    for file_name in required {
        let path = guide_root.join(file_name);
        let body = fs::read_to_string(&path)
            .map_err(|error| format!("required guide is missing at {}: {error}", path.display()))?;
        check_local_markdown_links(&root, &path, &body)?;
    }
    let journey_markers: &[(&str, &[&str])] = &[
        (
            "quick-start.md",
            &[
                "workspace init --json",
                "application create",
                "org.canisend.generic-application",
                "org.canisend.academic-job",
                "host setup --host codex",
                "mcp serve",
                "orient -> propose -> preview -> approve -> commit -> verify",
                "Unsupported legacy boundary",
                "submission_performed: false",
            ],
        ),
        (
            "agent-integration.md",
            &[
                "canisend_workspace_status",
                "canisend_application_show",
                "canisend_profile_source_list",
                "canisend_profile_association_list",
                "canisend_profile_association_preview",
                "canisend_profile_association_commit",
                "canisend_evidence_association_list",
                "canisend_evidence_association_preview",
                "canisend_evidence_association_commit",
                "canisend-workspace",
                "canisend-agent-v4.json",
                "orient -> propose -> preview -> approve -> commit -> verify",
                "must never edit `.canisend`",
                "submission_performed` is `false",
            ],
        ),
        (
            "desktop-gui.md",
            &[
                "org.canisend.generic-application",
                "org.canisend.academic-job",
                "Workspace v3",
                "v2→v3 migration",
                "Generic Pack Application",
            ],
        ),
        (
            "privacy-and-consent.md",
            &[
                "org.canisend.generic-application",
                "org.canisend.academic-job",
                "single-use tokens",
                "submission_performed: false",
            ],
        ),
        (
            "backup-and-recovery.md",
            &[
                "exact workflow Pack identity and digest",
                "v2→v3 semantic migration",
                "preserves the Academic Pack",
            ],
        ),
        (
            "upgrade-and-rollback.md",
            &[
                "Discover Pack and Workspace authority before mutation",
                "workspace migration-preview",
                "--expected-plan-sha256",
                "does not silently perform",
            ],
        ),
        (
            "known-limitations.md",
            &[
                "v1.0.0-alpha.5",
                "Pack installation",
                "image-only PDFs",
                "submission_performed: false",
                "Windows and Linux public GUI artifacts are not qualified",
            ],
        ),
    ];
    for (file_name, markers) in journey_markers {
        let path = guide_root.join(file_name);
        let body = fs::read_to_string(&path).map_err(|error| {
            format!(
                "roadmap user-journey guide is missing at {}: {error}",
                path.display()
            )
        })?;
        for marker in *markers {
            if !body.contains(marker) {
                return Err(format!(
                    "roadmap user-journey guide `{file_name}` is missing `{marker}`"
                ));
            }
        }
    }
    for path in [
        root.join("README.md"),
        root.join("AGENTS.md"),
        guide_root.join("README.md"),
        root.join("docs/development/defensive-assurance-routing.md"),
    ] {
        let body = fs::read_to_string(&path).map_err(|error| {
            format!(
                "documentation index is missing at {}: {error}",
                path.display()
            )
        })?;
        check_local_markdown_links(&root, &path, &body)?;
    }
    let repository_scope = fs::read_to_string(root.join("AGENTS.md"))
        .map_err(|error| format!("repository agent scope is missing: {error}"))?;
    for required in [
        "defensive software assurance",
        "Do not turn these tasks into instructions for accessing third-party systems",
        "do not ask the host to disable, downgrade, or bypass its safety policy",
        "Verification tiers",
    ] {
        if !repository_scope.contains(required) {
            return Err(format!("repository agent scope is missing `{required}`"));
        }
    }
    let smoke = root.join("scripts/smoke_documented_quickstart.sh");
    if !smoke.is_file() {
        return Err(format!(
            "documented quick-start smoke is missing at {}",
            smoke.display()
        ));
    }
    let release_runbooks = check_active_release_runbooks(&root)?;
    check_active_release_truth(&root)?;
    println!(
        "documentation: ok ({} guides, {release_runbooks} active release runbooks)",
        required.len()
    );
    Ok(())
}

fn check_active_release_truth(root: &Path) -> Result<(), String> {
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    check_active_release_truth_for_version(root, &version)
}

fn check_active_release_truth_for_version(root: &Path, version: &Version) -> Result<(), String> {
    let roadmap_relative = "docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md";
    let roadmap = fs::read_to_string(root.join(roadmap_relative))
        .map_err(|error| format!("active 1.0 roadmap is missing: {error}"))?;
    for required in [
        "# CanISend generic framework 1.0 delivery roadmap",
        "**Status:** Active — authoritative",
        "**Current machine stage:** Alpha / `pre-beta`",
        "**Next intended checkpoint:** `v1.0.0-alpha.7`",
    ] {
        if !roadmap.contains(required) {
            return Err(format!("active 1.0 roadmap header is missing `{required}`"));
        }
    }
    let public_tag = roadmap
        .lines()
        .find_map(|line| {
            line.strip_prefix("**Current public checkpoint:** [`")
                .and_then(|value| value.split('`').next())
        })
        .filter(|tag| tag.starts_with('v'))
        .ok_or_else(|| "active 1.0 roadmap has no current public checkpoint tag".to_owned())?;

    let parity: Value = serde_json::from_slice(
        &fs::read(root.join("docs/contracts/cli-gui-parity-v1.json"))
            .map_err(|error| format!("CLI/GUI parity manifest is missing: {error}"))?,
    )
    .map_err(|error| format!("CLI/GUI parity manifest is invalid JSON: {error}"))?;
    let operations = parity["entries"]
        .as_array()
        .ok_or_else(|| "CLI/GUI parity entries must be an array".to_owned())?;
    let implemented = operations
        .iter()
        .filter(|entry| entry["status"] == "implemented")
        .count();
    if implemented != operations.len() || operations.is_empty() {
        return Err(
            "active operation count requires every parity entry to be implemented".to_owned(),
        );
    }

    let readme = fs::read_to_string(root.join("README.md"))
        .map_err(|error| format!("README is missing: {error}"))?;
    for required in [
        "## Current status",
        &format!("The checked-in source version is `{version}`"),
        &format!("latest publicly qualified checkpoint is `{public_tag}`"),
        "domain-neutral Rust kernel",
        "org.canisend.generic-application",
        "org.canisend.academic-job",
    ] {
        if !readme.contains(required) {
            return Err(format!("active README status is missing `{required}`"));
        }
    }

    let release = fs::read_to_string(root.join("RELEASE.md"))
        .map_err(|error| format!("root release guide is missing: {error}"))?;
    for required in [
        &format!("Checked-in source: `{version}`"),
        &format!("Latest public checkpoint: [`{public_tag}`]"),
        "GPL-3.0-only",
        "Community signing",
        "not a publicly trusted publisher identity",
        "GitHub build provenance",
    ] {
        if !release.contains(required) {
            return Err(format!("root release guide is missing `{required}`"));
        }
    }
    for stale in [
        "R11.2 Beta hardening is active",
        "macOS Developer ID signing plus accepted notarization",
        "Windows Azure Artifact Signing Public Trust",
    ] {
        if release.contains(stale) {
            return Err(format!(
                "root release guide contains stale current claim `{stale}`"
            ));
        }
    }

    let issue_template = fs::read_to_string(root.join(".github/ISSUE_TEMPLATE/bug.yml"))
        .map_err(|error| format!("bug Issue template is missing: {error}"))?;
    if !issue_template.contains(&format!("placeholder: {version}")) {
        return Err(format!(
            "bug Issue template must suggest the checked-in version `{version}`"
        ));
    }

    let limitations = fs::read_to_string(root.join("release/KNOWN_LIMITATIONS.md"))
        .map_err(|error| format!("release known limitations are missing: {error}"))?;
    let operation_claim = format!(
        "covers all {} declared operation families",
        operations.len()
    );
    for required in [
        operation_claim.as_str(),
        "Community signatures do not establish an operating-system-trusted publisher",
        "Never disable an operating-system security control globally",
    ] {
        if !limitations.contains(required) {
            return Err(format!(
                "release known limitations are missing `{required}`"
            ));
        }
    }

    for (relative, body) in [
        ("README.md", readme.as_str()),
        ("RELEASE.md", release.as_str()),
        (roadmap_relative, roadmap.as_str()),
        (".github/ISSUE_TEMPLATE/bug.yml", issue_template.as_str()),
        ("release/KNOWN_LIMITATIONS.md", limitations.as_str()),
    ] {
        if body.contains("35 declared operation families") {
            return Err(format!(
                "active surface `{relative}` retains the stale 35-operation claim"
            ));
        }
    }
    println!(
        "active release truth: ok ({version}, public {public_tag}, {} operations)",
        operations.len()
    );
    Ok(())
}

fn check_active_release_runbooks(root: &Path) -> Result<usize, String> {
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let release = format!("v{}.{}.{}", version.major, version.minor, version.patch);
    let release_line = format!("{}.{}", version.major, version.minor);
    let beta = format!("{release}-beta.1");
    let rc_one = format!("{release}-rc.1");
    let rc_two = format!("{release}-rc.2");
    let runbooks = [
        (
            "docs/release/stage-transitions.md",
            vec![beta.clone(), rc_two.clone()],
        ),
        (
            "docs/release/qualification-ledger.md",
            vec![beta.clone(), rc_one.clone(), rc_two],
        ),
        (
            "docs/release/package-manager-qualification.md",
            vec![beta.clone(), rc_one.clone()],
        ),
        (
            "docs/release/upgrade-qualification.md",
            vec![beta.clone(), rc_one.clone()],
        ),
        (
            "docs/release/documentation-uninstall-qualification.md",
            vec![rc_one],
        ),
        (
            "docs/release/support-policy.md",
            vec![
                format!("# CanISend {release_line} Support Policy"),
                format!("Rust-native `{release_line}` line"),
            ],
        ),
        (
            "docs/release/signing-operations.md",
            vec![format!("not `{release_line}` release prerequisites")],
        ),
    ];
    for (relative, required) in &runbooks {
        let path = root.join(relative);
        let body = fs::read_to_string(&path)
            .map_err(|error| format!("active release runbook `{relative}` is missing: {error}"))?;
        check_local_markdown_links(root, &path, &body)?;
        for value in required {
            if !body.contains(value) {
                return Err(format!(
                    "active release runbook `{relative}` is missing current-line example `{value}`"
                ));
            }
        }
        for stale in ["v0.7.0-beta.", "v0.7.0-rc."] {
            if body.contains(stale) {
                return Err(format!(
                    "active release runbook `{relative}` still contains stale example `{stale}`"
                ));
            }
        }
    }
    let support = fs::read_to_string(root.join("docs/release/support-policy.md"))
        .map_err(|error| format!("support policy documentation is missing: {error}"))?;
    for stale in ["CanISend 0.7", "`0.7`", "0.7.x"] {
        if support.contains(stale) {
            return Err(format!(
                "active support policy still contains stale release-line text `{stale}`"
            ));
        }
    }
    Ok(runbooks.len())
}

fn check_release_notes_policy() -> Result<(), String> {
    let root = repository_root();
    let policy_path = root.join("release/release-notes-policy.json");
    let policy: Value = serde_json::from_slice(&fs::read(&policy_path).map_err(|error| {
        format!(
            "release-notes policy is missing at {}: {error}",
            policy_path.display()
        )
    })?)
    .map_err(|error| format!("release-notes policy is invalid JSON: {error}"))?;
    let sections = [
        "Highlights",
        "Compatibility",
        "Install and verify",
        "Upgrade and rollback",
        "Security and privacy",
        "Known limitations",
        "Feedback and support",
    ];
    let guidance = [
        "does not require Python",
        "canisend.workspace/v4",
        "canisend.agent/v4",
        "never submits an application",
        "SHA256SUMS",
        "GitHub build provenance",
        "back up every important workspace",
        "restore the pre-upgrade backup into a new directory",
        "no in-place database downgrade",
        "no telemetry",
        "KNOWN_LIMITATIONS.md",
        "Never attach a workspace",
    ];
    let guides = [
        "docs/guides/release-verification.md",
        "docs/guides/quick-start.md",
        "docs/guides/upgrade-and-rollback.md",
    ];
    let expected = json!({
        "schema": RELEASE_NOTES_POLICY_SCHEMA,
        "stage_neutral_body": true,
        "heading_tracks_workspace_version": true,
        "required_sections": sections,
        "required_guidance": guidance,
        "required_repository_guides": guides,
        "final_review_required_at_rc": true,
        "final_review_evidence": {
            "asset": "RELEASE_NOTES.md",
            "bind_latest_recorded_rc": true,
            "bind_release_manifest": true,
            "bind_release_notes_body": true,
            "bind_rollback_guide": true,
            "explicit_github_reviewer": true,
            "rc_iteration_resets_review": true
        }
    });
    if policy != expected {
        return Err("release-notes policy differs from the native release contract".to_owned());
    }
    let workspace_body = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    let version = Version::parse(
        workspace["workspace"]["package"]["version"]
            .as_str()
            .ok_or_else(|| "workspace manifest has no package version".to_owned())?,
    )
    .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let notes_path = root.join("release/RELEASE_NOTES.md");
    let notes = fs::read_to_string(&notes_path)
        .map_err(|error| format!("release notes are missing: {error}"))?;
    validate_release_notes(&root, &version, &notes, &sections, &guidance, &guides)?;
    println!(
        "release notes: ok ({} stage-neutral sections, RC final review required)",
        sections.len()
    );
    Ok(())
}

fn validate_release_notes(
    root: &Path,
    version: &Version,
    notes: &str,
    expected_sections: &[&str],
    required_guidance: &[&str],
    guides: &[&str],
) -> Result<(), String> {
    let mut lines = notes.lines();
    let expected_heading = format!("# CanISend {version}");
    if lines.next() != Some(expected_heading.as_str())
        || notes.matches(expected_heading.as_str()).count() != 1
    {
        return Err(
            "release-note heading must identify the exact workspace version once".to_owned(),
        );
    }
    let sections = notes
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .collect::<Vec<_>>();
    if sections != expected_sections {
        return Err(format!(
            "release-note sections differ: expected {expected_sections:?}, found {sections:?}"
        ));
    }
    let normalized = notes.split_whitespace().collect::<Vec<_>>().join(" ");
    for phrase in required_guidance {
        if !normalized.contains(phrase) {
            return Err(format!(
                "release notes are missing required guidance `{phrase}`"
            ));
        }
    }
    let body = notes
        .split_once('\n')
        .map(|(_, body)| body)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if body
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|word| matches!(word, "alpha" | "beta" | "prerelease" | "stable"))
        || body.contains("release candidate")
    {
        return Err(
            "release-note body must remain stage-neutral; only the version heading may change"
                .to_owned(),
        );
    }
    for guide in guides {
        if !root.join(guide).is_file() {
            return Err(format!("release-note guide is missing: {guide}"));
        }
        let url = format!("https://github.com/jxpeng98/CanISend/blob/main/{guide}");
        if !notes.contains(&url) {
            return Err(format!(
                "release notes do not link required guide `{guide}`"
            ));
        }
    }
    Ok(())
}

fn check_fuzz_policy() -> Result<(), String> {
    let root = repository_root();
    let workflow_path = root.join(".github/workflows/fuzz.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("scheduled fuzz workflow is missing: {error}"))?;
    let manifest_path = root.join("fuzz/Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("fuzz manifest is missing: {error}"))?;
    let targets = ["structured_inputs", "intake_parsers", "pdf_extract"];
    for required in [
        "schedule:",
        "workflow_dispatch:",
        FUZZ_TOOLCHAIN,
        CARGO_FUZZ_VERSION,
        "-max_total_time=300",
        "-timeout=15",
        "upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
    ] {
        if !workflow.contains(required) {
            return Err(format!("scheduled fuzz workflow is missing `{required}`"));
        }
    }
    for required in [
        "cargo-fuzz = true",
        "libfuzzer-sys = \"=0.4.13\"",
        "canisend-contracts",
        "canisend-io",
    ] {
        if !manifest.contains(required) {
            return Err(format!("fuzz manifest is missing `{required}`"));
        }
    }
    let workspace_body = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("workspace manifest is missing: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    let workspace_version = workspace["workspace"]["package"]["version"]
        .as_str()
        .ok_or_else(|| "workspace manifest has no package version".to_owned())?;
    let exact_internal_version = format!("version = \"={workspace_version}\"");
    if manifest.matches(&exact_internal_version).count() != 2 {
        return Err(format!(
            "fuzz manifest internal dependencies must use `{exact_internal_version}`"
        ));
    }
    let fuzz_lock_body = fs::read_to_string(root.join("fuzz/Cargo.lock"))
        .map_err(|error| format!("fuzz lockfile is missing: {error}"))?;
    let fuzz_lock: toml::Value = fuzz_lock_body
        .parse()
        .map_err(|error| format!("fuzz lockfile is invalid TOML: {error}"))?;
    let packages = fuzz_lock["package"]
        .as_array()
        .ok_or_else(|| "fuzz lockfile has no package entries".to_owned())?;
    let mut internal_package_count = 0;
    for package in packages {
        let Some(name) = package["name"].as_str() else {
            continue;
        };
        if !name.starts_with("canisend-") || name == "canisend-fuzz" {
            continue;
        }
        internal_package_count += 1;
        let version = package["version"]
            .as_str()
            .ok_or_else(|| format!("fuzz lockfile package `{name}` has no version"))?;
        if version != workspace_version {
            return Err(format!(
                "fuzz lockfile package `{name}` uses {version}, expected {workspace_version}"
            ));
        }
    }
    if internal_package_count == 0 {
        return Err("fuzz lockfile has no internal CanISend packages".to_owned());
    }
    for target in targets {
        if !workflow.contains(target)
            || !manifest.contains(&format!("name = \"{target}\""))
            || !root
                .join(format!("fuzz/fuzz_targets/{target}.rs"))
                .is_file()
        {
            return Err(format!("scheduled fuzz target `{target}` is incomplete"));
        }
    }
    let documentation_path = root.join("docs/testing/scheduled-fuzzing.md");
    let documentation = fs::read_to_string(&documentation_path)
        .map_err(|error| format!("scheduled fuzz documentation is missing: {error}"))?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    println!("fuzz policy: ok ({} scheduled targets)", targets.len());
    Ok(())
}

fn check_property_test_policy() -> Result<(), String> {
    let root = repository_root();
    let test_path = root.join("crates/canisend-contracts/tests/property_contract.rs");
    let test = fs::read_to_string(&test_path)
        .map_err(|error| format!("property-test target is missing: {error}"))?;
    for required in [
        "property_generated_portable_paths_round_trip_without_normalization",
        "property_inserting_any_reserved_component_is_always_rejected",
        "property_generated_sha256_digests_round_trip_and_mutations_fail",
        "property_generated_uuidv7_and_revisions_preserve_identity",
        "GENERATED_CASES: usize = 512",
    ] {
        if !test.contains(required) {
            return Err(format!("property-test target is missing `{required}`"));
        }
    }

    let command = "cargo test -p canisend-contracts --locked --test property_contract";
    for workflow in [
        ".github/workflows/fast-ci.yml",
        ".github/workflows/release.yml",
    ] {
        let path = root.join(workflow);
        let body = fs::read_to_string(&path)
            .map_err(|error| format!("property-test workflow `{workflow}` is missing: {error}"))?;
        if !body.contains(command) {
            return Err(format!(
                "property-test workflow `{workflow}` is missing `{command}`"
            ));
        }
    }

    let documentation_path = root.join("docs/testing/property-testing.md");
    let documentation = fs::read_to_string(&documentation_path)
        .map_err(|error| format!("property-test documentation is missing: {error}"))?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    println!("property-test policy: ok (4 generated properties)");
    Ok(())
}

fn check_local_markdown_links(root: &Path, source: &Path, body: &str) -> Result<(), String> {
    let parent = source
        .parent()
        .ok_or_else(|| format!("documentation path has no parent: {}", source.display()))?;
    let mut remaining = body;
    while let Some(start) = remaining.find("](") {
        let target_start = start + 2;
        remaining = &remaining[target_start..];
        let Some(end) = remaining.find(')') else {
            return Err(format!(
                "unterminated Markdown link in {}",
                source.display()
            ));
        };
        let destination = remaining[..end].trim();
        remaining = &remaining[end + 1..];
        if destination.is_empty()
            || destination.starts_with('#')
            || destination.starts_with("http://")
            || destination.starts_with("https://")
            || destination.starts_with("mailto:")
        {
            continue;
        }
        let relative = destination
            .split('#')
            .next()
            .expect("split always returns one element");
        let candidate = parent.join(relative);
        if !candidate.exists() {
            return Err(format!(
                "broken local link `{destination}` in {}",
                source.strip_prefix(root).unwrap_or(source).display()
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReleaseTarget {
    triple: String,
    runner: String,
    executable: String,
    archive: String,
    signing: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReleaseStatusGitFacts {
    head_commit: String,
    worktree_dirty: bool,
    public_tag: String,
    public_version: Version,
    public_commit: String,
    source_commits_ahead: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReleaseStatusSources {
    workspace_version: Version,
    workspace_license: String,
    qualification: Value,
    support: Value,
    targets: Value,
    alpha_package: Value,
    beta_readiness: Value,
    beta_freeze: Value,
    feedback: Value,
    cli_gui_parity: Value,
    svelte_parity: Value,
    signing: Value,
    git: ReleaseStatusGitFacts,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ChannelArtifact {
    target: String,
    archive: String,
    sha256: String,
    size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ChannelCandidateSource {
    tag: String,
    version: String,
    stage: ReleaseStage,
    source_commit: String,
    repository: String,
    manifest_file: String,
    manifest_sha256: String,
    artifacts: BTreeMap<String, ChannelArtifact>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct UpgradeQualificationSummary {
    run_id: u64,
    from_manifest_sha256: String,
    to_manifest_sha256: String,
    records: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DocumentationUninstallSummary {
    run_id: u64,
    records: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PackageManagerQualificationSummary {
    run_id: u64,
    records: usize,
}

impl ChannelCandidateSource {
    fn to_value(&self) -> Value {
        let artifacts = self
            .artifacts
            .values()
            .map(|artifact| {
                json!({
                    "target": artifact.target,
                    "archive": artifact.archive,
                    "sha256": artifact.sha256,
                    "size": artifact.size,
                })
            })
            .collect::<Vec<_>>();
        json!({
            "schema": CHANNEL_CANDIDATE_SOURCE_SCHEMA,
            "candidate_only": true,
            "publication_authorized": false,
            "release": {
                "tag": self.tag,
                "version": self.version,
                "stage": self.stage.as_str(),
                "source_commit": self.source_commit,
                "repository": self.repository,
                "manifest_file": self.manifest_file,
                "manifest_sha256": self.manifest_sha256,
            },
            "artifacts": artifacts,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReleaseStage {
    Alpha,
    Beta,
    ReleaseCandidate,
    Stable,
}

impl ReleaseStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Alpha => "alpha",
            Self::Beta => "beta",
            Self::ReleaseCandidate => "rc",
            Self::Stable => "stable",
        }
    }

    fn from_version(version: &Version) -> Result<Self, String> {
        let prerelease = version.pre.as_str();
        if prerelease.starts_with("alpha.") {
            Ok(Self::Alpha)
        } else if prerelease.starts_with("beta.") {
            Ok(Self::Beta)
        } else if prerelease.starts_with("rc.") {
            Ok(Self::ReleaseCandidate)
        } else if prerelease.is_empty() {
            Ok(Self::Stable)
        } else {
            Err(format!(
                "release tag prerelease `{prerelease}` is not alpha, beta, or rc"
            ))
        }
    }

    fn requires_intel_gui_release_evidence(self) -> bool {
        !matches!(self, Self::Alpha)
    }

    fn cargo_profile(self) -> &'static str {
        match self {
            Self::Alpha => "release-alpha",
            Self::Beta | Self::ReleaseCandidate | Self::Stable => "release",
        }
    }
}

fn windows_msi_product_version(version: &Version) -> Result<String, String> {
    if version.major > 255 || version.minor > 255 || version.patch > 255 {
        return Err(
            "MSI product versions support SemVer major, minor, and patch values up to 255"
                .to_owned(),
        );
    }

    let stage = ReleaseStage::from_version(version)?;
    let iteration = match stage {
        ReleaseStage::Alpha => Some(prerelease_iteration(version, "alpha")?),
        ReleaseStage::Beta => Some(prerelease_iteration(version, "beta")?),
        ReleaseStage::ReleaseCandidate => Some(prerelease_iteration(version, "rc")?),
        ReleaseStage::Stable => None,
    };
    if iteration.is_some_and(|iteration| iteration > 63) {
        return Err("MSI prerelease iterations must be between 1 and 63".to_owned());
    }
    let stage_build = match stage {
        ReleaseStage::Alpha => iteration.expect("Alpha iteration was validated"),
        ReleaseStage::Beta => 64 + iteration.expect("Beta iteration was validated"),
        ReleaseStage::ReleaseCandidate => 128 + iteration.expect("RC iteration was validated"),
        ReleaseStage::Stable => 255,
    };

    let build = version
        .patch
        .checked_mul(256)
        .and_then(|patch| patch.checked_add(stage_build))
        .ok_or_else(|| "MSI product version build field overflowed".to_owned())?;
    if build > 65_535 {
        return Err("MSI product version build field exceeds 65,535".to_owned());
    }
    Ok(format!("{}.{}.{}", version.major, version.minor, build))
}

struct RenderedStageTransition {
    from_version: Version,
    to_version: Version,
    from_stage: ReleaseStage,
    to_stage: ReleaseStage,
    files: BTreeMap<String, Vec<u8>>,
}

struct RenderedReleaseLineActivation {
    from_version: Version,
    to_version: Version,
    source_commit: String,
    history_line: String,
    files: BTreeMap<String, Vec<u8>>,
}

struct RenderedFeatureFreeze {
    baseline: String,
    files: BTreeMap<String, Vec<u8>>,
}

struct RenderedReleaseQualification {
    tag: String,
    run_id: u64,
    source_commit: String,
    manifest_sha256: String,
    ledger: Vec<u8>,
}

fn check_stage_transition_policy() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/stage-transition-policy.json");
    let policy: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "stage-transition policy is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("stage-transition policy is invalid JSON: {error}"))?;
    let expected = json!({
        "schema": STAGE_TRANSITION_POLICY_SCHEMA,
        "command": {
            "name": "cargo run -p xtask --locked -- release prepare-stage",
            "dry_run_default": true,
            "write_flag": "--write",
            "clean_worktree_required_for_write": true,
            "beta_readiness_max_age_hours": BETA_READINESS_MAX_AGE_HOURS
        },
        "allowed_transitions": [
            {
                "from": "alpha",
                "to": "beta",
                "target_prerelease": "beta.1",
                "ledger_status": "beta-qualifying",
                "release_notes_status": "beta-current"
            },
            {
                "from": "beta",
                "to": "rc",
                "target_prerelease": "rc.1",
                "ledger_status": "rc-qualifying",
                "release_notes_status": "rc-final"
            },
            {
                "from": "rc",
                "to": "stable",
                "target_prerelease": "",
                "ledger_status": "qualified",
                "release_notes_status": "stable-final"
            }
        ],
        "allowed_iterations": [
            {
                "stage": "alpha",
                "target_prerelease": "next-sequential-alpha",
                "ledger_status": "pre-beta",
                "release_notes_status": "alpha-current",
                "invalidates": [
                    "release/beta-readiness.json",
                    "release/beta-contract-freeze.json",
                    "release/feedback-snapshot.json"
                ]
            },
            {
                "stage": "rc",
                "target_prerelease": "next-sequential-rc",
                "ledger_status": "rc-qualifying",
                "release_notes_status": "rc-final"
            }
        ],
        "controlled_surfaces": [
            "Cargo.toml workspace version",
            "workspace Cargo.toml exact internal dependencies",
            "fuzz/Cargo.toml exact internal dependencies",
            "Cargo.lock workspace package versions",
            "fuzz/Cargo.lock internal package versions",
            "desktop and native-preview npm package versions plus the desktop fallback version",
            "docs/contracts/cli-gui-parity-v1.json Alpha scope",
            "docs/performance/macos-gui-alpha-baseline.json source version",
            "release/alpha-package-contract.json versioned asset names",
            ".github/workflows/release.yml dispatch default",
            "active README, root release guide, and known-limitations source-version claims",
            "sequential-Alpha pending readiness, freeze, and feedback identities",
            "release/qualification-ledger.json stage, Stable authorization, and RC notes-review reset fields",
            "release/RELEASE_NOTES.md heading",
            "release/support-policy.json Stable publication status",
            "release/feedback-snapshot.json next-roadmap publication status",
            "snapshot-declared next-roadmap publication marker"
        ],
        "preserved_history": [
            "release/beta-readiness.json",
            "release/beta-contract-freeze.json",
            "release/feedback-snapshot.json measured public metadata",
            "packaging/candidates/alpha"
        ]
    });
    if policy != expected {
        return Err(
            "stage-transition policy differs from the fail-closed release contract".to_owned(),
        );
    }
    let documentation_path = root.join("docs/release/stage-transitions.md");
    let documentation = fs::read_to_string(&documentation_path)
        .map_err(|error| format!("stage-transition runbook is missing: {error}"))?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let beta_example = format!(
        "prepare-stage v{}.{}.{}-beta.1",
        version.major, version.minor, version.patch
    );
    for required in [
        "release/stage-transition-policy.json",
        "prepare-stage v1.0.0-alpha.6",
        "sequential Alpha",
        beta_example.as_str(),
        "--write",
        "release/beta-readiness.json",
        "refresh_beta_readiness.sh",
        "refresh_release_feedback.sh",
        "Reviewed",
        "Published",
    ] {
        if !documentation.contains(required) {
            return Err(format!("stage-transition runbook is missing `{required}`"));
        }
    }
    let refresh_path = root.join("scripts/refresh_beta_readiness.sh");
    let refresh = fs::read_to_string(&refresh_path)
        .map_err(|error| format!("Beta-readiness refresh script is missing: {error}"))?;
    for required in [
        "gh api --paginate --slurp",
        "select(has(\"pull_request\") | not)",
        "verify-beta-readiness",
        "open_issue_count",
        "--write",
    ] {
        if !refresh.contains(required) {
            return Err(format!(
                "Beta-readiness refresh script is missing `{required}`"
            ));
        }
    }
    println!("stage-transition policy: ok (3 stage transitions + sequential Alpha/RC iteration)");
    Ok(())
}

fn prepare_stage_transition(tag: &str, write: bool) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "stage transition")?;
    }
    let transition = render_stage_transition(&root, tag)?;
    if write && matches!(transition.to_stage, ReleaseStage::Beta) {
        check_beta_readiness_freshness(&root, OffsetDateTime::now_utc())?;
    }
    let report = stage_transition_report(&root, &transition, write)?;
    if write {
        write_controlled_files_transactionally(&root, &transition.files, None)?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("could not serialize stage-transition plan: {error}"))?
    );
    Ok(())
}

fn activate_release_line(tag: &str, write: bool) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "release-line activation")?;
    }
    let source_commit = run_git_lines(&root, &["rev-parse", "HEAD"])?
        .into_iter()
        .next()
        .ok_or_else(|| "could not resolve release-line activation source commit".to_owned())?;
    validate_lower_hex("release-line activation source commit", &source_commit, 40)?;
    let activation = render_release_line_activation(&root, tag, &source_commit)?;
    let report = release_line_activation_report(&root, &activation, write)?;
    if write {
        write_controlled_files_transactionally(&root, &activation.files, None)?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| format!(
            "could not serialize release-line activation plan: {error}"
        ))?
    );
    Ok(())
}

fn render_release_line_activation(
    root: &Path,
    tag: &str,
    source_commit: &str,
) -> Result<RenderedReleaseLineActivation, String> {
    validate_lower_hex("release-line activation source commit", source_commit, 40)?;
    let workspace_path = root.join("Cargo.toml");
    let workspace_body = fs::read_to_string(&workspace_path)
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    let from_version = Version::parse(
        workspace["workspace"]["package"]["version"]
            .as_str()
            .ok_or_else(|| "workspace manifest has no package version".to_owned())?,
    )
    .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let (to_version, to_stage) = parse_release_tag(tag)?;
    validate_release_line_target(&from_version, &to_version, to_stage)?;
    let history_line = format!("{}.{}", from_version.major, from_version.minor);
    let history_root = format!("release/history/{history_line}");
    if root.join(&history_root).exists() {
        return Err(format!(
            "release history destination `{history_root}` already exists"
        ));
    }

    let mut files = render_workspace_version_update(
        root,
        &workspace,
        &workspace_body,
        &from_version,
        &to_version,
    )?;
    let archive_sources = [
        "release/RELEASE_NOTES.md",
        "release/beta-contract-freeze.json",
        "release/beta-readiness.json",
        "release/feature-freeze-exceptions.json",
        "release/feedback-snapshot.json",
        "release/qualification-ledger.json",
        "release/support-policy.json",
    ];
    let mut archived_files = Vec::new();
    for source_path in archive_sources {
        let body = fs::read(root.join(source_path)).map_err(|error| {
            format!("could not read historical source `{source_path}`: {error}")
        })?;
        let name = Path::new(source_path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("historical source path is invalid: {source_path}"))?;
        let archive_path = format!("{history_root}/{name}");
        archived_files.push(json!({
            "archive_path": archive_path,
            "sha256": sha256(&body),
            "source_path": source_path
        }));
        files.insert(archive_path, body);
    }
    let candidate_tree_sha256 =
        digest_regular_file_tree(&root.join("packaging/candidates/v0.7.0-alpha.1"))?;
    let history_manifest = json!({
        "schema": RELEASE_HISTORY_SCHEMA,
        "release_line": history_line,
        "archived_from_version": from_version.to_string(),
        "activation_source_commit": source_commit,
        "files": archived_files,
        "references": [{
            "kind": "repository-tree",
            "path": "packaging/candidates/v0.7.0-alpha.1",
            "tree_sha256": candidate_tree_sha256
        }]
    });
    files.insert(
        format!("{history_root}/manifest.json"),
        pretty_json_bytes(&history_manifest)?,
    );

    files.insert(
        "release/qualification-ledger.json".to_owned(),
        pretty_json_bytes(&initial_alpha_qualification_ledger())?,
    );
    files.insert(
        "release/feature-freeze-exceptions.json".to_owned(),
        pretty_json_bytes(&json!({
            "schema": FEATURE_FREEZE_EXCEPTIONS_SCHEMA,
            "status": "planned",
            "baseline_commit": null,
            "exceptions": []
        }))?,
    );
    files.insert(
        "release/support-policy.json".to_owned(),
        pretty_json_bytes(&build_support_policy(&to_version)?)?,
    );
    files.insert(
        "release/beta-readiness.json".to_owned(),
        pretty_json_bytes(&pending_beta_readiness(&to_version)?)?,
    );
    files.insert(
        "release/beta-contract-freeze.json".to_owned(),
        pretty_json_bytes(&pending_beta_contract_freeze(&to_version)?)?,
    );
    files.insert(
        "release/feedback-snapshot.json".to_owned(),
        pretty_json_bytes(&pending_release_feedback(&to_version)?)?,
    );
    files.insert(
        "release/RELEASE_NOTES.md".to_owned(),
        release_notes_for_version(&to_version).into_bytes(),
    );

    for (relative, after) in &files {
        let path = root.join(relative);
        if path.exists() {
            reject_symlink(&path)?;
            let before = fs::read(&path)
                .map_err(|error| format!("could not read controlled file `{relative}`: {error}"))?;
            if before == *after {
                return Err(format!(
                    "release-line activation would not change controlled file `{relative}`"
                ));
            }
        }
    }
    Ok(RenderedReleaseLineActivation {
        from_version,
        to_version,
        source_commit: source_commit.to_owned(),
        history_line,
        files,
    })
}

fn validate_release_line_target(
    from_version: &Version,
    to_version: &Version,
    to_stage: ReleaseStage,
) -> Result<(), String> {
    if to_stage != ReleaseStage::Alpha
        || to_version.patch != 0
        || to_version.pre.as_str() != "alpha.1"
        || !to_version.build.is_empty()
        || (to_version.major, to_version.minor) <= (from_version.major, from_version.minor)
    {
        return Err(
            "release-line activation requires a later X.Y.0-alpha.1 tag without build metadata"
                .to_owned(),
        );
    }
    Ok(())
}

fn render_workspace_version_update(
    root: &Path,
    workspace: &toml::Value,
    workspace_body: &str,
    from_version: &Version,
    to_version: &Version,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let members = workspace["workspace"]["members"]
        .as_array()
        .ok_or_else(|| "workspace manifest has no members array".to_owned())?
        .iter()
        .map(|member| {
            member
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "workspace member must be a string".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut package_names = BTreeSet::new();
    for member in &members {
        let manifest: toml::Value = fs::read_to_string(root.join(member).join("Cargo.toml"))
            .map_err(|error| format!("could not read {member}/Cargo.toml: {error}"))?
            .parse()
            .map_err(|error| format!("{member}/Cargo.toml is invalid TOML: {error}"))?;
        package_names.insert(
            manifest["package"]["name"]
                .as_str()
                .ok_or_else(|| format!("{member}/Cargo.toml has no package name"))?
                .to_owned(),
        );
    }
    let from = from_version.to_string();
    let to = to_version.to_string();
    let mut files = BTreeMap::new();
    files.insert(
        "Cargo.toml".to_owned(),
        replace_exact_count(
            workspace_body,
            &format!("version = \"{from}\""),
            &format!("version = \"{to}\""),
            1,
            "workspace version",
        )?
        .into_bytes(),
    );
    for member in &members {
        let relative = format!("{member}/Cargo.toml");
        let body = fs::read_to_string(root.join(&relative))
            .map_err(|error| format!("could not read {relative}: {error}"))?;
        let needle = format!("version = \"={from}\"");
        let occurrences = body.matches(&needle).count();
        if occurrences > 0 {
            files.insert(
                relative.clone(),
                replace_exact_count(
                    &body,
                    &needle,
                    &format!("version = \"={to}\""),
                    occurrences,
                    &format!("internal dependency versions in {relative}"),
                )?
                .into_bytes(),
            );
        }
    }
    let fuzz_manifest = "fuzz/Cargo.toml";
    if root.join(fuzz_manifest).is_file() {
        let body = fs::read_to_string(root.join(fuzz_manifest))
            .map_err(|error| format!("could not read {fuzz_manifest}: {error}"))?;
        let needle = format!("version = \"={from}\"");
        let occurrences = body.matches(&needle).count();
        if occurrences == 0 {
            return Err(format!(
                "{fuzz_manifest} has no internal dependencies pinned to {from}"
            ));
        }
        files.insert(
            fuzz_manifest.to_owned(),
            replace_exact_count(
                &body,
                &needle,
                &format!("version = \"={to}\""),
                occurrences,
                "fuzz manifest internal dependency versions",
            )?
            .into_bytes(),
        );
    }
    let mut lock = fs::read_to_string(root.join("Cargo.lock"))
        .map_err(|error| format!("could not read Cargo.lock: {error}"))?;
    for package in &package_names {
        lock = replace_exact_count(
            &lock,
            &format!("name = \"{package}\"\nversion = \"{from}\""),
            &format!("name = \"{package}\"\nversion = \"{to}\""),
            1,
            &format!("Cargo.lock package `{package}`"),
        )?;
    }
    files.insert("Cargo.lock".to_owned(), lock.into_bytes());

    let fuzz_lock = "fuzz/Cargo.lock";
    if root.join(fuzz_lock).is_file() {
        let body = fs::read_to_string(root.join(fuzz_lock))
            .map_err(|error| format!("could not read {fuzz_lock}: {error}"))?;
        let parsed: toml::Value = body
            .parse()
            .map_err(|error| format!("{fuzz_lock} is invalid TOML: {error}"))?;
        let packages = parsed["package"]
            .as_array()
            .ok_or_else(|| format!("{fuzz_lock} has no package entries"))?;
        let mut names = BTreeSet::new();
        for package in packages {
            let Some(name) = package["name"].as_str() else {
                continue;
            };
            if !name.starts_with("canisend-") || name == "canisend-fuzz" {
                continue;
            }
            if package["version"].as_str() != Some(from.as_str()) {
                return Err(format!("{fuzz_lock} package `{name}` does not use {from}"));
            }
            names.insert(name.to_owned());
        }
        if names.is_empty() {
            return Err(format!("{fuzz_lock} has no internal CanISend packages"));
        }
        let mut updated = body;
        for package in names {
            updated = replace_exact_count(
                &updated,
                &format!("name = \"{package}\"\nversion = \"{from}\""),
                &format!("name = \"{package}\"\nversion = \"{to}\""),
                1,
                &format!("{fuzz_lock} package `{package}`"),
            )?;
        }
        files.insert(fuzz_lock.to_owned(), updated.into_bytes());
    }
    insert_desktop_version_updates(root, &mut files, from_version, to_version)?;
    Ok(files)
}

fn insert_desktop_version_updates(
    root: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
    from_version: &Version,
    to_version: &Version,
) -> Result<(), String> {
    let from = from_version.to_string();
    let to = to_version.to_string();
    for (relative, context) in [
        (
            "apps/canisend-desktop/package.json",
            "desktop package version",
        ),
        (
            "crates/canisend-desktop/tauri.conf.json",
            "Tauri application version",
        ),
    ] {
        if !root.join(relative).is_file() {
            continue;
        }
        let body = fs::read_to_string(root.join(relative))
            .map_err(|error| format!("could not read {relative}: {error}"))?;
        files.insert(
            relative.to_owned(),
            replace_exact_count(
                &body,
                &format!("\"version\": \"{from}\""),
                &format!("\"version\": \"{to}\""),
                1,
                context,
            )?
            .into_bytes(),
        );
    }

    let windows_relative = "crates/canisend-desktop/tauri.windows.conf.json";
    if root.join(windows_relative).is_file() {
        let body = fs::read_to_string(root.join(windows_relative))
            .map_err(|error| format!("could not read {windows_relative}: {error}"))?;
        let from_msi = windows_msi_product_version(from_version)?;
        let to_msi = windows_msi_product_version(to_version)?;
        files.insert(
            windows_relative.to_owned(),
            replace_exact_count(
                &body,
                &format!("\"version\": \"{from_msi}\""),
                &format!("\"version\": \"{to_msi}\""),
                1,
                "Windows MSI product version",
            )?
            .into_bytes(),
        );
    }
    Ok(())
}

fn initial_alpha_qualification_ledger() -> Value {
    json!({
        "schema": RELEASE_QUALIFICATION_SCHEMA,
        "workspace_stage": "alpha",
        "status": "pre-beta",
        "stable_authorized": false,
        "beta": {"status": "pending"},
        "feature_freeze": {
            "allowed_change_classes": [
                "documentation",
                "release-blocker",
                "release-evidence"
            ],
            "baseline_commit": null,
            "status": "planned"
        },
        "release_candidates": [],
        "upgrade_matrix": {
            "status": "pending",
            "beta_tag": null,
            "rc_tag": null,
            "evidence": []
        },
        "documentation_uninstall": {
            "status": "prepared-local",
            "native_matrix_run": null,
            "evidence": []
        },
        "package_managers": {
            "channels": ["homebrew-cask", "scoop", "winget"],
            "evidence": [],
            "status": "candidates-only"
        },
        "release_notes": {
            "notes": "release/RELEASE_NOTES.md",
            "review": null,
            "rollback": "docs/guides/upgrade-and-rollback.md",
            "status": "alpha-current"
        }
    })
}

fn pending_beta_readiness(version: &Version) -> Result<Value, String> {
    Ok(json!({
        "schema": BETA_READINESS_SCHEMA,
        "status": "pending-alpha-publication",
        "release_line": format!("{}.{}", version.major, version.minor),
        "alpha_release": {
            "tag": alpha_tag_for_version(version)?,
            "source_commit": null,
            "release_run": null,
            "release_url": null
        },
        "default_telemetry": false,
        "unresolved_release_blockers": []
    }))
}

fn pending_beta_contract_freeze(version: &Version) -> Result<Value, String> {
    Ok(json!({
        "schema": BETA_CONTRACT_FREEZE_SCHEMA,
        "status": "pending-alpha-publication",
        "release_line": format!("{}.{}", version.major, version.minor),
        "baseline": {
            "release": alpha_tag_for_version(version)?,
            "source_commit": null
        }
    }))
}

fn pending_release_feedback(version: &Version) -> Result<Value, String> {
    Ok(json!({
        "schema": FEEDBACK_SNAPSHOT_SCHEMA,
        "status": "pending-alpha-publication",
        "release_line": format!("{}.{}", version.major, version.minor),
        "default_telemetry": false,
        "privacy_boundary": "public-metadata-only",
        "expected_release": {
            "tag": alpha_tag_for_version(version)?
        },
        "next_roadmap": {
            "path": "docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md",
            "status": "active"
        }
    }))
}

fn pending_release_feedback_is_canonical(
    snapshot: &Value,
    active: &Version,
) -> Result<bool, String> {
    if snapshot == &pending_release_feedback(active)? {
        return Ok(true);
    }
    if ReleaseStage::from_version(active) != Ok(ReleaseStage::Alpha)
        || prerelease_iteration(active, "alpha")? >= 6
    {
        return Ok(false);
    }
    let initial = Version::parse(&format!(
        "{}.{}.{}-alpha.1",
        active.major, active.minor, active.patch
    ))
    .map_err(|error| format!("initial Alpha feedback identity is invalid: {error}"))?;
    Ok(snapshot == &pending_release_feedback(&initial)?)
}

fn alpha_tag_for_version(version: &Version) -> Result<String, String> {
    if version.patch != 0 || ReleaseStage::from_version(version) != Ok(ReleaseStage::Alpha) {
        return Err(
            "release-line Alpha identity requires an Alpha with patch version zero".to_owned(),
        );
    }
    Ok(format!("v{version}"))
}

fn validate_alpha_baseline_tag(active: &Version, tag: &str) -> Result<Version, String> {
    let candidate = Version::parse(
        tag.strip_prefix('v')
            .ok_or_else(|| "Alpha baseline tag must start with `v`".to_owned())?,
    )
    .map_err(|error| format!("Alpha baseline tag is invalid SemVer: {error}"))?;
    if ReleaseStage::from_version(&candidate) != Ok(ReleaseStage::Alpha)
        || candidate.major != active.major
        || candidate.minor != active.minor
        || candidate.patch != active.patch
    {
        return Err("Alpha baseline tag differs from the active release line".to_owned());
    }
    if ReleaseStage::from_version(active) == Ok(ReleaseStage::Alpha) {
        let candidate_iteration = prerelease_iteration(&candidate, "alpha")?;
        let active_iteration = prerelease_iteration(active, "alpha")?;
        if candidate_iteration > active_iteration || active_iteration - candidate_iteration > 1 {
            return Err(
                "Alpha baseline must identify the current or immediately previous Alpha".to_owned(),
            );
        }
    }
    Ok(candidate)
}

fn release_notes_for_version(version: &Version) -> String {
    format!(
        r#"# CanISend {version}

## Highlights

CanISend 1.0 combines a macOS desktop interface, standalone command-line application, and versioned agent
integration in one Rust-native product. It installs without Python and does not require Python, Node.js, Java, a
separately installed SQLite library, or a Typst command.

The product provides local-first connected intake, independently Pack-bound Applications, explicit Evidence
associations, guarded planning and drafting, review, export, backup, recovery, and embedded PDF rendering. Codex,
Claude Code, and conforming MCP clients integrate through `canisend.agent/v4` and generated integrity-managed
Skills. CanISend prepares application materials but never submits an application.

## Compatibility

- This release line uses `canisend.workspace/v4`, `canisend.agent/v4`, and Agent schema major version 4.
- It does not migrate Python-era Workspaces, Workspace v2/v3, or preserve old Skills and Agent requests.
- Rust-native schema migrations are append-only. Unsupported or future authority is rejected before mutation.
- The macOS application bundles a version-matched CLI; standalone CLI archives cover the five declared targets.

## Install and verify

Download the archive for one supported target together with `SHA256SUMS`, the release manifest, notices, and
stage-required signing evidence. Verify their checksums, GitHub build provenance, manifest identity, and platform
signature before extracting the executable. Follow the
[native release verification guide](https://github.com/jxpeng98/CanISend/blob/main/docs/guides/release-verification.md)
and reject any incomplete or mismatched release unit.

Community builds use macOS ad-hoc signing and a Windows self-signed Authenticode certificate. These signatures
provide native integrity evidence but are not publicly trusted publisher identities; Gatekeeper, Unknown Publisher,
or SmartScreen warnings may still occur.

After extraction, run `canisend version --json`, `canisend doctor --json`, and the
[documented quick-start](https://github.com/jxpeng98/CanISend/blob/main/docs/guides/quick-start.md) before using private
application data.

## Upgrade and rollback

Check and back up every important workspace before replacing a binary. Retain the previous verified archive and its
notices. If the new binary opens a workspace, do not roll back by merely reinstalling the old executable: restore the
pre-upgrade backup into a new directory and check it with the old binary. There is no in-place database downgrade.
Follow the complete
[upgrade, rollback, and uninstall guide](https://github.com/jxpeng98/CanISend/blob/main/docs/guides/upgrade-and-rollback.md).

## Security and privacy

CanISend enables no telemetry, analytics, crash upload, or background reporting by default. User confirmation remains
authoritative for evidence, criteria, application decisions, review dispositions, exports, and final use. Provider
requests require explicit consent; portal login, upload, and submission are outside the product boundary.

## Known limitations

Read `KNOWN_LIMITATIONS.md` in the release assets before using real data. Text-based PDFs are supported; scanned or
image-only PDFs require external OCR and user review. User-authored Typst, external Typst packages/files, system or
user fonts, OCR, GUI automation, portal automation, and Linux arm64 archives are outside the 1.0 release scope.

## Feedback and support

Report reproducible problems through the repository issue templates. Include only sanitized public diagnostic
fields, exact release/target identity, and reproduction steps. Never attach a workspace, backup, application package,
private advert/profile content, provider request, token, certificate, or credential. The 1.0 line has no
service-level agreement or long-term-support commitment; consult the support policy shipped with the repository for
the current version window.
"#
    )
}

fn release_line_activation_report(
    root: &Path,
    activation: &RenderedReleaseLineActivation,
    write: bool,
) -> Result<Value, String> {
    let controlled_files = activation
        .files
        .iter()
        .map(|(relative, after)| {
            let path = root.join(relative);
            let before = if path.exists() {
                Some(sha256(&fs::read(&path).map_err(|error| {
                    format!("could not read `{relative}`: {error}")
                })?))
            } else {
                None
            };
            Ok(json!({
                "path": relative,
                "before_sha256": before,
                "after_sha256": sha256(after)
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!({
        "schema": RELEASE_LINE_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "from_version": activation.from_version.to_string(),
        "to_version": activation.to_version.to_string(),
        "history_line": activation.history_line,
        "activation_source_commit": activation.source_commit,
        "controlled_files": controlled_files,
        "external_actions": {
            "tag_created": false,
            "pushed": false,
            "release_published": false,
            "package_repository_modified": false
        },
        "next": "review and commit the activation, then run the locked release source gate"
    }))
}

fn digest_regular_file_tree(root: &Path) -> Result<String, String> {
    const MAX_FILES: usize = 256;
    const MAX_BYTES: u64 = 16 * 1024 * 1024;

    if !root.is_dir() {
        return Err(format!(
            "historical reference tree is missing: {}",
            root.display()
        ));
    }
    let mut pending = VecDeque::from([root.to_path_buf()]);
    let mut entries = Vec::new();
    let mut total_bytes = 0_u64;
    while let Some(directory) = pending.pop_front() {
        let mut children = fs::read_dir(&directory)
            .map_err(|error| format!("could not inspect {}: {error}", directory.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("could not inspect {}: {error}", directory.display()))?;
        children.sort_by_key(fs::DirEntry::file_name);
        for child in children {
            let path = child.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "historical reference tree contains a symlink: {}",
                    path.display()
                ));
            }
            if metadata.is_dir() {
                pending.push_back(path);
                continue;
            }
            if !metadata.is_file() {
                return Err(format!(
                    "historical reference tree contains a non-regular entry: {}",
                    path.display()
                ));
            }
            total_bytes = total_bytes
                .checked_add(metadata.len())
                .ok_or_else(|| "historical reference tree size overflowed".to_owned())?;
            if entries.len() >= MAX_FILES || total_bytes > MAX_BYTES {
                return Err(format!(
                    "historical reference tree exceeds {MAX_FILES} files or {MAX_BYTES} bytes"
                ));
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "historical reference path escaped its root".to_owned())?
                .to_str()
                .ok_or_else(|| "historical reference path is not UTF-8".to_owned())?
                .replace('\\', "/");
            entries.push((
                relative,
                fs::read(&path)
                    .map_err(|error| format!("could not read {}: {error}", path.display()))?,
            ));
        }
    }
    if entries.is_empty() {
        return Err("historical reference tree is empty".to_owned());
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(digest_named_bytes(&entries))
}

fn write_controlled_files_transactionally(
    root: &Path,
    files: &BTreeMap<String, Vec<u8>>,
    fail_after_replacements: Option<usize>,
) -> Result<(), String> {
    let mut staged = Vec::new();
    for (index, (relative, body)) in files.iter().enumerate() {
        let relative_path = Path::new(relative);
        if relative_path.is_absolute()
            || relative_path
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(format!(
                "controlled write path must be a normalized repository path: `{relative}`"
            ));
        }
        let target = root.join(relative_path);
        if target.exists() {
            reject_symlink(&target)?;
            if !target.is_file() {
                return Err(format!(
                    "controlled write target is not a regular file: {}",
                    target.display()
                ));
            }
        }
        let parent = target
            .parent()
            .ok_or_else(|| format!("controlled write target has no parent: {relative}"))?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        let name = target
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("controlled write target name is invalid: {relative}"))?;
        let temporary = parent.join(format!(
            ".{name}.canisend-transaction-{}-{index}",
            std::process::id()
        ));
        let mut output = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| {
                format!(
                    "could not stage controlled write {}: {error}",
                    temporary.display()
                )
            })?;
        if let Err(error) = output.write_all(body).and_then(|()| output.sync_all()) {
            let _ = fs::remove_file(&temporary);
            for (_, staged_path, _) in &staged {
                let _ = fs::remove_file(staged_path);
            }
            return Err(format!(
                "could not finish staged write {}: {error}",
                temporary.display()
            ));
        }
        let original = if target.exists() {
            Some(
                fs::read(&target)
                    .map_err(|error| format!("could not back up {}: {error}", target.display()))?,
            )
        } else {
            None
        };
        staged.push((target, temporary, original));
    }

    let mut applied = 0_usize;
    let mut failure = None;
    for (target, temporary, _) in &staged {
        if fail_after_replacements == Some(applied) {
            failure = Some("injected controlled-write failure".to_owned());
            break;
        }
        if let Err(error) = fs::rename(temporary, target) {
            failure = Some(format!(
                "could not replace controlled file {}: {error}",
                target.display()
            ));
            break;
        }
        applied += 1;
    }
    let Some(failure) = failure else {
        return Ok(());
    };

    let mut rollback_errors = Vec::new();
    for (index, (target, _, original)) in staged[..applied].iter().enumerate().rev() {
        if let Some(original) = original {
            let rollback =
                target.with_file_name(format!(".canisend-rollback-{}-{index}", std::process::id()));
            let rollback_result = (|| -> Result<(), std::io::Error> {
                let mut output = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&rollback)?;
                output.write_all(original)?;
                output.sync_all()?;
                fs::rename(&rollback, target)
            })();
            if let Err(error) = rollback_result {
                let _ = fs::remove_file(&rollback);
                rollback_errors.push(format!("{}: {error}", target.display()));
            }
        } else if let Err(error) = fs::remove_file(target) {
            rollback_errors.push(format!("{}: {error}", target.display()));
        }
    }
    for (_, temporary, _) in &staged[applied..] {
        let _ = fs::remove_file(temporary);
    }
    if rollback_errors.is_empty() {
        Err(format!("{failure}; all applied files were rolled back"))
    } else {
        Err(format!(
            "{failure}; rollback also failed for {}",
            rollback_errors.join(", ")
        ))
    }
}

fn activate_feature_freeze(baseline: &str, write: bool) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "feature-freeze activation")?;
    }
    let freeze = render_feature_freeze_activation(&root, baseline)?;
    let report = feature_freeze_report(&root, &freeze, write)?;
    if write {
        for (relative, body) in &freeze.files {
            let path = root.join(relative);
            fs::write(&path, body)
                .map_err(|error| format!("could not write {}: {error}", path.display()))?;
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("could not serialize feature-freeze plan: {error}"))?
    );
    Ok(())
}

fn record_beta_qualification(
    tag: &str,
    run_id: &str,
    assets: &Path,
    write: bool,
) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "Beta qualification")?;
    }
    let qualification = render_beta_qualification(&root, tag, run_id, assets)?;
    let ledger_path = root.join("release/qualification-ledger.json");
    let before = fs::read(&ledger_path)
        .map_err(|error| format!("could not read qualification ledger: {error}"))?;
    let report = json!({
        "schema": BETA_QUALIFICATION_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "tag": qualification.tag,
        "source_commit": qualification.source_commit,
        "signed_matrix_run": qualification.run_id,
        "release_manifest_sha256": qualification.manifest_sha256,
        "ledger": {
            "path": "release/qualification-ledger.json",
            "before_sha256": sha256(&before),
            "after_sha256": sha256(&qualification.ledger)
        },
        "next": "independently retain public attestation verification, commit the ledger, then activate the freeze"
    });
    if write {
        fs::write(&ledger_path, &qualification.ledger)
            .map_err(|error| format!("could not write {}: {error}", ledger_path.display()))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("could not serialize Beta qualification plan: {error}"))?
    );
    Ok(())
}

fn record_rc_qualification(
    tag: &str,
    run_id: &str,
    assets: &Path,
    write: bool,
) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "RC qualification")?;
    }
    let qualification = render_rc_qualification(&root, tag, run_id, assets)?;
    let ledger_path = root.join("release/qualification-ledger.json");
    let before = fs::read(&ledger_path)
        .map_err(|error| format!("could not read qualification ledger: {error}"))?;
    let report = json!({
        "schema": RC_QUALIFICATION_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "tag": qualification.tag,
        "source_commit": qualification.source_commit,
        "signed_matrix_run": qualification.run_id,
        "release_manifest_sha256": qualification.manifest_sha256,
        "ledger": {
            "path": "release/qualification-ledger.json",
            "before_sha256": sha256(&before),
            "after_sha256": sha256(&qualification.ledger)
        },
        "next": "commit this clean-tag matrix; record a distinct sequential RC before Stable"
    });
    if write {
        fs::write(&ledger_path, &qualification.ledger)
            .map_err(|error| format!("could not write {}: {error}", ledger_path.display()))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("could not serialize RC qualification plan: {error}"))?
    );
    Ok(())
}

fn record_release_notes_qualification(
    tag: &str,
    assets: &Path,
    reviewer: &str,
    write: bool,
) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "release-notes qualification")?;
    }
    validate_github_login(reviewer)?;
    let (version, stage) = parse_release_tag(tag)?;
    if stage != ReleaseStage::ReleaseCandidate {
        return Err("release-notes qualification requires an RC tag".to_owned());
    }
    let workspace_body = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    if workspace["workspace"]["package"]["version"].as_str() != Some(version.to_string().as_str()) {
        return Err("release-notes RC tag must match the current workspace version".to_owned());
    }

    check_release_notes_policy()?;
    verify_release(tag, assets)?;
    let manifest_path = assets.join(format!("canisend-{version}-manifest.json"));
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("could not read verified RC manifest: {error}"))?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("verified RC manifest is invalid JSON: {error}"))?;
    let source_commit = required_string(&manifest["source"], "commit", "RC release source")?;
    validate_lower_hex("RC release source commit", source_commit, 40)?;

    let notes_path = root.join("release/RELEASE_NOTES.md");
    let notes = fs::read(&notes_path)
        .map_err(|error| format!("could not read checked-in release notes: {error}"))?;
    let asset_notes = fs::read(assets.join("RELEASE_NOTES.md"))
        .map_err(|error| format!("could not read verified release-notes asset: {error}"))?;
    if notes != asset_notes {
        return Err(
            "checked-in release notes differ from the verified RC release asset".to_owned(),
        );
    }
    let notes_body_sha256 = release_notes_body_sha256(&notes)?;
    let rollback_sha256 = sha256_file(&root.join("docs/guides/upgrade-and-rollback.md"))?;

    let ledger_path = root.join("release/qualification-ledger.json");
    let before = fs::read(&ledger_path)
        .map_err(|error| format!("could not read qualification ledger: {error}"))?;
    let ledger: Value = serde_json::from_slice(&before)
        .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    let qualified = release_notes_qualified_ledger(
        &ledger,
        tag,
        source_commit,
        reviewer,
        &sha256(&manifest_bytes),
        &notes_body_sha256,
        &rollback_sha256,
    )?;
    let after = pretty_json_bytes(&qualified)?;
    let reviewed = &qualified["release_notes"]["review"];
    let report = json!({
        "schema": RELEASE_NOTES_QUALIFICATION_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "tag": tag,
        "reviewer": reviewer,
        "source_commit": source_commit,
        "signed_matrix_run": reviewed["signed_matrix_run"],
        "release_manifest_sha256": reviewed["release_manifest_sha256"],
        "release_notes_body_sha256": notes_body_sha256,
        "rollback_sha256": rollback_sha256,
        "ledger": {
            "path": "release/qualification-ledger.json",
            "before_sha256": sha256(&before),
            "after_sha256": sha256(&after)
        },
        "next": "independently inspect the final RC issue, asset, limitation, rollback, and package-channel state, then commit the ledger"
    });
    if write {
        fs::write(&ledger_path, after)
            .map_err(|error| format!("could not write {}: {error}", ledger_path.display()))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| {
            format!("could not serialize release-notes qualification plan: {error}")
        })?
    );
    Ok(())
}

fn release_notes_qualified_ledger(
    ledger: &Value,
    tag: &str,
    source_commit: &str,
    reviewer: &str,
    manifest_sha256: &str,
    notes_body_sha256: &str,
    rollback_sha256: &str,
) -> Result<Value, String> {
    let (_, stage) = parse_release_tag(tag)?;
    validate_github_login(reviewer)?;
    validate_lower_hex("RC source commit", source_commit, 40)?;
    validate_lower_hex("RC release manifest SHA-256", manifest_sha256, 64)?;
    validate_lower_hex("release-notes body SHA-256", notes_body_sha256, 64)?;
    validate_lower_hex("rollback guide SHA-256", rollback_sha256, 64)?;
    if stage != ReleaseStage::ReleaseCandidate
        || ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "rc"
        || ledger["status"] != "rc-qualifying"
        || ledger["feature_freeze"]["status"] != "frozen"
        || ledger["release_notes"]["status"] != "rc-final"
        || !ledger["release_notes"]["review"].is_null()
        || ledger["stable_authorized"] != false
    {
        return Err(
            "qualification ledger is not canonical pending RC notes-review state".to_owned(),
        );
    }
    let candidates = ledger["release_candidates"]
        .as_array()
        .filter(|candidates| !candidates.is_empty())
        .ok_or_else(|| "release-notes qualification requires a recorded RC matrix".to_owned())?;
    let candidate = candidates
        .last()
        .expect("non-empty release candidate array was checked");
    let (recorded_tag, recorded_source, recorded_run) =
        validate_qualification_release(candidate, ReleaseStage::ReleaseCandidate, "final RC")?;
    if recorded_tag != tag || recorded_source != source_commit {
        return Err(
            "release-notes review must bind the latest recorded RC tag and source".to_owned(),
        );
    }
    let evidence = vec![
        format!("{tag} release notes and rollback guidance reviewed by {reviewer}"),
        format!(
            "signed RC matrix run {recorded_run} manifest, public issues, assets, limitations, and package-channel state reviewed"
        ),
    ];
    let mut qualified = ledger.clone();
    qualified["release_notes"]["review"] = json!({
        "evidence": evidence,
        "release_manifest_sha256": manifest_sha256,
        "release_notes_body_sha256": notes_body_sha256,
        "reviewer": reviewer,
        "rollback_sha256": rollback_sha256,
        "signed_matrix_run": recorded_run,
        "source_commit": source_commit,
        "status": "reviewed",
        "tag": tag
    });
    Ok(qualified)
}

fn validate_github_login(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 39
        || value.starts_with('-')
        || value.ends_with('-')
        || value.contains("--")
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("release-notes reviewer must be a valid GitHub login".to_owned());
    }
    Ok(())
}

fn release_notes_body_sha256(notes: &[u8]) -> Result<String, String> {
    let notes = std::str::from_utf8(notes)
        .map_err(|_| "release notes must be UTF-8 before review".to_owned())?;
    let (_, body) = notes
        .split_once('\n')
        .ok_or_else(|| "release notes must contain a heading and body".to_owned())?;
    if body.trim().is_empty() {
        return Err("release notes body must not be empty".to_owned());
    }
    Ok(sha256(body.as_bytes()))
}

fn record_upgrade_qualification(
    from_tag: &str,
    to_tag: &str,
    evidence: &Path,
    write: bool,
) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "upgrade qualification")?;
    }
    let summary = verify_upgrade_qualification_evidence(from_tag, to_tag, evidence)?;
    let ledger_path = root.join("release/qualification-ledger.json");
    let before = fs::read(&ledger_path)
        .map_err(|error| format!("could not read qualification ledger: {error}"))?;
    let ledger: Value = serde_json::from_slice(&before)
        .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    let qualified = upgrade_qualified_ledger(&ledger, from_tag, to_tag, &summary)?;
    let after = pretty_json_bytes(&qualified)?;
    let report = json!({
        "schema": UPGRADE_QUALIFICATION_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "from_tag": from_tag,
        "to_tag": to_tag,
        "github_run_id": summary.run_id,
        "records": summary.records,
        "manifests": {
            "from_sha256": summary.from_manifest_sha256,
            "to_sha256": summary.to_manifest_sha256
        },
        "ledger": {
            "path": "release/qualification-ledger.json",
            "before_sha256": sha256(&before),
            "after_sha256": sha256(&after)
        },
        "next": "independently inspect the public run and attestations, then commit the qualification ledger"
    });
    if write {
        fs::write(&ledger_path, after)
            .map_err(|error| format!("could not write {}: {error}", ledger_path.display()))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("could not serialize upgrade qualification plan: {error}"))?
    );
    Ok(())
}

fn upgrade_qualified_ledger(
    ledger: &Value,
    from_tag: &str,
    to_tag: &str,
    summary: &UpgradeQualificationSummary,
) -> Result<Value, String> {
    let (_, from_stage) = parse_release_tag(from_tag)?;
    let (_, to_stage) = parse_release_tag(to_tag)?;
    let pending = json!({
        "beta_tag": null,
        "evidence": [],
        "rc_tag": null,
        "status": "pending"
    });
    if from_stage != ReleaseStage::Beta
        || to_stage != ReleaseStage::ReleaseCandidate
        || summary.run_id == 0
        || summary.records != 5
        || ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "rc"
        || ledger["status"] != "rc-qualifying"
        || ledger["beta"]["status"] != "qualified"
        || ledger["beta"]["tag"] != from_tag
        || ledger["feature_freeze"]["status"] != "frozen"
        || ledger["stable_authorized"] != false
        || ledger["upgrade_matrix"] != pending
    {
        return Err("qualification ledger is not canonical pending RC upgrade state".to_owned());
    }
    let has_rc = ledger["release_candidates"]
        .as_array()
        .is_some_and(|candidates| {
            candidates
                .iter()
                .any(|candidate| candidate["status"] == "success" && candidate["tag"] == to_tag)
        });
    if !has_rc {
        return Err(
            "upgrade qualification RC must already have a successful signed matrix".to_owned(),
        );
    }
    let mut qualified = ledger.clone();
    qualified["upgrade_matrix"] = json!({
        "beta_tag": from_tag,
        "evidence": [
            format!(
                "native upgrade qualification run {} passed five signed archive targets",
                summary.run_id
            ),
            format!(
                "{from_tag} to {to_tag} backup, old-binary, restore, host-pack, and uninstall lifecycle passed"
            )
        ],
        "rc_tag": to_tag,
        "status": "passed"
    });
    Ok(qualified)
}

fn record_documentation_uninstall_qualification(
    tag: &str,
    assets: &Path,
    evidence: &Path,
    write: bool,
) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "documentation/uninstall qualification")?;
    }
    let (version, stage) = parse_release_tag(tag)?;
    if stage != ReleaseStage::ReleaseCandidate {
        return Err("documentation/uninstall qualification requires an RC tag".to_owned());
    }
    let workspace_body = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    if workspace["workspace"]["package"]["version"].as_str() != Some(version.to_string().as_str()) {
        return Err(
            "documentation/uninstall tag must match the current workspace version".to_owned(),
        );
    }
    let summary = verify_documentation_uninstall_evidence(tag, assets, evidence)?;
    let ledger_path = root.join("release/qualification-ledger.json");
    let before = fs::read(&ledger_path)
        .map_err(|error| format!("could not read qualification ledger: {error}"))?;
    let ledger: Value = serde_json::from_slice(&before)
        .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    let qualified = documentation_uninstall_qualified_ledger(&ledger, tag, summary)?;
    let after = pretty_json_bytes(&qualified)?;
    let report = json!({
        "schema": DOCUMENTATION_UNINSTALL_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "tag": tag,
        "github_run_id": summary.run_id,
        "records": summary.records,
        "ledger": {
            "path": "release/qualification-ledger.json",
            "before_sha256": sha256(&before),
            "after_sha256": sha256(&after)
        },
        "next": "independently inspect the RC run and public asset attestations, then commit the qualification ledger"
    });
    if write {
        fs::write(&ledger_path, after)
            .map_err(|error| format!("could not write {}: {error}", ledger_path.display()))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| {
            format!("could not serialize documentation/uninstall qualification plan: {error}")
        })?
    );
    Ok(())
}

fn documentation_uninstall_qualified_ledger(
    ledger: &Value,
    tag: &str,
    summary: DocumentationUninstallSummary,
) -> Result<Value, String> {
    let (_, stage) = parse_release_tag(tag)?;
    validate_documentation_uninstall_progress(&ledger["documentation_uninstall"])?;
    if stage != ReleaseStage::ReleaseCandidate
        || summary.run_id == 0
        || summary.records != 5
        || ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "rc"
        || ledger["status"] != "rc-qualifying"
        || ledger["beta"]["status"] != "qualified"
        || ledger["feature_freeze"]["status"] != "frozen"
        || ledger["stable_authorized"] != false
        || ledger["documentation_uninstall"]["status"] == "passed"
    {
        return Err(
            "qualification ledger is not eligible for RC documentation/uninstall evidence"
                .to_owned(),
        );
    }
    let candidates = ledger["release_candidates"]
        .as_array()
        .ok_or_else(|| "qualification ledger release_candidates must be an array".to_owned())?;
    let matching = candidates
        .iter()
        .find(|candidate| candidate["tag"] == tag && candidate["status"] == "success");
    let Some(candidate) = matching else {
        return Err(
            "documentation/uninstall qualification tag has no recorded RC matrix".to_owned(),
        );
    };
    let (_, _, recorded_run) =
        validate_qualification_release(candidate, ReleaseStage::ReleaseCandidate, "RC")?;
    if recorded_run != summary.run_id {
        return Err(
            "documentation/uninstall evidence must come from the same recorded RC matrix run"
                .to_owned(),
        );
    }
    let mut qualified = ledger.clone();
    qualified["documentation_uninstall"] = json!({
        "evidence": [
            format!(
                "native RC run {} passed exact archive documentation and uninstall smoke on five targets",
                summary.run_id
            ),
            format!(
                "{tag} retained external workspaces after installed binary and notice removal"
            )
        ],
        "native_matrix_run": summary.run_id,
        "status": "passed"
    });
    Ok(qualified)
}

fn record_package_manager_qualification(
    from_tag: &str,
    to_tag: &str,
    evidence: &Path,
    write: bool,
) -> Result<(), String> {
    let root = repository_root();
    if write {
        require_clean_worktree(&root, "package-manager qualification")?;
    }
    let (to_version, to_stage) = parse_release_tag(to_tag)?;
    if to_stage != ReleaseStage::ReleaseCandidate {
        return Err("package-manager qualification target must be an RC tag".to_owned());
    }
    let workspace_body = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    if workspace["workspace"]["package"]["version"].as_str()
        != Some(to_version.to_string().as_str())
    {
        return Err("package-manager RC tag must match the current workspace version".to_owned());
    }
    let summary = verify_package_manager_evidence(from_tag, to_tag, evidence)?;
    let ledger_path = root.join("release/qualification-ledger.json");
    let before = fs::read(&ledger_path)
        .map_err(|error| format!("could not read qualification ledger: {error}"))?;
    let ledger: Value = serde_json::from_slice(&before)
        .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    let qualified = package_manager_qualified_ledger(&ledger, from_tag, to_tag, summary)?;
    let after = pretty_json_bytes(&qualified)?;
    let report = json!({
        "schema": PACKAGE_MANAGER_QUALIFICATION_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "from_tag": from_tag,
        "to_tag": to_tag,
        "github_run_id": summary.run_id,
        "records": summary.records,
        "ledger": {
            "path": "release/qualification-ledger.json",
            "before_sha256": sha256(&before),
            "after_sha256": sha256(&after)
        },
        "next": "independently inspect Homebrew, Scoop, and fresh WinGet Sandbox evidence, then commit the ledger"
    });
    if write {
        fs::write(&ledger_path, after)
            .map_err(|error| format!("could not write {}: {error}", ledger_path.display()))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| {
            format!("could not serialize package-manager qualification plan: {error}")
        })?
    );
    Ok(())
}

fn package_manager_qualified_ledger(
    ledger: &Value,
    from_tag: &str,
    to_tag: &str,
    summary: PackageManagerQualificationSummary,
) -> Result<Value, String> {
    let (_, from_stage) = parse_release_tag(from_tag)?;
    let (_, to_stage) = parse_release_tag(to_tag)?;
    let pending = json!({
        "channels": ["homebrew-cask", "scoop", "winget"],
        "evidence": [],
        "status": "candidates-only"
    });
    if from_stage != ReleaseStage::Beta
        || to_stage != ReleaseStage::ReleaseCandidate
        || summary.run_id == 0
        || summary.records != 4
        || ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "rc"
        || ledger["status"] != "rc-qualifying"
        || ledger["beta"]["status"] != "qualified"
        || ledger["beta"]["tag"] != from_tag
        || ledger["feature_freeze"]["status"] != "frozen"
        || ledger["stable_authorized"] != false
        || ledger["package_managers"] != pending
    {
        return Err(
            "qualification ledger is not canonical pending package-manager state".to_owned(),
        );
    }
    let has_rc = ledger["release_candidates"]
        .as_array()
        .is_some_and(|candidates| {
            candidates
                .iter()
                .any(|candidate| candidate["tag"] == to_tag && candidate["status"] == "success")
        });
    if !has_rc {
        return Err("package-manager qualification RC has no successful signed matrix".to_owned());
    }
    let mut qualified = ledger.clone();
    qualified["package_managers"] = json!({
        "channels": ["homebrew-cask", "scoop", "winget"],
        "evidence": [
            format!(
                "package-manager qualification run {} passed Homebrew arm64/Intel, Scoop, and WinGet records",
                summary.run_id
            ),
            format!(
                "{from_tag} to {to_tag} install, version, doctor, workspace, upgrade, uninstall, and retention passed"
            )
        ],
        "qualification": {
            "beta_tag": from_tag,
            "rc_tag": to_tag,
            "records": summary.records,
            "run_id": summary.run_id
        },
        "status": "passed"
    });
    Ok(qualified)
}

fn validate_package_manager_qualification_record(
    package_managers: &Value,
    beta: &Value,
    candidates: &[Value],
) -> Result<(String, String, u64), String> {
    if package_managers["status"] != "passed" {
        return Err("package-manager qualification status must be `passed`".to_owned());
    }
    let qualification = &package_managers["qualification"];
    let beta_tag = required_string(qualification, "beta_tag", "package-manager qualification")?;
    let rc_tag = required_string(qualification, "rc_tag", "package-manager qualification")?;
    let run_id = qualification["run_id"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| "package-manager qualification has no run ID".to_owned())?;
    if qualification["records"] != 4 {
        return Err("package-manager qualification must bind four native records".to_owned());
    }
    let (_, beta_stage) = parse_release_tag(beta_tag)?;
    let (_, rc_stage) = parse_release_tag(rc_tag)?;
    if beta_stage != ReleaseStage::Beta
        || rc_stage != ReleaseStage::ReleaseCandidate
        || beta["status"] != "qualified"
        || beta["tag"] != beta_tag
    {
        return Err("package-manager qualification release pair is invalid".to_owned());
    }
    let has_rc = candidates
        .iter()
        .any(|candidate| candidate["tag"] == rc_tag && candidate["status"] == "success");
    if !has_rc {
        return Err(
            "package-manager qualification does not bind a successful RC matrix".to_owned(),
        );
    }
    let canonical = json!({
        "channels": ["homebrew-cask", "scoop", "winget"],
        "evidence": [
            format!(
                "package-manager qualification run {run_id} passed Homebrew arm64/Intel, Scoop, and WinGet records"
            ),
            format!(
                "{beta_tag} to {rc_tag} install, version, doctor, workspace, upgrade, uninstall, and retention passed"
            )
        ],
        "qualification": {
            "beta_tag": beta_tag,
            "rc_tag": rc_tag,
            "records": 4,
            "run_id": run_id
        },
        "status": "passed"
    });
    if *package_managers != canonical {
        return Err(
            "package-manager qualification contains unknown or non-canonical fields".to_owned(),
        );
    }
    Ok((beta_tag.to_owned(), rc_tag.to_owned(), run_id))
}

fn render_beta_qualification(
    root: &Path,
    tag: &str,
    run_id: &str,
    assets: &Path,
) -> Result<RenderedReleaseQualification, String> {
    let run_id = run_id
        .parse::<u64>()
        .ok()
        .filter(|run| *run > 0)
        .ok_or_else(|| "Beta qualification run ID must be a positive integer".to_owned())?;
    let (version, stage) = parse_release_tag(tag)?;
    if stage != ReleaseStage::Beta {
        return Err("Beta qualification requires a Beta tag".to_owned());
    }
    let workspace_body = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    if workspace["workspace"]["package"]["version"].as_str() != Some(version.to_string().as_str()) {
        return Err("Beta qualification tag must match the current workspace version".to_owned());
    }

    verify_release(tag, assets)?;
    let manifest_path = assets.join(format!("canisend-{version}-manifest.json"));
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("could not read verified release manifest: {error}"))?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("verified release manifest is invalid JSON: {error}"))?;
    let source_commit = required_string(&manifest["source"], "commit", "Beta release source")?;
    validate_lower_hex("Beta release source commit", source_commit, 40)?;

    let ledger_path = root.join("release/qualification-ledger.json");
    let ledger: Value = serde_json::from_slice(
        &fs::read(&ledger_path)
            .map_err(|error| format!("could not read qualification ledger: {error}"))?,
    )
    .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    let ledger = beta_qualified_ledger(&ledger, tag, run_id, source_commit)?;
    Ok(RenderedReleaseQualification {
        tag: tag.to_owned(),
        run_id,
        source_commit: source_commit.to_owned(),
        manifest_sha256: sha256(&manifest_bytes),
        ledger: pretty_json_bytes(&ledger)?,
    })
}

fn render_rc_qualification(
    root: &Path,
    tag: &str,
    run_id: &str,
    assets: &Path,
) -> Result<RenderedReleaseQualification, String> {
    let run_id = run_id
        .parse::<u64>()
        .ok()
        .filter(|run| *run > 0)
        .ok_or_else(|| "RC qualification run ID must be a positive integer".to_owned())?;
    let (version, stage) = parse_release_tag(tag)?;
    if stage != ReleaseStage::ReleaseCandidate {
        return Err("RC qualification requires an RC tag".to_owned());
    }
    let workspace_body = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    if workspace["workspace"]["package"]["version"].as_str() != Some(version.to_string().as_str()) {
        return Err("RC qualification tag must match the current workspace version".to_owned());
    }

    verify_release(tag, assets)?;
    let manifest_path = assets.join(format!("canisend-{version}-manifest.json"));
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("could not read verified release manifest: {error}"))?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("verified release manifest is invalid JSON: {error}"))?;
    let source_commit = required_string(&manifest["source"], "commit", "RC release source")?;
    validate_lower_hex("RC release source commit", source_commit, 40)?;

    let ledger_path = root.join("release/qualification-ledger.json");
    let ledger: Value = serde_json::from_slice(
        &fs::read(&ledger_path)
            .map_err(|error| format!("could not read qualification ledger: {error}"))?,
    )
    .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    let ledger = rc_qualified_ledger(&ledger, tag, run_id, source_commit)?;
    Ok(RenderedReleaseQualification {
        tag: tag.to_owned(),
        run_id,
        source_commit: source_commit.to_owned(),
        manifest_sha256: sha256(&manifest_bytes),
        ledger: pretty_json_bytes(&ledger)?,
    })
}

fn beta_qualified_ledger(
    ledger: &Value,
    tag: &str,
    run_id: u64,
    source_commit: &str,
) -> Result<Value, String> {
    let (_, stage) = parse_release_tag(tag)?;
    validate_lower_hex("Beta source commit", source_commit, 40)?;
    let pending = json!({
        "signed_matrix_run": null,
        "signing_evidence_targets": [],
        "source_commit": null,
        "status": "pending",
        "tag": null
    });
    if stage != ReleaseStage::Beta
        || run_id == 0
        || ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "beta"
        || ledger["status"] != "beta-qualifying"
        || ledger["beta"] != pending
        || ledger["feature_freeze"]["status"] != "planned"
        || !ledger["feature_freeze"]["baseline_commit"].is_null()
        || ledger["stable_authorized"] != false
    {
        return Err("qualification ledger is not canonical pending Beta state".to_owned());
    }
    let mut qualified = ledger.clone();
    qualified["beta"] = json!({
        "signed_matrix_run": run_id,
        "signing_evidence_targets": [
            "aarch64-apple-darwin",
            "x86_64-apple-darwin",
            "x86_64-pc-windows-msvc"
        ],
        "source_commit": source_commit,
        "status": "qualified",
        "tag": tag
    });
    Ok(qualified)
}

fn rc_qualified_ledger(
    ledger: &Value,
    tag: &str,
    run_id: u64,
    source_commit: &str,
) -> Result<Value, String> {
    let (_, stage) = parse_release_tag(tag)?;
    validate_lower_hex("RC source commit", source_commit, 40)?;
    let baseline = required_string(
        &ledger["feature_freeze"],
        "baseline_commit",
        "feature freeze",
    )?;
    validate_lower_hex("feature-freeze baseline commit", baseline, 40)?;
    if stage != ReleaseStage::ReleaseCandidate
        || run_id == 0
        || ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "rc"
        || ledger["status"] != "rc-qualifying"
        || ledger["beta"]["status"] != "qualified"
        || ledger["feature_freeze"]["status"] != "frozen"
        || ledger["stable_authorized"] != false
    {
        return Err("qualification ledger is not canonical RC-qualifying state".to_owned());
    }
    let candidates = ledger["release_candidates"]
        .as_array()
        .ok_or_else(|| "qualification ledger release_candidates must be an array".to_owned())?;
    for candidate in candidates {
        if candidate["status"] != "success" {
            return Err("existing RC qualification is not successful".to_owned());
        }
        let (existing_tag, existing_commit, existing_run) =
            validate_qualification_release(candidate, ReleaseStage::ReleaseCandidate, "RC")?;
        if existing_tag == tag || existing_commit == source_commit || existing_run == run_id {
            return Err(
                "RC qualification tag, source commit, and run ID must all be distinct".to_owned(),
            );
        }
    }
    let mut qualified = ledger.clone();
    qualified["release_candidates"]
        .as_array_mut()
        .expect("validated RC candidate array")
        .push(json!({
            "signed_matrix_run": run_id,
            "source_commit": source_commit,
            "status": "success",
            "tag": tag
        }));
    Ok(qualified)
}

fn require_clean_worktree(root: &Path, context: &str) -> Result<(), String> {
    let changes = run_git_lines(root, &["status", "--porcelain", "--untracked-files=all"])?;
    if changes.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{context} write requires a clean worktree; commit or stash owned changes first"
        ))
    }
}

fn render_feature_freeze_activation(
    root: &Path,
    baseline: &str,
) -> Result<RenderedFeatureFreeze, String> {
    validate_lower_hex("feature-freeze baseline commit", baseline, 40)?;
    run_git(root, &["cat-file", "-e", &format!("{baseline}^{{commit}}")])?;
    let head = run_git_lines(root, &["rev-parse", "HEAD"])?
        .into_iter()
        .next()
        .ok_or_else(|| "could not resolve HEAD for feature-freeze activation".to_owned())?;
    if baseline != head {
        return Err(format!(
            "feature-freeze activation baseline must equal current HEAD `{head}`"
        ));
    }

    let ledger_path = root.join("release/qualification-ledger.json");
    let mut ledger: Value = serde_json::from_slice(
        &fs::read(&ledger_path)
            .map_err(|error| format!("could not read qualification ledger: {error}"))?,
    )
    .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    if ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "beta"
        || ledger["status"] != "beta-qualifying"
        || ledger["beta"]["status"] != "qualified"
        || ledger["stable_authorized"] != false
    {
        return Err(
            "feature-freeze activation requires a qualified signed Beta workspace".to_owned(),
        );
    }
    if ledger["feature_freeze"]["status"] != "planned"
        || !ledger["feature_freeze"]["baseline_commit"].is_null()
    {
        return Err("feature freeze is not in canonical planned state".to_owned());
    }

    let exceptions_path = root.join("release/feature-freeze-exceptions.json");
    let mut exceptions: Value = serde_json::from_slice(
        &fs::read(&exceptions_path)
            .map_err(|error| format!("could not read feature-freeze exception record: {error}"))?,
    )
    .map_err(|error| format!("feature-freeze exception record is invalid JSON: {error}"))?;
    let planned = json!({
        "schema": FEATURE_FREEZE_EXCEPTIONS_SCHEMA,
        "status": "planned",
        "baseline_commit": null,
        "exceptions": []
    });
    if exceptions != planned {
        return Err("feature-freeze exception record is not canonical planned state".to_owned());
    }

    ledger["feature_freeze"]["status"] = Value::String("frozen".to_owned());
    ledger["feature_freeze"]["baseline_commit"] = Value::String(baseline.to_owned());
    exceptions["status"] = Value::String("frozen".to_owned());
    exceptions["baseline_commit"] = Value::String(baseline.to_owned());
    let files = BTreeMap::from([
        (
            "release/feature-freeze-exceptions.json".to_owned(),
            pretty_json_bytes(&exceptions)?,
        ),
        (
            "release/qualification-ledger.json".to_owned(),
            pretty_json_bytes(&ledger)?,
        ),
    ]);
    Ok(RenderedFeatureFreeze {
        baseline: baseline.to_owned(),
        files,
    })
}

fn feature_freeze_report(
    root: &Path,
    freeze: &RenderedFeatureFreeze,
    write: bool,
) -> Result<Value, String> {
    let files = freeze
        .files
        .iter()
        .map(|(relative, after)| {
            let before = fs::read(root.join(relative))
                .map_err(|error| format!("could not read {relative}: {error}"))?;
            Ok(json!({
                "path": relative,
                "before_sha256": sha256(&before),
                "after_sha256": sha256(after)
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!({
        "schema": FEATURE_FREEZE_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "baseline_commit": freeze.baseline,
        "files": files,
        "next": "commit the two automatic release-state files, then run release check"
    }))
}

fn check_beta_readiness_freshness(root: &Path, now: OffsetDateTime) -> Result<(), String> {
    let path = root.join("release/beta-readiness.json");
    let readiness: Value =
        serde_json::from_slice(&fs::read(&path).map_err(|error| {
            format!("could not read Beta readiness for stage transition: {error}")
        })?)
        .map_err(|error| format!("Beta readiness is invalid JSON: {error}"))?;
    let audited_at = required_string(&readiness, "audited_at", "Beta readiness")?;
    let audited_at = OffsetDateTime::parse(audited_at, &Rfc3339)
        .map_err(|error| format!("Beta readiness audit timestamp is invalid: {error}"))?;
    if audited_at > now + Duration::minutes(5) {
        return Err("Beta readiness audit timestamp is unreasonably in the future".to_owned());
    }
    let age = now - audited_at;
    if age > Duration::hours(BETA_READINESS_MAX_AGE_HOURS) {
        return Err(format!(
            "Beta readiness audit is older than {BETA_READINESS_MAX_AGE_HOURS} hours; refresh it before --write"
        ));
    }
    Ok(())
}

fn render_stage_transition(root: &Path, tag: &str) -> Result<RenderedStageTransition, String> {
    let workspace_path = root.join("Cargo.toml");
    let workspace_body = fs::read_to_string(&workspace_path)
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    let from_version = Version::parse(
        workspace["workspace"]["package"]["version"]
            .as_str()
            .ok_or_else(|| "workspace manifest has no package version".to_owned())?,
    )
    .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let from_stage = ReleaseStage::from_version(&from_version)?;
    let (to_version, to_stage) = parse_release_tag(tag)?;
    validate_stage_transition(&from_version, from_stage, &to_version, to_stage)?;
    if matches!(
        (from_stage, to_stage),
        (ReleaseStage::Alpha, ReleaseStage::Beta)
    ) {
        check_beta_transition_authorities(root, &from_version)?;
    }

    let members = workspace["workspace"]["members"]
        .as_array()
        .ok_or_else(|| "workspace manifest has no members array".to_owned())?
        .iter()
        .map(|member| {
            member
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "workspace member must be a string".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut package_names = BTreeSet::new();
    for member in &members {
        let body = fs::read_to_string(root.join(member).join("Cargo.toml"))
            .map_err(|error| format!("could not read {member}/Cargo.toml: {error}"))?;
        let manifest: toml::Value = body
            .parse()
            .map_err(|error| format!("{member}/Cargo.toml is invalid TOML: {error}"))?;
        package_names.insert(
            manifest["package"]["name"]
                .as_str()
                .ok_or_else(|| format!("{member}/Cargo.toml has no package name"))?
                .to_owned(),
        );
    }

    let from = from_version.to_string();
    let to = to_version.to_string();
    let mut files = BTreeMap::new();
    let workspace_after = replace_exact_count(
        &workspace_body,
        &format!("version = \"{from}\""),
        &format!("version = \"{to}\""),
        1,
        "workspace version",
    )?;
    files.insert("Cargo.toml".to_owned(), workspace_after.into_bytes());

    for member in &members {
        let relative = format!("{member}/Cargo.toml");
        let body = fs::read_to_string(root.join(&relative))
            .map_err(|error| format!("could not read {relative}: {error}"))?;
        let needle = format!("version = \"={from}\"");
        let occurrences = body.matches(&needle).count();
        if occurrences > 0 {
            let updated = replace_exact_count(
                &body,
                &needle,
                &format!("version = \"={to}\""),
                occurrences,
                &format!("internal dependency versions in {relative}"),
            )?;
            files.insert(relative, updated.into_bytes());
        }
    }

    let fuzz_relative = "fuzz/Cargo.toml";
    let fuzz_path = root.join(fuzz_relative);
    if fuzz_path.is_file() {
        let body = fs::read_to_string(&fuzz_path)
            .map_err(|error| format!("could not read {fuzz_relative}: {error}"))?;
        let needle = format!("version = \"={from}\"");
        let occurrences = body.matches(&needle).count();
        if occurrences == 0 {
            return Err(format!(
                "{fuzz_relative} has no internal dependencies pinned to {from}"
            ));
        }
        let updated = replace_exact_count(
            &body,
            &needle,
            &format!("version = \"={to}\""),
            occurrences,
            "fuzz manifest internal dependency versions",
        )?;
        files.insert(fuzz_relative.to_owned(), updated.into_bytes());
    }

    let lock_path = root.join("Cargo.lock");
    let mut lock = fs::read_to_string(&lock_path)
        .map_err(|error| format!("could not read Cargo.lock: {error}"))?;
    for package in &package_names {
        lock = replace_exact_count(
            &lock,
            &format!("name = \"{package}\"\nversion = \"{from}\""),
            &format!("name = \"{package}\"\nversion = \"{to}\""),
            1,
            &format!("Cargo.lock package `{package}`"),
        )?;
    }
    files.insert("Cargo.lock".to_owned(), lock.into_bytes());

    let fuzz_lock_relative = "fuzz/Cargo.lock";
    let fuzz_lock_path = root.join(fuzz_lock_relative);
    if fuzz_lock_path.is_file() {
        let mut fuzz_lock = fs::read_to_string(&fuzz_lock_path)
            .map_err(|error| format!("could not read {fuzz_lock_relative}: {error}"))?;
        let parsed: toml::Value = fuzz_lock
            .parse()
            .map_err(|error| format!("{fuzz_lock_relative} is invalid TOML: {error}"))?;
        let packages = parsed["package"]
            .as_array()
            .ok_or_else(|| format!("{fuzz_lock_relative} has no package entries"))?;
        let mut fuzz_package_names = BTreeSet::new();
        for package in packages {
            let Some(name) = package["name"].as_str() else {
                continue;
            };
            if !name.starts_with("canisend-") || name == "canisend-fuzz" {
                continue;
            }
            let version = package["version"]
                .as_str()
                .ok_or_else(|| format!("{fuzz_lock_relative} package `{name}` has no version"))?;
            if version != from {
                return Err(format!(
                    "{fuzz_lock_relative} package `{name}` uses {version}, expected {from}"
                ));
            }
            fuzz_package_names.insert(name.to_owned());
        }
        if fuzz_package_names.is_empty() {
            return Err(format!(
                "{fuzz_lock_relative} has no internal CanISend packages"
            ));
        }
        for package in fuzz_package_names {
            fuzz_lock = replace_exact_count(
                &fuzz_lock,
                &format!("name = \"{package}\"\nversion = \"{from}\""),
                &format!("name = \"{package}\"\nversion = \"{to}\""),
                1,
                &format!("{fuzz_lock_relative} package `{package}`"),
            )?;
        }
        files.insert(fuzz_lock_relative.to_owned(), fuzz_lock.into_bytes());
    }

    insert_desktop_version_updates(root, &mut files, &from_version, &to_version)?;
    if matches!(
        (from_stage, to_stage),
        (ReleaseStage::Alpha, ReleaseStage::Alpha)
    ) {
        insert_sequential_alpha_updates(root, &mut files, &from_version, &to_version)?;
    }

    let ledger_path = root.join("release/qualification-ledger.json");
    let mut ledger: Value = serde_json::from_slice(
        &fs::read(&ledger_path)
            .map_err(|error| format!("could not read qualification ledger: {error}"))?,
    )
    .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    validate_transition_ledger_preconditions(&ledger, &from_version, from_stage, to_stage)?;
    ledger["workspace_stage"] = Value::String(to_stage.as_str().to_owned());
    ledger["status"] = Value::String(qualification_status_for_stage(to_stage).to_owned());
    ledger["release_notes"]["status"] =
        Value::String(release_notes_status_for_stage(to_stage).to_owned());
    if matches!(
        (from_stage, to_stage),
        (
            ReleaseStage::ReleaseCandidate,
            ReleaseStage::ReleaseCandidate
        )
    ) && !ledger["release_notes"]["review"].is_null()
    {
        ledger["release_notes"]["review"] = Value::Null;
    }
    if matches!(to_stage, ReleaseStage::Stable) {
        ledger["stable_authorized"] = Value::Bool(true);
    }
    let ledger_bytes = pretty_json_bytes(&ledger)?;
    if fs::read(&ledger_path)
        .map_err(|error| format!("could not reread qualification ledger: {error}"))?
        != ledger_bytes
    {
        files.insert("release/qualification-ledger.json".to_owned(), ledger_bytes);
    }

    let notes_path = root.join("release/RELEASE_NOTES.md");
    let notes = fs::read_to_string(&notes_path)
        .map_err(|error| format!("could not read release notes: {error}"))?;
    files.insert(
        "release/RELEASE_NOTES.md".to_owned(),
        replace_exact_count(
            &notes,
            &format!("# CanISend {from}"),
            &format!("# CanISend {to}"),
            1,
            "release-note version heading",
        )?
        .into_bytes(),
    );

    if matches!(to_stage, ReleaseStage::Stable) {
        let feedback_path = root.join("release/feedback-snapshot.json");
        let feedback_body = fs::read_to_string(&feedback_path)
            .map_err(|error| format!("could not read release feedback: {error}"))?;
        let feedback: Value = serde_json::from_str(&feedback_body)
            .map_err(|error| format!("release feedback is invalid JSON: {error}"))?;
        if feedback["schema"] != FEEDBACK_SNAPSHOT_SCHEMA
            || feedback["snapshot_stage"] != "rc"
            || feedback["next_roadmap"]["status"] != "reviewed"
        {
            return Err(
                "Stable transition requires reviewed RC feedback and next-roadmap state".to_owned(),
            );
        }
        check_final_rc_feedback_binding(root, &feedback)?;
        files.insert(
            "release/feedback-snapshot.json".to_owned(),
            replace_exact_count(
                &feedback_body,
                "\"status\": \"reviewed\"",
                "\"status\": \"published\"",
                1,
                "next-roadmap feedback publication marker",
            )?
            .into_bytes(),
        );

        let roadmap_relative = feedback_roadmap_relative(&feedback)?;
        let roadmap = fs::read_to_string(root.join(&roadmap_relative))
            .map_err(|error| format!("could not read next roadmap: {error}"))?;
        files.insert(
            roadmap_relative.clone(),
            replace_exact_count(
                &roadmap,
                "**Status:** Reviewed",
                "**Status:** Published",
                1,
                "next-roadmap publication marker",
            )?
            .into_bytes(),
        );

        let support_path = root.join("release/support-policy.json");
        let mut support: Value = serde_json::from_slice(
            &fs::read(&support_path)
                .map_err(|error| format!("could not read support policy: {error}"))?,
        )
        .map_err(|error| format!("support policy is invalid JSON: {error}"))?;
        if support["publication_status"] != "pre-stable-draft" {
            return Err("Stable transition requires a pre-stable support-policy draft".to_owned());
        }
        support["publication_status"] = Value::String("published".to_owned());
        files.insert(
            "release/support-policy.json".to_owned(),
            pretty_json_bytes(&support)?,
        );
    }

    for (relative, body) in &files {
        let current = fs::read(root.join(relative))
            .map_err(|error| format!("could not read {relative}: {error}"))?;
        if current == *body {
            return Err(format!(
                "stage transition would not change controlled file `{relative}`"
            ));
        }
    }
    Ok(RenderedStageTransition {
        from_version,
        to_version,
        from_stage,
        to_stage,
        files,
    })
}

fn insert_sequential_alpha_updates(
    root: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
    from_version: &Version,
    to_version: &Version,
) -> Result<(), String> {
    let from = from_version.to_string();
    let to = to_version.to_string();
    let from_tag = format!("v{from}");
    let to_tag = format!("v{to}");

    for (relative, before, after, context) in [
        (
            "tools/native-preview/package.json",
            format!("\"version\": \"{from}\""),
            format!("\"version\": \"{to}\""),
            "native-preview package version",
        ),
        (
            "apps/canisend-desktop/src/App.svelte",
            format!("product?.version ?? \"{from}\""),
            format!("product?.version ?? \"{to}\""),
            "desktop fallback version",
        ),
        (
            "docs/contracts/cli-gui-parity-v1.json",
            format!("\"version\": \"{from}\""),
            format!("\"version\": \"{to}\""),
            "CLI/GUI parity Alpha scope",
        ),
        (
            "docs/performance/macos-gui-alpha-baseline.json",
            format!("\"version\": \"{from}\""),
            format!("\"version\": \"{to}\""),
            "macOS GUI Alpha performance baseline version",
        ),
        (
            "README.md",
            format!("The checked-in source version is `{from}`"),
            format!("The checked-in source version is `{to}`"),
            "README source version",
        ),
        (
            "RELEASE.md",
            format!("Checked-in source: `{from}`"),
            format!("Checked-in source: `{to}`"),
            "root release guide source version",
        ),
        (
            ".github/ISSUE_TEMPLATE/bug.yml",
            format!("placeholder: {from}"),
            format!("placeholder: {to}"),
            "bug Issue template version",
        ),
    ] {
        let body = fs::read_to_string(root.join(relative))
            .map_err(|error| format!("could not read {relative}: {error}"))?;
        files.insert(
            relative.to_owned(),
            replace_exact_count(&body, &before, &after, 1, context)?.into_bytes(),
        );
    }

    let release_workflow_relative = ".github/workflows/release.yml";
    let release_workflow = fs::read_to_string(root.join(release_workflow_relative))
        .map_err(|error| format!("could not read {release_workflow_relative}: {error}"))?;
    let from_default = format!("default: \"{from_tag}\"");
    let to_default = format!("default: \"{to_tag}\"");
    if release_workflow.contains(&from_default) {
        files.insert(
            release_workflow_relative.to_owned(),
            replace_exact_count(
                &release_workflow,
                &from_default,
                &to_default,
                1,
                "release workflow default",
            )?
            .into_bytes(),
        );
    } else if release_workflow.matches(&to_default).count() != 1 {
        return Err(format!(
            "release workflow must default to either the source candidate `{from_tag}` or target candidate `{to_tag}`"
        ));
    }

    let limitations_relative = "docs/guides/known-limitations.md";
    let limitations = fs::read_to_string(root.join(limitations_relative))
        .map_err(|error| format!("could not read {limitations_relative}: {error}"))?;
    let limitations = replace_exact_count(
        &limitations,
        &format!("It applies to the `{from}` development line"),
        &format!("It applies to the `{to}` development line"),
        1,
        "known-limitations development version",
    )?;
    let limitations = replace_exact_count(
        &limitations,
        &format!("source version still says `{from}`"),
        &format!("source version still says `{to}`"),
        1,
        "known-limitations source version",
    )?;
    files.insert(limitations_relative.to_owned(), limitations.into_bytes());

    let contract_relative = "release/alpha-package-contract.json";
    let contract_body = fs::read_to_string(root.join(contract_relative))
        .map_err(|error| format!("could not read {contract_relative}: {error}"))?;
    let contract: Value = serde_json::from_str(&contract_body)
        .map_err(|error| format!("Alpha package contract is invalid JSON: {error}"))?;
    let expected_contract_schema = alpha_package_contract_schema(from_version)?;
    if contract["schema"] != expected_contract_schema
        || contract["version"] != from
        || contract["tag"] != from_tag
    {
        return Err(
            "Alpha package contract does not match the sequential-Alpha source version".to_owned(),
        );
    }
    let occurrence_count = contract_body.matches(&from).count();
    if occurrence_count < 3 {
        return Err(
            "Alpha package contract has no complete versioned asset inventory to update".to_owned(),
        );
    }
    let contract_after = contract_body.replace(&from, &to);
    if contract_after.contains(&from) {
        return Err("Alpha package contract retained the previous source version".to_owned());
    }
    let updated_contract: Value = serde_json::from_str(&contract_after)
        .map_err(|error| format!("updated Alpha package contract is invalid JSON: {error}"))?;
    if updated_contract["version"] != to || updated_contract["tag"] != to_tag {
        return Err("updated Alpha package contract does not bind the target version".to_owned());
    }
    files.insert(contract_relative.to_owned(), contract_after.into_bytes());

    files.insert(
        "release/beta-readiness.json".to_owned(),
        pretty_json_bytes(&pending_beta_readiness(to_version)?)?,
    );
    files.insert(
        "release/beta-contract-freeze.json".to_owned(),
        pretty_json_bytes(&pending_beta_contract_freeze(to_version)?)?,
    );
    files.insert(
        "release/feedback-snapshot.json".to_owned(),
        pretty_json_bytes(&pending_release_feedback(to_version)?)?,
    );
    Ok(())
}

fn alpha_package_contract_schema(version: &Version) -> Result<&'static str, String> {
    let first_v3 = Version::parse(FIRST_ALPHA_PACKAGE_V3_VERSION)
        .map_err(|error| format!("invalid first Alpha package v3 version: {error}"))?;
    Ok(if version < &first_v3 {
        ALPHA_PACKAGE_CONTRACT_V2_SCHEMA
    } else {
        ALPHA_PACKAGE_CONTRACT_V3_SCHEMA
    })
}

fn validate_stage_transition(
    from: &Version,
    from_stage: ReleaseStage,
    to: &Version,
    to_stage: ReleaseStage,
) -> Result<(), String> {
    if (from.major, from.minor, from.patch) != (to.major, to.minor, to.patch)
        || !from.build.is_empty()
        || !to.build.is_empty()
    {
        return Err(
            "stage transitions must preserve the release line and omit build metadata".to_owned(),
        );
    }
    let expected_prerelease = match (from_stage, to_stage) {
        (ReleaseStage::Alpha, ReleaseStage::Alpha) => {
            let from_iteration = prerelease_iteration(from, "alpha")?;
            let to_iteration = prerelease_iteration(to, "alpha")?;
            if to_iteration != from_iteration + 1 {
                return Err(
                    "Alpha iteration target must increment the prerelease number by one".to_owned(),
                );
            }
            return Ok(());
        }
        (ReleaseStage::ReleaseCandidate, ReleaseStage::ReleaseCandidate) => {
            let from_iteration = prerelease_iteration(from, "rc")?;
            let to_iteration = prerelease_iteration(to, "rc")?;
            if to_iteration != from_iteration + 1 {
                return Err(
                    "RC iteration target must increment the prerelease number by one".to_owned(),
                );
            }
            return Ok(());
        }
        (ReleaseStage::Alpha, ReleaseStage::Beta) => "beta.1",
        (ReleaseStage::Beta, ReleaseStage::ReleaseCandidate) => "rc.1",
        (ReleaseStage::ReleaseCandidate, ReleaseStage::Stable) => "",
        _ => {
            return Err(format!(
                "unsupported stage transition {} -> {}; only the next release stage is allowed",
                from_stage.as_str(),
                to_stage.as_str()
            ));
        }
    };
    if to.pre.as_str() != expected_prerelease {
        return Err(format!(
            "{} -> {} transition target must use prerelease `{expected_prerelease}`",
            from_stage.as_str(),
            to_stage.as_str()
        ));
    }
    Ok(())
}

fn prerelease_iteration(version: &Version, prefix: &str) -> Result<u64, String> {
    let (actual_prefix, iteration) = version
        .pre
        .as_str()
        .split_once('.')
        .ok_or_else(|| format!("{prefix} version has no numeric prerelease iteration"))?;
    if actual_prefix != prefix || iteration.contains('.') {
        return Err(format!("version prerelease must use `{prefix}.N`"));
    }
    iteration
        .parse::<u64>()
        .ok()
        .filter(|iteration| *iteration > 0)
        .ok_or_else(|| format!("{prefix} prerelease iteration must be a positive integer"))
}

fn validate_transition_ledger_preconditions(
    ledger: &Value,
    from_version: &Version,
    from_stage: ReleaseStage,
    to_stage: ReleaseStage,
) -> Result<(), String> {
    if ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != from_stage.as_str()
        || ledger["status"] != qualification_status_for_stage(from_stage)
        || ledger["stable_authorized"] != false
    {
        return Err("qualification ledger does not match the current workspace stage".to_owned());
    }
    if matches!(to_stage, ReleaseStage::ReleaseCandidate)
        && (ledger["beta"]["status"] != "qualified"
            || ledger["feature_freeze"]["status"] != "frozen")
    {
        return Err(
            "RC transition requires a qualified signed Beta and active feature freeze".to_owned(),
        );
    }
    if matches!(
        (from_stage, to_stage),
        (
            ReleaseStage::ReleaseCandidate,
            ReleaseStage::ReleaseCandidate
        )
    ) {
        let candidates = ledger["release_candidates"]
            .as_array()
            .ok_or_else(|| "sequential RC requires a qualification candidate list".to_owned())?;
        let current = candidates.last().ok_or_else(|| {
            "sequential RC requires the current RC matrix to be recorded first".to_owned()
        })?;
        let (tag, _, _) =
            validate_qualification_release(current, ReleaseStage::ReleaseCandidate, "current RC")?;
        if tag != format!("v{from_version}") {
            return Err(format!(
                "sequential RC requires current `{from_version}` evidence before preparing the next RC"
            ));
        }
    }
    if matches!(to_stage, ReleaseStage::Stable) {
        let mut candidate = ledger.clone();
        candidate["release_notes"]["status"] = Value::String("stable-final".to_owned());
        candidate["stable_authorized"] = Value::Bool(true);
        validate_stable_qualification(&candidate)?;
    }
    Ok(())
}

fn check_beta_transition_authorities(root: &Path, from_version: &Version) -> Result<(), String> {
    if prerelease_iteration(from_version, "alpha")? != 7 {
        return Err(
            "Beta transition is authorized only from the qualified dual-pack Alpha.7 checkpoint"
                .to_owned(),
        );
    }
    let readiness: Value = serde_json::from_slice(
        &fs::read(root.join("release/beta-readiness.json"))
            .map_err(|error| format!("Beta transition readiness is missing: {error}"))?,
    )
    .map_err(|error| format!("Beta transition readiness is invalid JSON: {error}"))?;
    if readiness["status"] != "qualified" || readiness["schema"] != BETA_READINESS_SCHEMA {
        return Err("Beta transition requires qualified Alpha.7 readiness evidence".to_owned());
    }
    let tag = format!("v{from_version}");
    if readiness["alpha_release"]["tag"] != tag {
        return Err("Beta readiness does not bind the active Alpha.7 tag".to_owned());
    }
    let source = required_string(
        &readiness["alpha_release"],
        "source_commit",
        "Alpha.7 release",
    )?;
    validate_lower_hex("Alpha.7 release source commit", source, 40)?;
    if readiness["alpha_release"]["release_run"]
        .as_u64()
        .filter(|run| *run > 0)
        .is_none()
        || required_string(
            &readiness["alpha_release"],
            "release_url",
            "Alpha.7 release",
        )? != format!("https://github.com/jxpeng98/CanISend/releases/tag/{tag}")
    {
        return Err("Beta readiness does not bind the exact public Alpha.7 run and URL".to_owned());
    }
    if readiness["contracts"] != beta_readiness_contracts(root)? {
        return Err(
            "Beta readiness does not bind canonical v4 contracts and both Pack digests".to_owned(),
        );
    }
    let freeze: Value = serde_json::from_slice(
        &fs::read(root.join("release/beta-contract-freeze.json"))
            .map_err(|error| format!("Beta contract freeze is missing: {error}"))?,
    )
    .map_err(|error| format!("Beta contract freeze is invalid JSON: {error}"))?;
    if freeze["baseline"]["release"] != tag || freeze["baseline"]["source_commit"] != source {
        return Err("Beta contract freeze does not bind the qualified Alpha.7 source".to_owned());
    }
    Ok(())
}

fn beta_readiness_contracts(root: &Path) -> Result<Value, String> {
    let mut packs = Vec::new();
    for id in [
        "org.canisend.academic-job",
        "org.canisend.generic-application",
    ] {
        let path = root.join(format!(
            "crates/canisend-resources/resources/workflow-packs/{id}/manifest.json"
        ));
        let manifest: Value = serde_json::from_slice(
            &fs::read(&path)
                .map_err(|error| format!("embedded workflow Pack `{id}` is missing: {error}"))?,
        )
        .map_err(|error| format!("embedded workflow Pack `{id}` is invalid JSON: {error}"))?;
        let digest = required_string(&manifest, "content_digest", "embedded workflow Pack")?;
        validate_lower_hex("embedded workflow Pack content digest", digest, 64)?;
        packs.push(json!({
            "id": required_string(&manifest, "id", "embedded workflow Pack")?,
            "version": required_string(&manifest, "version", "embedded workflow Pack")?,
            "content_digest": digest,
        }));
    }
    Ok(json!({
        "agent_protocol": AGENT_V4_PROTOCOL,
        "workspace_format": WORKSPACE_V4_FORMAT,
        "workflow_pack_format": "canisend.workflow-pack/v1",
        "workflow_packs": packs,
    }))
}

fn print_alpha_package_contract_bindings() -> Result<(), String> {
    let bindings = alpha_package_contract_bindings(&repository_root())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&bindings)
            .map_err(|error| format!("could not render Alpha package bindings: {error}"))?
    );
    Ok(())
}

fn alpha_package_contract_bindings(root: &Path) -> Result<Value, String> {
    let mut bindings = beta_readiness_contracts(root)?;
    let fields = bindings
        .as_object_mut()
        .ok_or_else(|| "Alpha package contract bindings must be an object".to_owned())?;

    let resource_manifest_path = "crates/canisend-resources/resources/manifest.json";
    let resource_manifest_bytes = fs::read(root.join(resource_manifest_path))
        .map_err(|error| format!("embedded resource manifest is missing: {error}"))?;
    let resource_manifest: Value = serde_json::from_slice(&resource_manifest_bytes)
        .map_err(|error| format!("embedded resource manifest is invalid JSON: {error}"))?;
    let resource_entry_count = resource_manifest
        .as_array()
        .ok_or_else(|| "embedded resource manifest must be an array".to_owned())?
        .len();
    fields.insert(
        "resource_manifest".to_owned(),
        json!({
            "path": resource_manifest_path,
            "entry_count": resource_entry_count,
            "sha256": hex::encode(Sha256::digest(&resource_manifest_bytes)),
        }),
    );

    let operation_registry_path = "crates/canisend-resources/resources/operations/v4/registry.json";
    let operation_registry_bytes = fs::read(root.join(operation_registry_path))
        .map_err(|error| format!("operation registry is missing: {error}"))?;
    let operation_registry: Value = serde_json::from_slice(&operation_registry_bytes)
        .map_err(|error| format!("operation registry is invalid JSON: {error}"))?;
    if operation_registry["format"] != "canisend.operation-registry/v4"
        || operation_registry["agent_protocol"] != AGENT_V4_PROTOCOL
        || operation_registry["workspace_format"] != WORKSPACE_V4_FORMAT
        || operation_registry["compatibility_aliases_supported"] != false
    {
        return Err("operation registry is not the clean Agent/Workspace v4 authority".to_owned());
    }
    fields.insert(
        "operation_registry".to_owned(),
        json!({
            "path": operation_registry_path,
            "format": "canisend.operation-registry/v4",
            "compatibility_alias_count": 0,
            "compatibility_pack_scope": "none",
            "sha256": hex::encode(Sha256::digest(&operation_registry_bytes)),
        }),
    );

    let migrations = migration_inventory_at(root)?;
    let current_schema_version = migrations
        .last()
        .map(|(version, _, _)| *version)
        .ok_or_else(|| "workspace migration inventory is empty".to_owned())?;
    let declared_schema_version = declared_database_schema_version_at(root)?;
    if current_schema_version != declared_schema_version {
        return Err(format!(
            "database schema constant {declared_schema_version} does not match migration inventory {current_schema_version}"
        ));
    }
    let migration_entries = migrations
        .iter()
        .map(|(_, name, path)| {
            read_frozen_contract_text(path, "migration").map(|bytes| (name.clone(), bytes))
        })
        .collect::<Result<Vec<_>, _>>()?;
    fields.insert(
        "migration_inventory".to_owned(),
        json!({
            "directory": "crates/canisend-store/migrations",
            "through": current_schema_version,
            "count": migration_entries.len(),
            "tree_sha256": digest_named_bytes(&migration_entries),
        }),
    );

    Ok(bindings)
}

fn release_notes_status_for_stage(stage: ReleaseStage) -> &'static str {
    match stage {
        ReleaseStage::Alpha => "alpha-current",
        ReleaseStage::Beta => "beta-current",
        ReleaseStage::ReleaseCandidate => "rc-final",
        ReleaseStage::Stable => "stable-final",
    }
}

fn replace_exact_count(
    body: &str,
    from: &str,
    to: &str,
    expected: usize,
    context: &str,
) -> Result<String, String> {
    let actual = body.matches(from).count();
    if actual != expected {
        return Err(format!(
            "{context} expected {expected} exact source values, found {actual}"
        ));
    }
    Ok(body.replace(from, to))
}

fn pretty_json_bytes(value: &Value) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("could not serialize stage-transition JSON: {error}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn stage_transition_report(
    root: &Path,
    transition: &RenderedStageTransition,
    write: bool,
) -> Result<Value, String> {
    let files = transition
        .files
        .iter()
        .map(|(relative, after)| {
            let before = fs::read(root.join(relative))
                .map_err(|error| format!("could not read {relative}: {error}"))?;
            Ok(json!({
                "path": relative,
                "before_sha256": sha256(&before),
                "after_sha256": sha256(after)
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let preserved_history = [
        "release/beta-readiness.json",
        "release/beta-contract-freeze.json",
        "release/feedback-snapshot.json",
        "packaging/candidates/alpha",
    ]
    .into_iter()
    .filter(|relative| !transition.files.contains_key(*relative))
    .collect::<Vec<_>>();
    Ok(json!({
        "schema": STAGE_TRANSITION_PLAN_SCHEMA,
        "mode": if write { "write" } else { "dry-run" },
        "writes_performed": write,
        "from": {
            "version": transition.from_version.to_string(),
            "stage": transition.from_stage.as_str()
        },
        "to": {
            "version": transition.to_version.to_string(),
            "stage": transition.to_stage.as_str()
        },
        "files": files,
        "preserved_history": preserved_history
    }))
}

fn check_internal_dependency_versions() -> Result<(), String> {
    let root = repository_root();
    let workspace_path = root.join("Cargo.toml");
    let workspace_body = fs::read_to_string(&workspace_path)
        .map_err(|error| format!("could not read workspace manifest: {error}"))?;
    let workspace: toml::Value = workspace_body
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    let members = workspace["workspace"]["members"]
        .as_array()
        .ok_or_else(|| "workspace manifest has no members array".to_owned())?;
    let expected = format!("={}", env!("CARGO_PKG_VERSION"));
    let mut manifests = Vec::with_capacity(members.len());
    let mut internal_packages = BTreeSet::new();

    for member in members {
        let member = member
            .as_str()
            .ok_or_else(|| "workspace member must be a string".to_owned())?;
        let path = root.join(member).join("Cargo.toml");
        let body = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let manifest: toml::Value = body
            .parse()
            .map_err(|error| format!("{} is invalid TOML: {error}", path.display()))?;
        let package = manifest["package"]["name"]
            .as_str()
            .ok_or_else(|| format!("{} has no package name", path.display()))?
            .to_owned();
        internal_packages.insert(package);
        manifests.push((member.to_owned(), manifest));
    }

    for (member, manifest) in &manifests {
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            check_dependency_table(
                member,
                section,
                manifest.get(section),
                &internal_packages,
                &expected,
            )?;
        }
        if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
            for (target, target_manifest) in targets {
                for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                    check_dependency_table(
                        member,
                        &format!("target.{target}.{section}"),
                        target_manifest.get(section),
                        &internal_packages,
                        &expected,
                    )?;
                }
            }
        }
    }

    println!(
        "internal dependency versions: ok ({} packages, {expected})",
        internal_packages.len()
    );
    Ok(())
}

fn print_third_party_lock_fingerprint() -> Result<(), String> {
    let (sha256, packages) = third_party_lock_fingerprint(&repository_root())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": THIRD_PARTY_LOCK_FINGERPRINT_SCHEMA,
            "sha256": sha256,
            "packages": packages,
        }))
        .map_err(|error| format!("could not serialize dependency fingerprint: {error}"))?
    );
    Ok(())
}

fn third_party_lock_fingerprint(root: &Path) -> Result<(String, usize), String> {
    let lock_body = fs::read_to_string(root.join("Cargo.lock"))
        .map_err(|error| format!("could not read Cargo.lock: {error}"))?;
    let lock: toml::Value = toml::from_str(&lock_body)
        .map_err(|error| format!("Cargo.lock is invalid TOML: {error}"))?;
    let packages = lock["package"]
        .as_array()
        .ok_or_else(|| "Cargo.lock has no package array".to_owned())?;
    let mut third_party = Vec::new();
    for package in packages {
        let Some(source) = package.get("source").and_then(toml::Value::as_str) else {
            continue;
        };
        let name = package
            .get("name")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| "third-party Cargo.lock package has no name".to_owned())?;
        let version = package
            .get("version")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("third-party Cargo.lock package {name} has no version"))?;
        let checksum = package
            .get("checksum")
            .and_then(toml::Value::as_str)
            .map(str::to_owned);
        let dependencies = package
            .get("dependencies")
            .and_then(toml::Value::as_array)
            .map(|dependencies| {
                dependencies
                    .iter()
                    .map(|dependency| {
                        dependency.as_str().map(str::to_owned).ok_or_else(|| {
                            format!("third-party Cargo.lock dependency of {name} is not a string")
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()
            })
            .transpose()?
            .unwrap_or_default();
        third_party.push(json!({
            "name": name,
            "version": version,
            "source": source,
            "checksum": checksum,
            "dependencies": dependencies,
        }));
    }
    third_party.sort_by(|left, right| {
        left["name"]
            .as_str()
            .expect("constructed package name")
            .cmp(right["name"].as_str().expect("constructed package name"))
            .then_with(|| {
                left["version"]
                    .as_str()
                    .expect("constructed package version")
                    .cmp(
                        right["version"]
                            .as_str()
                            .expect("constructed package version"),
                    )
            })
            .then_with(|| {
                left["source"]
                    .as_str()
                    .expect("constructed package source")
                    .cmp(
                        right["source"]
                            .as_str()
                            .expect("constructed package source"),
                    )
            })
    });
    let bytes = serde_json::to_vec(&third_party)
        .map_err(|error| format!("could not serialize third-party lock facts: {error}"))?;
    Ok((sha256(&bytes), third_party.len()))
}

fn check_dependency_assurance() -> Result<(), String> {
    let root = repository_root();
    let deny_body = fs::read_to_string(root.join("deny.toml"))
        .map_err(|error| format!("could not read deny.toml: {error}"))?;
    let deny: toml::Value = toml::from_str(&deny_body)
        .map_err(|error| format!("deny.toml is invalid TOML: {error}"))?;
    let ignored = deny["advisories"]["ignore"]
        .as_array()
        .ok_or_else(|| "deny.toml advisories.ignore must be an array".to_owned())?;
    let mut deny_exceptions = BTreeMap::new();
    for exception in ignored {
        let table = exception
            .as_table()
            .ok_or_else(|| "deny.toml advisory exception must be a table".to_owned())?;
        if table.keys().cloned().collect::<BTreeSet<_>>()
            != BTreeSet::from(["id".to_owned(), "reason".to_owned()])
        {
            return Err("deny.toml advisory exceptions must contain only id and reason".to_owned());
        }
        let id = table
            .get("id")
            .and_then(toml::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "deny.toml advisory exception has no id".to_owned())?;
        let reason = table
            .get("reason")
            .and_then(toml::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("deny.toml advisory exception {id} has no reason"))?;
        if deny_exceptions
            .insert(id.to_owned(), reason.to_owned())
            .is_some()
        {
            return Err(format!(
                "deny.toml contains duplicate advisory exception {id}"
            ));
        }
    }

    let policy_path = root.join("release/dependency-advisory-exceptions.json");
    let policy: Value = serde_json::from_slice(
        &fs::read(&policy_path)
            .map_err(|error| format!("dependency exception policy is missing: {error}"))?,
    )
    .map_err(|error| format!("dependency exception policy is invalid JSON: {error}"))?;
    if policy["schema"].as_str() != Some(DEPENDENCY_EXCEPTION_POLICY_SCHEMA) {
        return Err(format!(
            "dependency exception policy schema must be {DEPENDENCY_EXCEPTION_POLICY_SCHEMA}"
        ));
    }
    let (lock_sha256, package_count) = third_party_lock_fingerprint(&root)?;
    if policy["third_party_lock"]["schema"].as_str() != Some(THIRD_PARTY_LOCK_FINGERPRINT_SCHEMA)
        || policy["third_party_lock"]["sha256"].as_str() != Some(lock_sha256.as_str())
        || policy["third_party_lock"]["packages"].as_u64() != Some(package_count as u64)
    {
        return Err(
            "dependency exception review is not bound to the current third-party Cargo.lock facts"
                .to_owned(),
        );
    }

    let exceptions = policy["exceptions"]
        .as_array()
        .ok_or_else(|| "dependency exception policy needs an exceptions array".to_owned())?;
    let expected_fields = BTreeSet::from([
        "advisory_id",
        "kind",
        "owner",
        "reachability",
        "reviewed_on",
        "review_by",
        "expires_on",
        "removal_condition",
        "upstream_tracking",
    ]);
    let today = OffsetDateTime::now_utc().date();
    let vulnerability_ids = BTreeSet::from([
        "RUSTSEC-2026-0194".to_owned(),
        "RUSTSEC-2026-0195".to_owned(),
    ]);
    let mut policy_ids = BTreeSet::new();
    let mut previous_id: Option<&str> = None;
    for exception in exceptions {
        let object = exception
            .as_object()
            .ok_or_else(|| "dependency exception entry must be an object".to_owned())?;
        let fields = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
        if fields != expected_fields {
            return Err(format!(
                "dependency exception fields must be {expected_fields:?}, found {fields:?}"
            ));
        }
        let id = required_string(exception, "advisory_id", "dependency exception")?;
        if !id.starts_with("RUSTSEC-") || id.len() != 17 {
            return Err(format!("dependency exception advisory ID is invalid: {id}"));
        }
        if previous_id.is_some_and(|previous| previous >= id) {
            return Err("dependency exception IDs must be unique and sorted".to_owned());
        }
        previous_id = Some(id);
        policy_ids.insert(id.to_owned());
        let kind = required_string(exception, "kind", "dependency exception")?;
        let expected_kind = if vulnerability_ids.contains(id) {
            "vulnerability"
        } else {
            "unmaintained"
        };
        if kind != expected_kind {
            return Err(format!(
                "dependency exception {id} kind must be {expected_kind}"
            ));
        }
        if required_string(exception, "owner", "dependency exception")? != "CanISend maintainer" {
            return Err(format!("dependency exception {id} has no canonical owner"));
        }
        let reachability = required_string(exception, "reachability", "dependency exception")?;
        if deny_exceptions.get(id).map(String::as_str) != Some(reachability) {
            return Err(format!(
                "dependency exception {id} reachability differs from deny.toml"
            ));
        }
        let reviewed_on = parse_policy_date(
            required_string(exception, "reviewed_on", "dependency exception")?,
            "reviewed_on",
        )?;
        let review_by = parse_policy_date(
            required_string(exception, "review_by", "dependency exception")?,
            "review_by",
        )?;
        let expires_on = parse_policy_date(
            required_string(exception, "expires_on", "dependency exception")?,
            "expires_on",
        )?;
        validate_dependency_exception_dates(id, reviewed_on, review_by, expires_on, today)?;
        let removal = required_string(exception, "removal_condition", "dependency exception")?;
        if removal.len() < 24 {
            return Err(format!(
                "dependency exception {id} removal condition is too weak"
            ));
        }
        let tracking = required_string(exception, "upstream_tracking", "dependency exception")?;
        if !tracking.starts_with("https://") {
            return Err(format!("dependency exception {id} tracking must use HTTPS"));
        }
    }
    let deny_ids = deny_exceptions.keys().cloned().collect::<BTreeSet<_>>();
    if policy_ids != deny_ids {
        return Err(format!(
            "dependency exception policy IDs differ from deny.toml: policy={policy_ids:?}, deny={deny_ids:?}"
        ));
    }
    if !vulnerability_ids.is_subset(&policy_ids) {
        return Err(
            "known quick-xml vulnerability exceptions are not explicitly governed".to_owned(),
        );
    }

    let render = fs::read_to_string(root.join("crates/canisend-io/src/render.rs"))
        .map_err(|error| format!("could not inspect renderer boundary: {error}"))?;
    if render.contains("bibliography(") || render.contains("publication(") {
        return Err(
            "the fixed renderer invokes the transitive quick-xml bibliography surface".to_owned(),
        );
    }
    let cv_template = fs::read_to_string(
        root.join("crates/canisend-resources/resources/templates/modernpro-cv.typ"),
    )
    .map_err(|error| format!("could not inspect modernpro CV template: {error}"))?;
    if cv_template.matches("publication(").count() != 1
        || !cv_template.contains("#let publication(path, styletype)")
    {
        return Err(
            "the embedded CV template bibliography helper is no longer declaration-only".to_owned(),
        );
    }

    let workflow = fs::read_to_string(root.join(".github/workflows/dependency-assurance.yml"))
        .map_err(|error| format!("dependency assurance workflow is missing: {error}"))?;
    for required in [
        "name: dependency-assurance",
        "Cargo.lock",
        "**/Cargo.toml",
        "deny.toml",
        "release/dependency-advisory-exceptions.json",
        "xtask/src/**",
        "crates/canisend-io/src/render.rs",
        "crates/canisend-resources/resources/templates/modernpro-cv.typ",
        "docs/release/dependency-assurance.md",
        "runs-on: ubuntu-24.04",
        "uses: dtolnay/rust-toolchain@1.97.0",
        "cargo run -p xtask --locked -- dependencies check",
        "uses: EmbarkStudios/cargo-deny-action@6c8f9facfa5047ec02d8485b6bf52b587b7777d1",
        "command-arguments: advisories bans licenses sources",
        "RUSTC_WRAPPER: \"\"",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "dependency assurance workflow is missing invariant `{required}`"
            ));
        }
    }
    for forbidden in [
        "contents: write",
        "packages: write",
        "releases: write",
        "git push",
        "cargo build --release",
        "cargo test --release",
    ] {
        if workflow.contains(forbidden) {
            return Err(format!(
                "dependency assurance workflow exceeds its read-only scope: `{forbidden}`"
            ));
        }
    }

    let guide = fs::read_to_string(root.join("docs/release/dependency-assurance.md"))
        .map_err(|error| format!("dependency assurance guide is missing: {error}"))?;
    for required in [
        "dependency-advisory-exceptions.json",
        "dependencies check",
        "review_by",
        "expires_on",
        "quick-xml",
        "packaged-binary qualification",
    ] {
        if !guide.contains(required) {
            return Err(format!(
                "dependency assurance guide is missing `{required}`"
            ));
        }
    }

    println!(
        "dependency assurance: ok ({} reviewed exceptions, {} vulnerabilities, {} third-party packages)",
        exceptions.len(),
        vulnerability_ids.len(),
        package_count
    );
    Ok(())
}

fn validate_dependency_exception_dates(
    id: &str,
    reviewed_on: Date,
    review_by: Date,
    expires_on: Date,
    today: Date,
) -> Result<(), String> {
    if reviewed_on > today || review_by < reviewed_on || expires_on < review_by {
        return Err(format!(
            "dependency exception {id} has inconsistent review dates"
        ));
    }
    if review_by - reviewed_on > Duration::days(14) || expires_on - reviewed_on > Duration::days(30)
    {
        return Err(format!(
            "dependency exception {id} review window is too broad"
        ));
    }
    if today > review_by {
        return Err(format!(
            "dependency exception {id} review is overdue on {review_by}"
        ));
    }
    if today > expires_on {
        return Err(format!("dependency exception {id} expired on {expires_on}"));
    }
    Ok(())
}

fn check_rust_toolchain_alignment() -> Result<(), String> {
    let root = repository_root();
    let workspace: toml::Value = toml::from_str(
        &fs::read_to_string(root.join("Cargo.toml"))
            .map_err(|error| format!("could not read workspace Cargo.toml: {error}"))?,
    )
    .map_err(|error| format!("workspace Cargo.toml is invalid: {error}"))?;
    let declared = workspace
        .get("workspace")
        .and_then(|value| value.get("package"))
        .and_then(|value| value.get("rust-version"))
        .and_then(toml::Value::as_str);
    if declared != Some(DECLARED_RUST_VERSION) {
        return Err(format!(
            "workspace rust-version must be {DECLARED_RUST_VERSION}, found {}",
            declared.unwrap_or("<missing>")
        ));
    }

    let toolchain: toml::Value = toml::from_str(
        &fs::read_to_string(root.join("rust-toolchain.toml"))
            .map_err(|error| format!("could not read rust-toolchain.toml: {error}"))?,
    )
    .map_err(|error| format!("rust-toolchain.toml is invalid: {error}"))?;
    let channel = toolchain
        .get("toolchain")
        .and_then(|value| value.get("channel"))
        .and_then(toml::Value::as_str);
    if channel != Some(PINNED_RUST_TOOLCHAIN) {
        return Err(format!(
            "pinned Rust channel must be {PINNED_RUST_TOOLCHAIN}, found {}",
            channel.unwrap_or("<missing>")
        ));
    }

    let clippy: toml::Value = toml::from_str(
        &fs::read_to_string(root.join("clippy.toml"))
            .map_err(|error| format!("could not read clippy.toml: {error}"))?,
    )
    .map_err(|error| format!("clippy.toml is invalid: {error}"))?;
    if clippy.get("msrv").and_then(toml::Value::as_str) != Some(PINNED_RUST_TOOLCHAIN) {
        return Err(format!("Clippy MSRV must be {PINNED_RUST_TOOLCHAIN}"));
    }

    let readme = fs::read_to_string(root.join("README.md"))
        .map_err(|error| format!("could not read README.md: {error}"))?;
    for required in [
        "Rust-1.97%2B-orange",
        &format!("alt=\"Rust {DECLARED_RUST_VERSION}+\""),
    ] {
        if !readme.contains(required) {
            return Err(format!("README Rust badge is missing `{required}`"));
        }
    }

    let workflow_dir = root.join(".github/workflows");
    let mut stable_action_uses = 0_usize;
    for entry in fs::read_dir(&workflow_dir)
        .map_err(|error| format!("could not read {}: {error}", workflow_dir.display()))?
    {
        let path = entry
            .map_err(|error| format!("could not inspect workflow entry: {error}"))?
            .path();
        if !matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("yml" | "yaml")
        ) {
            continue;
        }
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        for (index, line) in source.lines().enumerate() {
            let Some((_, version)) = line.split_once("uses: dtolnay/rust-toolchain@") else {
                continue;
            };
            stable_action_uses += 1;
            let version = version.split_whitespace().next().unwrap_or_default();
            if version != PINNED_RUST_TOOLCHAIN {
                return Err(format!(
                    "{}:{} pins Rust {version}, expected {PINNED_RUST_TOOLCHAIN}",
                    path.display(),
                    index + 1
                ));
            }
        }
    }
    if stable_action_uses == 0 {
        return Err("no pinned stable Rust workflow action was found".to_owned());
    }

    let native_policy: Value = serde_json::from_slice(
        &fs::read(root.join("release/native-test-ownership.json"))
            .map_err(|error| format!("could not read native test ownership policy: {error}"))?,
    )
    .map_err(|error| format!("native test ownership policy is invalid JSON: {error}"))?;
    if native_policy["source_gate"]["rust_toolchain"].as_str() != Some(PINNED_RUST_TOOLCHAIN) {
        return Err(format!(
            "native source gate must declare Rust {PINNED_RUST_TOOLCHAIN}"
        ));
    }

    println!(
        "Rust toolchain: ok (declared {DECLARED_RUST_VERSION}, pinned {PINNED_RUST_TOOLCHAIN}, {stable_action_uses} workflow uses)"
    );
    Ok(())
}

fn check_desktop_distribution_versions() -> Result<(), String> {
    let root = repository_root();
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let expected = version.to_string();

    for (relative, pointer, context) in [
        (
            "apps/canisend-desktop/package.json",
            "/version",
            "desktop package version",
        ),
        (
            "crates/canisend-desktop/tauri.conf.json",
            "/version",
            "Tauri application version",
        ),
    ] {
        let document: Value = serde_json::from_slice(
            &fs::read(root.join(relative))
                .map_err(|error| format!("could not read {relative}: {error}"))?,
        )
        .map_err(|error| format!("{relative} is invalid JSON: {error}"))?;
        if document.pointer(pointer).and_then(Value::as_str) != Some(expected.as_str()) {
            return Err(format!("{context} must match workspace version {expected}"));
        }
    }

    let windows_relative = "crates/canisend-desktop/tauri.windows.conf.json";
    let windows: Value = serde_json::from_slice(
        &fs::read(root.join(windows_relative))
            .map_err(|error| format!("could not read {windows_relative}: {error}"))?,
    )
    .map_err(|error| format!("{windows_relative} is invalid JSON: {error}"))?;
    let expected_msi = windows_msi_product_version(&version)?;
    if windows
        .pointer("/bundle/windows/wix/version")
        .and_then(Value::as_str)
        != Some(expected_msi.as_str())
    {
        return Err(format!(
            "Windows MSI product version must be {expected_msi} for workspace version {expected}"
        ));
    }
    println!("desktop distribution versions: ok ({expected}, Windows MSI {expected_msi})");
    Ok(())
}

fn check_dependency_table(
    member: &str,
    section: &str,
    dependencies: Option<&toml::Value>,
    internal_packages: &BTreeSet<String>,
    expected: &str,
) -> Result<(), String> {
    let Some(dependencies) = dependencies.and_then(toml::Value::as_table) else {
        return Ok(());
    };
    for (alias, dependency) in dependencies {
        let Some(detail) = dependency.as_table() else {
            continue;
        };
        if !detail.contains_key("path") {
            continue;
        }
        let package = detail
            .get("package")
            .and_then(toml::Value::as_str)
            .unwrap_or(alias);
        if !internal_packages.contains(package) {
            continue;
        }
        let actual = detail.get("version").and_then(toml::Value::as_str);
        if actual != Some(expected) {
            return Err(format!(
                "internal dependency `{alias}` in {member}/Cargo.toml [{section}] must use exact version `{expected}`, found {}",
                actual.unwrap_or("<missing>")
            ));
        }
    }
    Ok(())
}

fn check_native_test_ownership() -> Result<(), String> {
    let root = repository_root();
    let policy_path = root.join("release/native-test-ownership.json");
    let policy: Value = serde_json::from_slice(&fs::read(&policy_path).map_err(|error| {
        format!(
            "native test ownership policy is missing at {}: {error}",
            policy_path.display()
        )
    })?)
    .map_err(|error| format!("native test ownership policy is invalid JSON: {error}"))?;
    let expected = json!({
        "schema": NATIVE_TEST_OWNERSHIP_SCHEMA,
        "source_gate": {
            "command": "cargo test --workspace --locked",
            "frontend": {
                "browser_channel": "chrome",
                "commands": [
                    "pnpm --dir apps/canisend-desktop install --frozen-lockfile",
                    "pnpm --dir apps/canisend-desktop format:check",
                    "pnpm --dir apps/canisend-desktop check",
                    "pnpm --dir apps/canisend-desktop test",
                    "pnpm --dir apps/canisend-desktop build",
                    "pnpm --dir apps/canisend-desktop exec playwright install --with-deps chrome",
                    "pnpm --dir apps/canisend-desktop test:accessibility"
                ],
                "node_version": "26.5.0",
                "pnpm_version": "11.17.0"
            },
            "property_contract_command":
                "cargo test -p canisend-contracts --locked --test property_contract",
            "rust_toolchain": PINNED_RUST_TOOLCHAIN,
            "runner": "ubuntu-24.04",
            "runs_per_candidate": 1
        },
        "compiler_cache": {
            "action":
                "mozilla-actions/sccache-action@9e7fa8a12102821edf02ca5dbea1acd0f89a2696",
            "action_release": "v0.0.10",
            "binary_version": "v0.16.0",
            "backend": "github-actions-v2",
            "checksum_verification": "official-release-sha256-sidecar",
            "cache_namespaces_include": [
                "target",
                "rust-version",
                "profile",
                "feature-set",
                "epoch"
            ],
            "cargo_registry_cache_only": true,
            "fallback": "ordinary-cargo",
            "setup_failure_blocks_build": false,
            "stats_schema": SCCACHE_STATS_SCHEMA,
            "authoritative_release_evidence": false,
            "time_saved_measurement": "cold-warm-invalidated-candidate"
        },
        "build_profiles": {
            "alpha": "release-alpha",
            "beta": "release",
            "rc": "release",
            "stable": "release"
        },
        "development_fast_ci": {
            "workflow": ".github/workflows/fast-ci.yml",
            "runners": [
                "macos-15",
                "ubuntu-24.04",
                "windows-2025"
            ],
            "jobs": [
                "browser-keyboard-accessibility",
                "cross-platform-core",
                "desktop-ui",
                "macos-quality",
                "macos-tests"
            ],
            "commands": [
                "pnpm install --frozen-lockfile",
                "pnpm format:check",
                "pnpm check",
                "pnpm test",
                "pnpm build",
                "pnpm exec playwright install --with-deps chrome",
                "pnpm test:accessibility",
                "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings",
                "cargo test --workspace --locked",
                "cargo test --locked -p canisend-core -p canisend-store -p canisend-io -p canisend-cli -p canisend-mcp",
                "cargo test -p canisend-contracts --locked --test property_contract",
                "cargo run -p xtask --locked -- release check",
                "cargo build --locked -p canisend-cli -p canisend-gui --features canisend-gui/custom-protocol"
            ],
            "target_seconds_after_cache_warmup": 300,
            "windows_linux_native_tests": true,
            "authoritative_release_evidence": false
        },
        "candidate_native_matrix": {
            "common_gates": [
                "locked-release-build",
                "exact-extracted-archive-binary",
                "version-and-doctor",
                "documented-quickstart",
                "agent-v4-host-smoke",
                "agent-v4-mcp-lifecycle-smoke",
                "isolated-install-uninstall-workspace-retention"
            ],
            "targets": [
                {
                    "owned_gates": [
                        "native-runner-architecture",
                        "tar-gzip-archive",
                        "apple-adhoc-signing-beta-plus"
                    ],
                    "runner": "macos-15",
                    "target": "aarch64-apple-darwin"
                },
                {
                    "owned_gates": [
                        "native-runner-architecture",
                        "tar-gzip-archive",
                        "apple-adhoc-signing-beta-plus",
                        "intel-gui-compile-beta-plus"
                    ],
                    "runner": "macos-15-intel",
                    "target": "x86_64-apple-darwin"
                },
                {
                    "owned_gates": [
                        "native-runner-architecture",
                        "tar-gzip-archive",
                        "release-performance-budget",
                        "full-synthetic-workflow-budget"
                    ],
                    "runner": "ubuntu-24.04",
                    "target": "x86_64-unknown-linux-gnu"
                },
                {
                    "owned_gates": [
                        "native-runner-architecture",
                        "tar-gzip-archive",
                        "musl-linker"
                    ],
                    "runner": "ubuntu-24.04",
                    "target": "x86_64-unknown-linux-musl"
                },
                {
                    "owned_gates": [
                        "native-runner-architecture",
                        "zip-archive",
                        "powershell-parser",
                        "authenticode-self-signed-beta-plus"
                    ],
                    "runner": "windows-2025",
                    "target": "x86_64-pc-windows-msvc"
                }
            ],
            "timing_evidence_schema": "canisend.native-release-timing/v1",
            "workspace_suite_repeated_per_target": false
        },
        "desktop_gui": {
            "owned_gates": [
                "locked-cli-gui-release-build",
                "bounded-app-archive",
                "compressed-dmg-image",
                "readonly-dmg-mount",
                "applications-link",
                "nested-and-outer-adhoc-signatures",
                "exact-companion-integrity",
                "packaged-cli-workflows",
                "packaged-gui-launch"
            ],
            "runner": "macos-15",
            "target": "aarch64-apple-darwin",
            "timing_evidence_schema": "canisend.native-release-timing/v1"
        },
        "extended_assurance": [
            {
                "owner": "fast-ci/desktop-ui",
                "scope": "Formatting, Svelte and TypeScript checks, unit tests, and production frontend build"
            },
            {
                "owner": "fast-ci/browser-keyboard-accessibility",
                "scope": "Chrome keyboard, focus restoration, automated accessibility, reflow, and key visual-state checks"
            },
            {
                "owner": "fast-ci/cross-platform-core",
                "scope": "Ubuntu and Windows core, Store, IO, CLI, and MCP contract tests"
            },
            {
                "owner": "native-release/source-gates",
                "scope": "Locked formatting, Svelte and TypeScript checks, unit tests, production build, and Chrome accessibility checks"
            },
            {
                "owner": "fast-ci/macos-tests",
                "scope": "macOS development workspace, recovery, rendering, CLI, and GUI"
            },
            {
                "owner": "native-release/source-and-native",
                "scope": "release-only Linux and Windows tests, performance, packaging, and exact archive smoke"
            },
            {
                "owner": "intel-gui-compile/scheduled-alpha",
                "scope": "non-publishing Intel GUI compile regression"
            },
            {
                "owner": "desktop-platform-qualification/scheduled",
                "scope": "non-publishing Windows and Linux latest-template profile matrices, one-host desktop packages, runtime smoke, and size evidence"
            },
            {
                "owner": "fuzz/scheduled",
                "scope": "extended malformed-input fuzzing"
            }
        ]
    });
    if policy != expected {
        return Err(
            "native test ownership policy contains unknown, missing, or noncanonical fields"
                .to_owned(),
        );
    }

    let release_targets = release_targets()?
        .into_iter()
        .map(|target| (target.triple, target.runner))
        .collect::<BTreeSet<_>>();
    let policy_targets = policy["candidate_native_matrix"]["targets"]
        .as_array()
        .ok_or_else(|| "native test ownership targets are missing".to_owned())?
        .iter()
        .map(|target| {
            Ok((
                required_string(target, "target", "native test ownership target")?.to_owned(),
                required_string(target, "runner", "native test ownership target")?.to_owned(),
            ))
        })
        .collect::<Result<BTreeSet<_>, String>>()?;
    if release_targets != policy_targets {
        return Err("native test ownership targets do not match release targets".to_owned());
    }

    let fast_ci_path = root.join(".github/workflows/fast-ci.yml");
    let fast_ci = fs::read_to_string(&fast_ci_path)
        .map_err(|error| format!("fast CI workflow is missing: {error}"))?;
    for required in [
        "name: fast-ci",
        "cancel-in-progress: true",
        "  desktop-ui:",
        "  browser-keyboard-accessibility:",
        "  cross-platform-core:",
        "  macos-quality:",
        "  macos-tests:",
        "runs-on: macos-15",
        "runs-on: ubuntu-24.04",
        "runs-on: ${{ matrix.runner }}",
        "runner: ubuntu-24.04",
        "runner: windows-2025",
        "pnpm install --frozen-lockfile",
        "pnpm format:check",
        "pnpm check",
        "pnpm test",
        "pnpm build",
        "Install Chrome for keyboard and accessibility gate",
        "Run Chrome keyboard, accessibility, and key-visual checks",
        "pnpm exec playwright install --with-deps chrome",
        "pnpm test:accessibility",
        "Upload production desktop UI",
        "Download exact production desktop UI",
        "needs: desktop-ui",
        "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings",
        "cargo test --workspace --locked",
        "cargo test --locked -p canisend-core -p canisend-store -p canisend-io -p canisend-cli -p canisend-mcp",
        "cargo test -p canisend-contracts --locked --test property_contract",
        "cargo run -p xtask --locked -- release check",
        "cargo build --locked -p canisend-cli -p canisend-gui",
        "--features canisend-gui/custom-protocol",
        "Smoke Agent v4 host resources and MCP through the built CLI",
        "./scripts/smoke_host_v4.sh",
        "$RUNNER_TEMP/canisend-host-v4-smoke",
        "./scripts/smoke_agent_v4_mcp.sh",
        "./target/debug/canisend",
        "$RUNNER_TEMP/canisend-agent-v4-mcp-smoke",
        "Target: 300 seconds or less after cache warm-up",
    ] {
        if !fast_ci.contains(required) {
            return Err(format!(
                "fast CI workflow is missing invariant `{required}`"
            ));
        }
    }
    if fast_ci.matches("runs-on: macos-15").count() != 3 {
        return Err(
            "fast CI must contain exactly three macOS jobs, including the shared desktop UI build"
                .to_owned(),
        );
    }
    if fast_ci.matches("needs: desktop-ui").count() != 2 {
        return Err(
            "both macOS Rust jobs must consume the exact production desktop UI build".to_owned(),
        );
    }
    let host_v4_smoke_path = root.join("scripts/smoke_host_v4.sh");
    let host_v4_smoke = fs::read_to_string(&host_v4_smoke_path)
        .map_err(|error| format!("Agent v4 host smoke script is missing: {error}"))?;
    for required in [
        "workspace init --json",
        "host setup",
        "host status",
        "host remove",
        "--host codex",
        "unsupported pre-v4 host resources",
        "PRE-V4-HOST-RESOURCE-SENTINEL",
        "Agent v4 host smoke: ok",
    ] {
        if !host_v4_smoke.contains(required) {
            return Err(format!(
                "Agent v4 host smoke script is missing invariant `{required}`"
            ));
        }
    }
    let agent_v4_mcp_smoke_path = root.join("scripts/smoke_agent_v4_mcp.sh");
    let agent_v4_mcp_smoke = fs::read_to_string(&agent_v4_mcp_smoke_path)
        .map_err(|error| format!("Agent v4 MCP smoke script is missing: {error}"))?;
    for required in [
        "workspace init --json",
        "--pack org.canisend.generic-application",
        "--pack org.canisend.academic-job",
        "canisend_workspace_status",
        "canisend_workspace_check",
        "canisend_application_list",
        "canisend_application_show",
        "profile-source import",
        "profile-source list",
        "profile association list",
        "evidence association list",
        "canisend_profile_source_list",
        "canisend_profile_association_list",
        "canisend_profile_association_preview",
        "canisend_profile_association_commit",
        "canisend_evidence_association_list",
        "canisend_evidence_association_preview",
        "canisend_evidence_association_commit",
        "canisend_requirement_extract_preview",
        "canisend_requirement_extract_commit",
        "canisend_requirement_confirm_preview",
        "canisend_requirement_confirm_commit",
        "canisend_plan_propose_preview",
        "canisend_plan_propose_commit",
        "canisend_plan_confirm_preview",
        "canisend_plan_confirm_commit",
        "canisend_deliverable_draft_preview",
        "canisend_deliverable_draft_commit",
        "canisend_deliverable_audit",
        "approved: false",
        "full guarded dual-Pack lifecycle passed",
        "MCP-V4-PROFILE-PRIVATE-SENTINEL",
        "MCP-V4-GENERIC-PRIVATE-SENTINEL",
        "MCP-V4-ACADEMIC-PRIVATE-SENTINEL",
    ] {
        if !agent_v4_mcp_smoke.contains(required) {
            return Err(format!(
                "Agent v4 MCP smoke script is missing invariant `{required}`"
            ));
        }
    }
    let release_archive_smoke =
        fs::read_to_string(root.join("scripts/smoke_release_archive.sh"))
            .map_err(|error| format!("release archive smoke script is missing: {error}"))?;
    if !release_archive_smoke
        .contains("smoke_host_v4.sh\" \"$executable\" \"$smoke_root/host-v4-workflow")
    {
        return Err(
            "release archive smoke must exercise clean Agent v4 host resources on every packaged CLI"
                .to_owned(),
        );
    }
    if !release_archive_smoke
        .contains("smoke_agent_v4_mcp.sh\" \"$executable\" \"$smoke_root/agent-v4-mcp-workflow")
    {
        return Err(
            "release archive smoke must exercise Agent v4 MCP on every packaged CLI".to_owned(),
        );
    }
    let git_attributes_path = root.join(".gitattributes");
    let git_attributes = fs::read_to_string(&git_attributes_path)
        .map_err(|error| format!("Git attributes are missing: {error}"))?;
    if !git_attributes
        .lines()
        .any(|line| line.trim() == "*.md text eol=lf")
    {
        return Err(
            "Pack-bound Markdown resources must use canonical LF checkout bytes".to_owned(),
        );
    }
    let browser_start = fast_ci
        .find("\n  browser-keyboard-accessibility:\n")
        .ok_or_else(|| "fast CI browser job is missing".to_owned())?;
    let browser_tail = &fast_ci[browser_start..];
    let browser_end = browser_tail
        .find("\n  cross-platform-core:\n")
        .ok_or_else(|| "fast CI browser job boundary is missing".to_owned())?;
    let browser_job = &browser_tail[..browser_end];
    for required in [
        "runs-on: ubuntu-24.04",
        "node-version: 26.5.0",
        "version: 11.17.0",
        "pnpm install --frozen-lockfile",
        "pnpm exec playwright install --with-deps chrome",
        "pnpm test:accessibility",
    ] {
        if !browser_job.contains(required) {
            return Err(format!(
                "fast CI browser job is missing invariant `{required}`"
            ));
        }
    }
    let core_start = fast_ci
        .find("\n  cross-platform-core:\n")
        .ok_or_else(|| "fast CI cross-platform core job is missing".to_owned())?;
    let core_tail = &fast_ci[core_start..];
    let core_end = core_tail
        .find("\n  macos-quality:\n")
        .ok_or_else(|| "fast CI cross-platform core job boundary is missing".to_owned())?;
    let core_job = &core_tail[..core_end];
    for required in [
        "runs-on: ${{ matrix.runner }}",
        "runner: ubuntu-24.04",
        "runner: windows-2025",
        "uses: dtolnay/rust-toolchain@1.97.0",
        "cargo test --locked -p canisend-core -p canisend-store -p canisend-io -p canisend-cli -p canisend-mcp",
    ] {
        if !core_job.contains(required) {
            return Err(format!(
                "fast CI cross-platform core job is missing invariant `{required}`"
            ));
        }
    }
    for forbidden in [
        "cargo build --release",
        "cargo test --release",
        "cargo build --profile",
        "cargo test --profile",
    ] {
        if fast_ci.contains(forbidden) {
            return Err(format!(
                "fast CI contains release-only or non-macOS work `{forbidden}`"
            ));
        }
    }

    let workflow_path = root.join(".github/workflows/release.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("release workflow is missing: {error}"))?;
    let source_gate_start = workflow
        .find("\n  source-gates:\n")
        .ok_or_else(|| "release workflow source-gates job is missing".to_owned())?;
    let source_gate_tail = &workflow[source_gate_start..];
    let source_gate_end = source_gate_tail
        .find("\n  windows-release-tests:\n")
        .ok_or_else(|| "release workflow source-gates boundary is missing".to_owned())?;
    let source_gate_job = &source_gate_tail[..source_gate_end];
    for required in [
        "if: needs.release-identity.outputs.mode == 'candidate'",
        "runs-on: ubuntu-24.04",
        "Install pinned pnpm for source gates",
        "version: 11.17.0",
        "Install pinned Node.js for source gates",
        "node-version: 26.5.0",
        "Install Chrome for accessibility source gate",
        "Run critical browser accessibility checks for release",
    ] {
        if !source_gate_job.contains(required) {
            return Err(format!(
                "release source-gates job is missing frontend invariant `{required}`"
            ));
        }
    }
    let frontend_commands = policy["source_gate"]["frontend"]["commands"]
        .as_array()
        .ok_or_else(|| "native source-gate frontend commands are missing".to_owned())?;
    for command in frontend_commands {
        let command = command
            .as_str()
            .ok_or_else(|| "native source-gate frontend command must be a string".to_owned())?;
        if !source_gate_job.contains(command) {
            return Err(format!(
                "release source-gates job is missing frontend command `{command}`"
            ));
        }
    }

    let desktop_package: Value = serde_json::from_slice(
        &fs::read(root.join("apps/canisend-desktop/package.json"))
            .map_err(|error| format!("desktop package is missing: {error}"))?,
    )
    .map_err(|error| format!("desktop package is invalid JSON: {error}"))?;
    let expected_accessibility_script = "playwright test tests/visual/application-shell.a11y.spec.ts tests/visual/ui-system.a11y.spec.ts";
    if desktop_package["scripts"]["test:accessibility"].as_str()
        != Some(expected_accessibility_script)
    {
        return Err(
            "desktop test:accessibility must run only the two critical accessibility specs"
                .to_owned(),
        );
    }
    let expected_format_script =
        "prettier --check \"src/**/*.{svelte,ts}\" \"tests/**/*.ts\" \"*.{ts,js,json}\"";
    if desktop_package["scripts"]["format:check"].as_str() != Some(expected_format_script) {
        return Err(
            "desktop format:check must cover Svelte, TypeScript, and UI configuration".to_owned(),
        );
    }
    for (dependency, version) in [("prettier", "3.9.6"), ("prettier-plugin-svelte", "4.1.1")] {
        if desktop_package["devDependencies"][dependency].as_str() != Some(version) {
            return Err(format!(
                "desktop formatting gate must pin {dependency} {version}"
            ));
        }
    }
    let prettier = fs::read_to_string(root.join("apps/canisend-desktop/.prettierrc.json"))
        .map_err(|error| format!("desktop Prettier config is missing: {error}"))?;
    let prettier: Value = serde_json::from_str(&prettier)
        .map_err(|error| format!("desktop Prettier config is invalid JSON: {error}"))?;
    if prettier
        != json!({
            "plugins": ["prettier-plugin-svelte"],
            "printWidth": 100,
            "overrides": [{
                "files": "src/lib/components/ui/**/*.{svelte,ts}",
                "options": {"useTabs": true}
            }]
        })
    {
        return Err(
            "desktop Prettier config must preserve the pinned Svelte formatting profile".to_owned(),
        );
    }
    let playwright = fs::read_to_string(root.join("apps/canisend-desktop/playwright.config.ts"))
        .map_err(|error| format!("desktop Playwright config is missing: {error}"))?;
    if !playwright.contains("channel: \"chrome\"") {
        return Err(
            "desktop accessibility source gate must use the pinned Chrome channel".to_owned(),
        );
    }

    let source_suite = "cargo test --workspace --locked";
    if workflow.matches(source_suite).count() != 1 {
        return Err(
            "release workflow must run the complete locked workspace suite exactly once".to_owned(),
        );
    }
    if workflow.contains("cargo test --workspace --locked --target") {
        return Err(
            "release native matrix must not repeat the complete workspace suite per target"
                .to_owned(),
        );
    }
    for required in [
        "Run native release timing regression",
        "./scripts/test_native_release_timing.sh",
        "./scripts/write_native_release_timing.sh",
        "Smoke exact extracted release archive",
        "Check release performance budgets",
        "Check full synthetic workflow budget",
        "Build compile-only Intel macOS GUI",
        "Parse Windows release signing verifier",
        "windows-release-tests",
        "Exercise Windows recovery and concurrency contracts",
        "Test Windows embedded fonts and complex layout",
        "Test Windows revision-bound package render",
        "Install musl linker",
        "Package exact ad-hoc-signed macOS ZIP and DMG",
        "Smoke exact extracted macOS application archive",
        "Enforce one-host desktop payload budget",
        "desktop size-record",
        "Smoke exact read-only macOS application DMG",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "release workflow is missing native test owner `{required}`"
            ));
        }
    }
    let performance_contract =
        fs::read_to_string(root.join("crates/canisend-cli/tests/performance_contract.rs"))
            .map_err(|error| format!("release performance contract is missing: {error}"))?;
    for required in [
        "preview_pasted_text_intake_v4",
        "preview_local_file_intake_v4",
        "host_status_startup_median_ms",
        "status_100_applications_median_ms",
    ] {
        if !performance_contract.contains(required) {
            return Err(format!(
                "clean-v4 release performance contract is missing `{required}`"
            ));
        }
    }
    for retired in ["Application::create_job", "agent\", \"capabilities"] {
        if performance_contract.contains(retired) {
            return Err(format!(
                "release performance contract retains retired surface `{retired}`"
            ));
        }
    }
    for required in [
        "      - name: Build compile-only Intel macOS GUI\n        if: matrix.target == 'x86_64-apple-darwin' && needs.release-identity.outputs.stage != 'alpha'",
        "      - name: Record Intel macOS GUI compilation evidence\n        if: matrix.target == 'x86_64-apple-darwin' && needs.release-identity.outputs.stage != 'alpha'",
    ] {
        if !workflow.contains(required) {
            return Err(
                "release workflow must limit Intel GUI release evidence to Beta or later"
                    .to_owned(),
            );
        }
    }
    let sccache_action = "mozilla-actions/sccache-action@9e7fa8a12102821edf02ca5dbea1acd0f89a2696";
    if workflow.matches(sccache_action).count() != 3
        || workflow.matches("          version: \"v0.16.0\"").count() != 3
        || workflow.matches("          cache-targets: false").count() != 3
        || workflow.matches("./scripts/configure_sccache.sh").count() != 3
        || workflow.matches("./scripts/write_sccache_stats.sh").count() != 3
    {
        return Err(
            "release workflow must configure the pinned compiler cache once for source, native, and desktop owners"
                .to_owned(),
        );
    }
    for required in [
        "continue-on-error: true",
        "disable_annotations: true",
        "Run compiler cache contract regression",
        "./scripts/test_sccache_contract.sh",
        "cache_epoch:",
        "candidate cache epoch must be a 1..32 character lowercase body-free token",
        "# cargo-deny does not compile product code, so keep this integrity gate isolated.",
        "          RUSTC_WRAPPER: \"\"",
        "canisend-v1-rust-1.97.0-x86_64-unknown-linux-gnu-debug-release-all-features-${{ needs.release-identity.outputs.cache_epoch }}",
        "cargo_profile: ${{ steps.identity.outputs.cargo_profile }}",
        "cargo build --profile ${{ needs.release-identity.outputs.cargo_profile }}",
        "cargo test --profile ${{ needs.release-identity.outputs.cargo_profile }}",
        "canisend-v1-rust-1.97.0-${{ matrix.target }}-${{ needs.release-identity.outputs.cargo_profile }}-cli-default-${{ needs.release-identity.outputs.cache_epoch }}",
        "canisend-v1-rust-1.97.0-aarch64-apple-darwin-${{ needs.release-identity.outputs.cargo_profile }}-cli-gui-default-${{ needs.release-identity.outputs.cache_epoch }}",
        "${{ runner.temp }}/sccache-stats/*.json",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "release workflow compiler cache is missing invariant `{required}`"
            ));
        }
    }
    if workflow
        .matches("${{ runner.temp }}/release-timing/*.json")
        .count()
        != 2
    {
        return Err(
            "release workflow must upload CLI and desktop timing evidence exactly once each"
                .to_owned(),
        );
    }

    let desktop_qualification_path =
        root.join(".github/workflows/desktop-platform-qualification.yml");
    let desktop_qualification = fs::read_to_string(&desktop_qualification_path)
        .map_err(|error| format!("desktop platform qualification workflow is missing: {error}"))?;
    for required in [
        "name: desktop-platform-qualification",
        "runs-on: windows-2025",
        "runs-on: ubuntu-22.04",
        "CARGO_PROFILE_RELEASE_OPT_LEVEL: z",
        "CARGO_PROFILE_RELEASE_LTO: fat",
        "Measure latest-template Windows profile matrix",
        "./scripts/measure_desktop_profile_matrix.ps1",
        "Measure latest-template Linux profile matrix",
        "./scripts/measure_desktop_profile_matrix.sh",
        "Build one-host Windows desktop without bundling",
        "Self-sign exact unified Windows host",
        "Bundle per-user NSIS and MSI standard installers",
        "Build one-host Linux desktop without bundling",
        "Bundle DEB, RPM, and separately budgeted AppImage",
        "Smoke GUI, renamed CLI, and MCP modes",
        "desktop size-record",
        "x86_64-pc-windows-msvc release msi",
        "x86_64-pc-windows-msvc release nsis",
        "x86_64-pc-windows-msvc release nsis-offline",
        "x86_64-unknown-linux-gnu release deb",
        "x86_64-unknown-linux-gnu release rpm",
        "x86_64-unknown-linux-gnu release appimage",
        "rpm2cpio",
        "rpm2cpio returned non-zero; validating the emitted CPIO archive",
        "test -s \"$rpm_archive\"",
        "--appimage-extract",
        "appimage_payload=\"$appimage_extract/squashfs-root\"",
        "\"$appimage_host\" version --json",
        "\"$appimage_host\" doctor --json",
        "\"$appimage_host\" mcp --help",
        "retention-days: 14",
    ] {
        if !desktop_qualification.contains(required) {
            return Err(format!(
                "desktop platform qualification is missing invariant `{required}`"
            ));
        }
    }
    for forbidden in [
        "contents: write",
        "packages: write",
        "releases: write",
        "git push",
        "$host =",
        "$Host =",
    ] {
        if desktop_qualification.contains(forbidden) {
            return Err(format!(
                "desktop platform qualification must remain nonpublishing; found `{forbidden}`"
            ));
        }
    }

    let timing_script = fs::read_to_string(root.join("scripts/write_native_release_timing.sh"))
        .map_err(|error| format!("native release timing writer is missing: {error}"))?;
    for required in [
        "canisend.native-release-timing/v1",
        "profile does not match the validated release stage",
        "profile: $profile",
        "workspace_suite_repeated_on_target: false",
        "authoritative_release_evidence: false",
    ] {
        if !timing_script.contains(required) {
            return Err(format!(
                "native release timing writer is missing invariant `{required}`"
            ));
        }
    }
    for required in [
        "scripts/test_native_release_timing.sh",
        "scripts/configure_sccache.sh",
        "scripts/write_sccache_stats.sh",
        "scripts/test_sccache_contract.sh",
        "docs/release/native-test-ownership.md",
    ] {
        if !root.join(required).is_file() {
            return Err(format!("native test ownership file is missing: {required}"));
        }
    }
    let sccache_configuration = fs::read_to_string(root.join("scripts/configure_sccache.sh"))
        .map_err(|error| format!("sccache configuration helper is missing: {error}"))?;
    for required in [
        "SCCACHE_GHA_ENABLED=true",
        "SCCACHE_IGNORE_SERVER_IO_ERROR=1",
        "RUSTC_WRAPPER=$tool",
        "CARGO_INCREMENTAL=0",
        "installation-or-server-unavailable",
        "continuing with ordinary Cargo compilation",
    ] {
        if !sccache_configuration.contains(required) {
            return Err(format!(
                "sccache fallback configuration is missing invariant `{required}`"
            ));
        }
    }
    let sccache_statistics = fs::read_to_string(root.join("scripts/write_sccache_stats.sh"))
        .map_err(|error| format!("sccache statistics writer is missing: {error}"))?;
    for required in [
        SCCACHE_STATS_SCHEMA,
        "compile_requests",
        "cache_hits",
        "cache_misses",
        "cache_errors",
        "hit_rate_percent",
        "version: \"v0.16.0\"",
        "time_saved_seconds: null",
        "cold-warm-candidate-comparison-required",
        "authoritative_release_evidence: false",
        "cache_hit_is_release_evidence: false",
        "fallback_preserves_build_command: true",
        "no_publication: true",
    ] {
        if !sccache_statistics.contains(required) {
            return Err(format!(
                "sccache statistics writer is missing invariant `{required}`"
            ));
        }
    }
    let scheduled_workflow =
        fs::read_to_string(root.join(".github/workflows/intel-gui-compile.yml"))
            .map_err(|error| format!("scheduled Intel GUI workflow is missing: {error}"))?;
    for required in [
        "schedule:",
        "workflow_dispatch:",
        "macos-15-intel",
        "cargo build --release --locked --target x86_64-apple-darwin",
        "canisend.scheduled-macos-gui-compilation/v1",
        "release_manifest_evidence: false",
        "archive_published: false",
        "native_runtime_qualified: false",
        "support_claim: false",
        "no_publication: true",
        sccache_action,
        "version: \"v0.16.0\"",
        "cache-targets: false",
        "continue-on-error: true",
        "disable_annotations: true",
        "canisend-v1-rust-1.97.0-x86_64-apple-darwin-release-gui-default",
        "./scripts/configure_sccache.sh",
        "./scripts/write_sccache_stats.sh",
        "${{ runner.temp }}/sccache-stats/*.json",
    ] {
        if !scheduled_workflow.contains(required) {
            return Err(format!(
                "scheduled Intel GUI workflow is missing invariant `{required}`"
            ));
        }
    }
    println!(
        "native test ownership: ok (one source suite, {} CLI targets, macOS ZIP+DMG plus scheduled Windows/Linux desktop packages, non-authoritative compiler cache)",
        policy_targets.len()
    );
    Ok(())
}

fn check_release_contract() -> Result<(), String> {
    let root = repository_root();
    let targets = release_targets()?;
    let expected = BTreeSet::from([
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "x86_64-pc-windows-msvc",
    ]);
    let actual = targets
        .iter()
        .map(|target| target.triple.as_str())
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!(
            "release target set differs: expected {expected:?}, found {actual:?}"
        ));
    }
    for required in [
        "release/KNOWN_LIMITATIONS.md",
        "release/ISSUE_COLLECTION.md",
        "release/RELEASE_NOTES.md",
        "release/beta-readiness.json",
        "release/beta-contract-freeze.json",
        "release/stage-transition-policy.json",
        "packaging/macos/AppIcon.icns",
        "scripts/stage_native_bundle.sh",
        "scripts/package_native_release.sh",
        "scripts/smoke_release_archive.sh",
        "scripts/smoke_agent_v4_mcp.sh",
        "scripts/smoke_host_v4.sh",
        "scripts/stage_macos_gui_app.sh",
        "scripts/package_macos_gui_release.sh",
        "scripts/smoke_macos_gui_dmg.sh",
        "scripts/smoke_macos_gui_release_archive.sh",
        "scripts/verify_macos_gui_app.sh",
        "scripts/download_github_draft_asset.sh",
    ] {
        let path = root.join(required);
        if !path.is_file() {
            return Err(format!("release contract file is missing: {required}"));
        }
    }
    let workflow_path = root.join(".github/workflows/release.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("release workflow is missing: {error}"))?;
    for target in &targets {
        for required in [
            target.triple.as_str(),
            target.runner.as_str(),
            target.executable.as_str(),
            target.archive.as_str(),
        ] {
            if !workflow.contains(required) {
                return Err(format!(
                    "release workflow does not reference `{required}` for {}",
                    target.triple
                ));
            }
        }
    }
    for required in [
        "release validate-tag",
        "release assemble",
        "release verify",
        "release verify-candidate",
        "attest-build-provenance",
        "locate-release-candidate",
        "promote-release-candidate",
        "canisend.release-candidate-promotion/v1",
        "recompiled_during_promotion: false",
        "cancel-in-progress: false",
        "head_sha",
        "--signer-workflow",
        "--source-digest",
        "build-macos-gui-archive",
        "canisend.macos-gui-compilation/v1",
        "x86_64-apple-darwin-gui-compilation.json",
        "package_macos_gui_release.sh",
        "smoke_macos_gui_dmg.sh",
        "smoke_macos_gui_release_archive.sh",
        "CanISend-$version-aarch64-apple-darwin.dmg",
        "aarch64-apple-darwin-dmg-qualification.json",
        "desktop-macos-aarch64",
        "stage-draft-release-assets",
        "Stage exact draft bytes for read-only native smoke jobs",
        "verify-draft-release-assets",
        "promote_existing_tag:",
        "existing-tag promotion requires an annotated tag",
        "existing-tag promotion must run from refs/heads/main",
        "download_github_draft_asset.sh",
        "publish-verified-release",
        "verify-published-release",
        "gh attestation verify",
        "SHA256SUMS",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "release workflow is missing required gate `{required}`"
            ));
        }
    }
    let version = env!("CARGO_PKG_VERSION");
    let release_line = Version::parse(version)
        .map_err(|error| format!("workspace version is invalid SemVer: {error}"))?;
    let expected_tag_pattern = format!(
        "v{}.{}.{}*",
        release_line.major, release_line.minor, release_line.patch
    );
    let expected_default = expected_release_workflow_default(&release_line)?;
    if !workflow.contains(&expected_tag_pattern)
        || !workflow.contains(&format!("default: \"{expected_default}\""))
    {
        return Err(format!(
            "release workflow must listen to `{expected_tag_pattern}` and default to `{expected_default}`"
        ));
    }
    let recovery_workflow =
        fs::read_to_string(root.join(".github/workflows/finalize-verified-release.yml"))
            .map_err(|error| format!("release recovery workflow is missing: {error}"))?;
    for required in [
        "locate-release-candidate",
        "promote-release-candidate",
        "candidate_promoted_without_recompile: true",
        "--signer-workflow",
        "--source-digest",
    ] {
        if !recovery_workflow.contains(required) {
            return Err(format!(
                "release recovery workflow is missing required gate `{required}`"
            ));
        }
    }
    println!("release contract: ok ({} targets)", targets.len());
    Ok(())
}

fn expected_release_workflow_default(version: &Version) -> Result<String, String> {
    if (version.major, version.minor, version.patch) == (1, 0, 0)
        && ReleaseStage::from_version(version) == Ok(ReleaseStage::Alpha)
        && prerelease_iteration(version, "alpha")? < 6
    {
        Ok("v1.0.0-alpha.6".to_owned())
    } else {
        Ok(format!("v{version}"))
    }
}

fn check_cli_gui_parity() -> Result<(), String> {
    let path = repository_root().join("docs/contracts/cli-gui-parity-v1.json");
    let document: Value = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("CLI/GUI parity manifest is missing: {error}"))?,
    )
    .map_err(|error| format!("CLI/GUI parity manifest is invalid JSON: {error}"))?;
    if document.get("format").and_then(Value::as_str) != Some("canisend.cli-gui-parity/v1") {
        return Err("CLI/GUI parity manifest format must be canisend.cli-gui-parity/v1".to_owned());
    }
    let alpha_scope = document
        .get("alpha_scope")
        .and_then(Value::as_object)
        .ok_or_else(|| "CLI/GUI parity manifest must define alpha_scope".to_owned())?;
    if alpha_scope.get("version").and_then(Value::as_str) != Some(env!("CARGO_PKG_VERSION")) {
        return Err(format!(
            "CLI/GUI parity Alpha scope must equal workspace version {}",
            env!("CARGO_PKG_VERSION")
        ));
    }
    if alpha_scope
        .get("planned_statuses_allowed")
        .and_then(Value::as_bool)
        != Some(false)
    {
        return Err(
            "CLI/GUI parity Alpha scope must prohibit unresolved planned statuses".to_owned(),
        );
    }
    let entries = document
        .get("entries")
        .and_then(Value::as_array)
        .ok_or_else(|| "CLI/GUI parity manifest entries must be an array".to_owned())?;
    let required_implemented = BTreeSet::from([
        "product.version",
        "product.doctor",
        "product.update.check",
        "workspace.init",
        "workspace.status",
        "workspace.check",
        "workspace.backup",
        "workspace.restore",
        "workspace.repair",
        "job.import",
        "workflow.start",
        "workflow.begin",
        "workflow.complete",
        "workflow.rerun",
        "cli.install.status",
        "cli.install",
        "cli.uninstall",
        "profile.*",
        "discovery.*",
        "criteria.*",
        "match.*",
        "plan.*",
        "agent.*",
    ]);
    let mut operations = BTreeSet::new();
    let mut implemented = BTreeSet::new();
    let mut deferred = 0_usize;
    for entry in entries {
        let operation = entry
            .get("operation")
            .and_then(Value::as_str)
            .ok_or_else(|| "CLI/GUI parity entry is missing operation".to_owned())?;
        if !operations.insert(operation) {
            return Err(format!("duplicate CLI/GUI parity operation: {operation}"));
        }
        let status = entry
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("CLI/GUI parity entry {operation} is missing status"))?;
        match status {
            "implemented" => {
                implemented.insert(operation);
            }
            "deferred-beta" => deferred += 1,
            other => {
                return Err(format!(
                    "CLI/GUI parity entry {operation} has unresolved or unknown Alpha status {other}"
                ));
            }
        }
    }
    if !required_implemented.is_subset(&implemented) {
        let missing = required_implemented
            .difference(&implemented)
            .copied()
            .collect::<Vec<_>>();
        return Err(format!(
            "CLI/GUI Alpha parity is missing implemented operations: {missing:?}"
        ));
    }
    if deferred == 0 {
        println!(
            "CLI/GUI parity: ok ({} implemented, no deferred Beta operations)",
            implemented.len()
        );
    } else {
        println!(
            "CLI/GUI parity: ok ({} implemented, {deferred} deferred to Beta)",
            implemented.len()
        );
    }
    Ok(())
}

fn check_workspace_dependency_graph() -> Result<(), String> {
    let root = repository_root();
    check_current_architecture_decisions(&root)?;
    let policy_path =
        root.join("docs/architecture/rust-native/workspace-dependency-policy-v1.json");
    let policy: Value = serde_json::from_slice(
        &fs::read(&policy_path)
            .map_err(|error| format!("workspace dependency policy is missing: {error}"))?,
    )
    .map_err(|error| format!("workspace dependency policy is invalid JSON: {error}"))?;
    let (packages, edges) = current_workspace_dependency_facts(&root)?;
    let summary = validate_workspace_dependency_policy(
        &policy,
        &packages,
        &edges,
        OffsetDateTime::now_utc().date(),
    )?;
    println!(
        "workspace dependency graph: ok ({} product + {} automation crates, {} actual / {} target edges, {} temporary exception)",
        summary.product_crates,
        summary.automation_crates,
        summary.actual_edges,
        summary.target_edges,
        summary.temporary_exceptions
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WorkspaceDependencySummary {
    product_crates: usize,
    automation_crates: usize,
    actual_edges: usize,
    target_edges: usize,
    temporary_exceptions: usize,
}

fn check_current_architecture_decisions(root: &Path) -> Result<(), String> {
    let decisions = root.join("docs/architecture/rust-native/decisions");
    let old_workspace = fs::read_to_string(decisions.join("0002-cargo-workspace-boundaries.md"))
        .map_err(|error| format!("ADR-RN-0002 is missing: {error}"))?;
    if !old_workspace.contains("**Status:** Superseded by ADR-RN-0019") {
        return Err("ADR-RN-0002 must be superseded by ADR-RN-0019".to_owned());
    }
    let old_desktop = fs::read_to_string(decisions.join("0013-native-desktop-adapter.md"))
        .map_err(|error| format!("ADR-RN-0013 is missing: {error}"))?;
    if !old_desktop.contains("**Status:** Superseded by ADR-RN-0015") {
        return Err("ADR-RN-0013 must be superseded by ADR-RN-0015".to_owned());
    }
    let tauri = fs::read_to_string(decisions.join("0015-replace-egui-with-tauri-svelte.md"))
        .map_err(|error| format!("ADR-RN-0015 is missing: {error}"))?;
    if !tauri.contains("Status: Accepted") {
        return Err("ADR-RN-0015 must remain Accepted".to_owned());
    }
    let current = fs::read_to_string(decisions.join("0019-current-product-graph.md"))
        .map_err(|error| format!("ADR-RN-0019 is missing: {error}"))?;
    for required in [
        "**Status:** Accepted",
        "## Actual graph",
        "## Target graph",
        "canisend-mcp",
        "canisend-gui",
        "Tauri 2",
        "Svelte 5",
        "unified host",
        "canisend-store -> canisend-io",
        "2026-08-10",
        "2026-08-17",
        "workspace-dependency-policy-v1.json",
    ] {
        if !current.contains(required) {
            return Err(format!(
                "ADR-RN-0019 is missing required authority `{required}`"
            ));
        }
    }
    Ok(())
}

fn current_workspace_dependency_facts(
    root: &Path,
) -> Result<(BTreeSet<String>, Vec<Value>), String> {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--locked",
            "--offline",
            "--format-version",
            "1",
            "--no-deps",
        ])
        .current_dir(root)
        .output()
        .map_err(|error| format!("could not run locked Cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "locked Cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Cargo metadata output is invalid JSON: {error}"))?;
    workspace_dependency_facts_from_metadata(&metadata)
}

fn workspace_dependency_facts_from_metadata(
    metadata: &Value,
) -> Result<(BTreeSet<String>, Vec<Value>), String> {
    let member_ids = metadata["workspace_members"]
        .as_array()
        .ok_or_else(|| "Cargo metadata has no workspace_members array".to_owned())?
        .iter()
        .map(|member| {
            member
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "Cargo workspace member ID must be a string".to_owned())
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| "Cargo metadata has no packages array".to_owned())?;
    let workspace_packages = packages
        .iter()
        .filter(|package| {
            package["id"]
                .as_str()
                .is_some_and(|id| member_ids.contains(id))
        })
        .collect::<Vec<_>>();
    if workspace_packages.len() != member_ids.len() {
        return Err("Cargo metadata does not describe every workspace member".to_owned());
    }
    let package_names = workspace_packages
        .iter()
        .map(|package| {
            package["name"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "Cargo workspace package has no name".to_owned())
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if package_names.len() != workspace_packages.len() {
        return Err("Cargo workspace package names must be unique".to_owned());
    }

    let mut edges = Vec::new();
    for package in workspace_packages {
        let from = package["name"].as_str().expect("validated package name");
        let dependencies = package["dependencies"]
            .as_array()
            .ok_or_else(|| format!("Cargo package {from} has no dependencies array"))?;
        for dependency in dependencies {
            let Some(to) = dependency["name"].as_str() else {
                return Err(format!("Cargo dependency of {from} has no name"));
            };
            if dependency["path"].is_null() || !package_names.contains(to) {
                continue;
            }
            let edge = json!({
                "from": from,
                "to": to,
                "kind": dependency["kind"].as_str().unwrap_or("normal"),
                "target": dependency["target"].clone(),
                "optional": dependency["optional"].clone(),
                "default_features": dependency["uses_default_features"].clone(),
                "features": dependency["features"].clone(),
                "rename": dependency["rename"].clone(),
            });
            edges.push(normalize_dependency_edge(&edge)?);
        }
    }
    sort_json_values(&mut edges)?;
    Ok((package_names, edges))
}

fn validate_workspace_dependency_policy(
    policy: &Value,
    actual_packages: &BTreeSet<String>,
    actual_edges: &[Value],
    today: Date,
) -> Result<WorkspaceDependencySummary, String> {
    if policy["format"].as_str() != Some(WORKSPACE_DEPENDENCY_POLICY_SCHEMA) {
        return Err(format!(
            "workspace dependency policy format must be {WORKSPACE_DEPENDENCY_POLICY_SCHEMA}"
        ));
    }
    if policy["adr"].as_str() != Some("ADR-RN-0019") {
        return Err("workspace dependency policy must be owned by ADR-RN-0019".to_owned());
    }
    let reviewed_at = parse_policy_date(
        policy["reviewed_at"]
            .as_str()
            .ok_or_else(|| "workspace dependency policy needs reviewed_at".to_owned())?,
        "reviewed_at",
    )?;
    if reviewed_at > today {
        return Err(format!(
            "workspace dependency policy review date is in the future: {reviewed_at}"
        ));
    }
    let product_crates = string_set(&policy["product_crates"], "product_crates")?;
    let automation_crates = string_set(&policy["automation_crates"], "automation_crates")?;
    if product_crates.len() != 9 || automation_crates != BTreeSet::from(["xtask".to_owned()]) {
        return Err(
            "dependency policy must classify nine product crates and only xtask automation"
                .to_owned(),
        );
    }
    let classified = product_crates
        .union(&automation_crates)
        .cloned()
        .collect::<BTreeSet<_>>();
    if &classified != actual_packages {
        return Err(format!(
            "workspace crate classification drifted: expected {classified:?}, found {actual_packages:?}"
        ));
    }
    let dimensions = string_set(
        &policy["covered_edge_dimensions"],
        "covered_edge_dimensions",
    )?;
    let required_dimensions = BTreeSet::from([
        "normal".to_owned(),
        "dev".to_owned(),
        "build".to_owned(),
        "target-specific".to_owned(),
        "optional".to_owned(),
        "feature-enabled".to_owned(),
    ]);
    if dimensions != required_dimensions {
        return Err(format!(
            "dependency policy edge dimensions must be {required_dimensions:?}"
        ));
    }

    let allowed_actual = normalized_edge_array(&policy["actual_edges"], "actual_edges")?;
    let normalized_actual = actual_edges
        .iter()
        .map(normalize_dependency_edge)
        .collect::<Result<Vec<_>, _>>()?;
    compare_dependency_edges(&allowed_actual, &normalized_actual, "actual graph")?;
    let target = normalized_edge_array(&policy["target_edges"], "target_edges")?;
    validate_dependency_graph_edges(&allowed_actual, actual_packages, "actual graph")?;
    validate_dependency_graph_edges(&target, actual_packages, "target graph")?;

    let exceptions = policy["temporary_exceptions"]
        .as_array()
        .ok_or_else(|| "temporary_exceptions must be an array".to_owned())?;
    let mut exception_edges = Vec::new();
    for exception in exceptions {
        for field in [
            "owner",
            "rationale",
            "review_by",
            "expires_on",
            "removal_condition",
            "tracking",
        ] {
            if exception[field]
                .as_str()
                .is_none_or(|value| value.trim().is_empty())
            {
                return Err(format!("temporary dependency exception is missing {field}"));
            }
        }
        let edge = normalize_dependency_edge(&exception["edge"])?;
        let review_by = parse_policy_date(
            exception["review_by"]
                .as_str()
                .expect("validated review_by"),
            "review_by",
        )?;
        let expires_on = parse_policy_date(
            exception["expires_on"]
                .as_str()
                .expect("validated expires_on"),
            "expires_on",
        )?;
        if expires_on < review_by {
            return Err("dependency exception expires before its review date".to_owned());
        }
        if review_by <= reviewed_at {
            return Err(
                "dependency exception review must follow the policy review date".to_owned(),
            );
        }
        if today > review_by {
            return Err(format!(
                "dependency exception review is overdue for {} -> {} (review_by {review_by})",
                edge["from"].as_str().expect("normalized edge"),
                edge["to"].as_str().expect("normalized edge")
            ));
        }
        if today > expires_on {
            return Err(format!(
                "dependency exception expired for {} -> {} on {expires_on}",
                edge["from"].as_str().expect("normalized edge"),
                edge["to"].as_str().expect("normalized edge")
            ));
        }
        exception_edges.push(edge);
    }
    if exception_edges.len() != 1
        || exception_edges[0]["from"] != "canisend-store"
        || exception_edges[0]["to"] != "canisend-io"
        || exception_edges[0]["kind"] != "normal"
    {
        return Err(
            "the only temporary dependency exception must be canisend-store -> canisend-io normal"
                .to_owned(),
        );
    }

    let planned_removals = policy["planned_removals"]
        .as_array()
        .ok_or_else(|| "planned_removals must be an array".to_owned())?;
    let mut removal_edges = Vec::new();
    for removal in planned_removals {
        if removal["tracking"]
            .as_str()
            .is_none_or(|value| value.trim().is_empty())
            || removal["rationale"]
                .as_str()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err("planned dependency removal needs tracking and rationale".to_owned());
        }
        removal_edges.push(normalize_dependency_edge(&removal["edge"])?);
    }
    let planned_additions =
        normalized_edge_array(&policy["planned_additions"], "planned_additions")?;

    let actual_keys = dependency_edge_keys(&allowed_actual)?;
    let target_keys = dependency_edge_keys(&target)?;
    let removed = actual_keys
        .difference(&target_keys)
        .cloned()
        .collect::<BTreeSet<_>>();
    let documented_removed = exception_edges
        .iter()
        .chain(&removal_edges)
        .map(dependency_edge_key)
        .collect::<Result<BTreeSet<_>, _>>()?;
    if removed != documented_removed {
        return Err(format!(
            "actual-to-target removals drifted: graph={removed:?}, documented={documented_removed:?}"
        ));
    }
    let added = target_keys
        .difference(&actual_keys)
        .cloned()
        .collect::<BTreeSet<_>>();
    let documented_added = dependency_edge_keys(&planned_additions)?;
    if added != documented_added {
        return Err(format!(
            "actual-to-target additions drifted: graph={added:?}, documented={documented_added:?}"
        ));
    }

    Ok(WorkspaceDependencySummary {
        product_crates: product_crates.len(),
        automation_crates: automation_crates.len(),
        actual_edges: allowed_actual.len(),
        target_edges: target.len(),
        temporary_exceptions: exceptions.len(),
    })
}

fn normalize_dependency_edge(value: &Value) -> Result<Value, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "dependency edge must be an object".to_owned())?;
    let expected_fields = BTreeSet::from([
        "from",
        "to",
        "kind",
        "target",
        "optional",
        "default_features",
        "features",
        "rename",
    ]);
    let actual_fields = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    if actual_fields != expected_fields {
        return Err(format!(
            "dependency edge fields must be {expected_fields:?}, found {actual_fields:?}"
        ));
    }
    let from = edge_string(value, "from")?;
    let to = edge_string(value, "to")?;
    if from == to {
        return Err(format!(
            "dependency edge cannot be self-referential: {from}"
        ));
    }
    let kind = edge_string(value, "kind")?;
    if !matches!(kind, "normal" | "dev" | "build") {
        return Err(format!("unknown dependency edge kind {kind}"));
    }
    let target = match &value["target"] {
        Value::Null => Value::Null,
        Value::String(target) if !target.trim().is_empty() => Value::String(target.clone()),
        _ => return Err("dependency edge target must be null or a non-empty string".to_owned()),
    };
    let optional = value["optional"]
        .as_bool()
        .ok_or_else(|| "dependency edge optional must be boolean".to_owned())?;
    let default_features = value["default_features"]
        .as_bool()
        .ok_or_else(|| "dependency edge default_features must be boolean".to_owned())?;
    let features = string_set(&value["features"], "dependency edge features")?;
    let rename = match &value["rename"] {
        Value::Null => Value::Null,
        Value::String(rename) if !rename.trim().is_empty() => Value::String(rename.clone()),
        _ => return Err("dependency edge rename must be null or a non-empty string".to_owned()),
    };
    Ok(json!({
        "from": from,
        "to": to,
        "kind": kind,
        "target": target,
        "optional": optional,
        "default_features": default_features,
        "features": features.into_iter().collect::<Vec<_>>(),
        "rename": rename,
    }))
}

fn normalized_edge_array(value: &Value, context: &str) -> Result<Vec<Value>, String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{context} must be an array"))?;
    let mut normalized = values
        .iter()
        .map(normalize_dependency_edge)
        .collect::<Result<Vec<_>, _>>()?;
    let keys = dependency_edge_keys(&normalized)?;
    if keys.len() != normalized.len() {
        return Err(format!("{context} contains a duplicate edge"));
    }
    sort_json_values(&mut normalized)?;
    Ok(normalized)
}

fn validate_dependency_graph_edges(
    edges: &[Value],
    packages: &BTreeSet<String>,
    context: &str,
) -> Result<(), String> {
    let mut indegree = packages
        .iter()
        .map(|package| (package.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<String, BTreeSet<String>>::new();
    for edge in edges {
        let from = edge["from"].as_str().expect("normalized edge");
        let to = edge["to"].as_str().expect("normalized edge");
        if !packages.contains(from) || !packages.contains(to) {
            return Err(format!(
                "{context} contains unknown crate edge {from} -> {to}"
            ));
        }
        if edge["kind"] == "dev" {
            continue;
        }
        if outgoing
            .entry(from.to_owned())
            .or_default()
            .insert(to.to_owned())
        {
            *indegree.get_mut(to).expect("known dependency crate") += 1;
        }
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(package, degree)| (*degree == 0).then_some(package.clone()))
        .collect::<VecDeque<_>>();
    let mut visited = 0_usize;
    while let Some(package) = ready.pop_front() {
        visited += 1;
        if let Some(dependencies) = outgoing.get(&package) {
            for dependency in dependencies {
                let degree = indegree.get_mut(dependency).expect("known dependency");
                *degree -= 1;
                if *degree == 0 {
                    ready.push_back(dependency.clone());
                }
            }
        }
    }
    if visited != packages.len() {
        return Err(format!("{context} contains a non-dev dependency cycle"));
    }
    Ok(())
}

fn compare_dependency_edges(
    expected: &[Value],
    actual: &[Value],
    context: &str,
) -> Result<(), String> {
    let expected = dependency_edge_keys(expected)?;
    let actual = dependency_edge_keys(actual)?;
    if expected == actual {
        return Ok(());
    }
    let unapproved = actual.difference(&expected).cloned().collect::<Vec<_>>();
    let removed_or_reclassified = expected.difference(&actual).cloned().collect::<Vec<_>>();
    Err(format!(
        "{context} drifted: unapproved={unapproved:?}, removed_or_reclassified={removed_or_reclassified:?}"
    ))
}

fn dependency_edge_keys(edges: &[Value]) -> Result<BTreeSet<String>, String> {
    edges.iter().map(dependency_edge_key).collect()
}

fn dependency_edge_key(edge: &Value) -> Result<String, String> {
    serde_json::to_string(&normalize_dependency_edge(edge)?)
        .map_err(|error| format!("cannot serialize dependency edge: {error}"))
}

fn sort_json_values(values: &mut [Value]) -> Result<(), String> {
    let mut keyed = values
        .iter()
        .cloned()
        .map(|value| Ok((dependency_edge_key(&value)?, value)))
        .collect::<Result<Vec<_>, String>>()?;
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    for (slot, (_, value)) in values.iter_mut().zip(keyed) {
        *slot = value;
    }
    Ok(())
}

fn string_set(value: &Value, context: &str) -> Result<BTreeSet<String>, String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{context} must be an array"))?;
    let set = values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("{context} entries must be non-empty strings"))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if set.len() != values.len() {
        return Err(format!("{context} contains a duplicate"));
    }
    Ok(set)
}

fn edge_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value[field]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("dependency edge {field} must be a non-empty string"))
}

fn parse_policy_date(value: &str, context: &str) -> Result<Date, String> {
    let mut parts = value.split('-');
    let year = parts
        .next()
        .and_then(|part| part.parse::<i32>().ok())
        .ok_or_else(|| format!("{context} must be YYYY-MM-DD"))?;
    let month = parts
        .next()
        .and_then(|part| part.parse::<u8>().ok())
        .and_then(|month| Month::try_from(month).ok())
        .ok_or_else(|| format!("{context} must be YYYY-MM-DD"))?;
    let day = parts
        .next()
        .and_then(|part| part.parse::<u8>().ok())
        .ok_or_else(|| format!("{context} must be YYYY-MM-DD"))?;
    if parts.next().is_some() {
        return Err(format!("{context} must be YYYY-MM-DD"));
    }
    Date::from_calendar_date(year, month, day)
        .map_err(|error| format!("{context} is invalid: {error}"))
}

fn check_approval_broker() -> Result<(), String> {
    let root = repository_root();
    let read = |relative: &str| {
        fs::read_to_string(root.join(relative))
            .map_err(|error| format!("could not read {relative}: {error}"))
    };
    let app = read("crates/canisend-app/src/approval.rs")?;
    let mcp = read("crates/canisend-mcp/src/lib.rs")?;
    let desktop_state = read("crates/canisend-desktop/src/approval.rs")?;
    let desktop_host = read("crates/canisend-desktop/src/lib.rs")?;
    let desktop_job = read("crates/canisend-desktop/src/job_intake.rs")?;
    let desktop_discovery = read("crates/canisend-desktop/src/discovery.rs")?;
    let desktop_workflow = read("crates/canisend-desktop/src/workflow.rs")?;
    let desktop_application = read("crates/canisend-desktop/src/application_intake.rs")?;
    let application_association = read("crates/canisend-app/src/association_v4.rs")?;
    let desktop_association = read("crates/canisend-desktop/src/association_v4.rs")?;
    let application_mutations = read("crates/canisend-app/src/application_mutations_v4.rs")?;
    let desktop_mutations = read("crates/canisend-desktop/src/application_mutations_v4.rs")?;
    let bridge = read("apps/canisend-desktop/src/lib/bridge.ts")?;
    validate_approval_broker_sources(ApprovalBrokerSources {
        app: &app,
        mcp: &mcp,
        desktop_state: &desktop_state,
        desktop_host: &desktop_host,
        associations: [&application_association, &desktop_association],
        mutations: [&application_mutations, &desktop_mutations],
        desktop_families: [
            &desktop_job,
            &desktop_discovery,
            &desktop_workflow,
            &desktop_application,
        ],
        bridge: &bridge,
    })?;
    println!(
        "approval broker: ok (10-minute monotonic TTL, 16-entry bound, 7 guarded adapter families)"
    );
    Ok(())
}

struct ApprovalBrokerSources<'a> {
    app: &'a str,
    mcp: &'a str,
    desktop_state: &'a str,
    desktop_host: &'a str,
    associations: [&'a str; 2],
    mutations: [&'a str; 2],
    desktop_families: [&'a str; 4],
    bridge: &'a str,
}

fn validate_approval_broker_sources(sources: ApprovalBrokerSources<'_>) -> Result<(), String> {
    let ApprovalBrokerSources {
        app,
        mcp,
        desktop_state,
        desktop_host,
        associations: [application_association, desktop_association],
        mutations: [application_mutations, desktop_mutations],
        desktop_families,
        bridge,
    } = sources;
    for required in [
        "pub const APPROVAL_DEFAULT_CAPACITY: usize = 16",
        "Duration::from_secs(10 * 60)",
        "getrandom::fill(destination)",
        "pub enum ApprovalDisposition",
        "RestoreSameApproval",
        "fn start_sweeper",
        "CapacityFull",
        "ApprovalSourceVersion",
    ] {
        if !app.contains(required) {
            return Err(format!(
                "shared approval broker is missing required invariant source: {required}"
            ));
        }
    }
    for required in [
        "AssociationApprovalBrokerV4",
        "preview_token",
        "approved: bool",
        "confirmed_private_read",
        "destructive_hint = true",
    ] {
        if !mcp.contains(required) {
            return Err(format!(
                "MCP association approval adapter is missing boundary evidence: {required}"
            ));
        }
    }
    for required in [
        "ApprovalBroker<PendingAssociationApprovalV4>",
        "approval_disposition_for_application_error",
        "ApprovalSourceVersion::RevisionAndSnapshot",
        "remaining_ttl_seconds",
        "expires_at_unix_ms",
    ] {
        if !application_association.contains(required) {
            return Err(format!(
                "Application association approval facade is missing invariant evidence: {required}"
            ));
        }
    }
    for required in [
        "ApprovalBroker<DesktopPendingApproval>",
        "DesktopPendingApproval",
        "DesktopApprovalStore",
    ] {
        if !desktop_state.contains(required) {
            return Err(format!(
                "desktop approval state is missing shared broker evidence: {required}"
            ));
        }
    }
    if desktop_host
        .matches("DesktopApprovalStore::default()")
        .count()
        != 1
    {
        return Err("desktop host must manage exactly one shared DesktopApprovalStore".to_owned());
    }
    if desktop_host
        .matches("AssociationApprovalBrokerV4::default()")
        .count()
        != 1
    {
        return Err("desktop host must manage exactly one AssociationApprovalBrokerV4".to_owned());
    }
    if desktop_host
        .matches("ApplicationMutationApprovalBrokerV4::default()")
        .count()
        != 1
    {
        return Err(
            "desktop host must manage exactly one ApplicationMutationApprovalBrokerV4".to_owned(),
        );
    }
    for (index, family) in desktop_families.into_iter().enumerate() {
        for required in [
            "tauri::State<'_, DesktopApprovalStore>",
            "remaining_ttl_seconds",
            "expires_at_unix_ms",
        ] {
            if !family.contains(required) {
                return Err(format!(
                    "desktop preview family {} is missing shared broker evidence: {required}",
                    index + 1
                ));
            }
        }
    }
    for required in [
        "tauri::State<'_, AssociationApprovalBrokerV4>",
        "preview_token",
        "approved",
    ] {
        if !desktop_association.contains(required) {
            return Err(format!(
                "desktop association approval family is missing broker evidence: {required}"
            ));
        }
    }
    for required in [
        "ApprovalBroker<PendingApplicationMutationV4>",
        "ApprovalSourceVersion::RevisionAndSnapshot",
        "remaining_ttl_seconds",
        "expires_at_unix_ms",
    ] {
        if !application_mutations.contains(required) {
            return Err(format!(
                "Application mutation approval facade is missing invariant evidence: {required}"
            ));
        }
    }
    for required in [
        "tauri::State<'_, ApplicationMutationApprovalBrokerV4>",
        "preview_token",
        "approved",
    ] {
        if !desktop_mutations.contains(required) {
            return Err(format!(
                "desktop Application mutation family is missing broker evidence: {required}"
            ));
        }
    }
    if bridge.matches("remaining_ttl_seconds: number").count() < 7
        || bridge.matches("expires_at_unix_ms: number").count() < 7
    {
        return Err(
            "desktop bridge must expose expiry metadata for all seven preview read models"
                .to_owned(),
        );
    }
    let adapter_sources = [
        mcp,
        desktop_state,
        desktop_host,
        application_association,
        desktop_association,
        application_mutations,
        desktop_mutations,
    ]
    .into_iter()
    .chain(desktop_families)
    .collect::<Vec<_>>()
    .join("\n");
    for forbidden in [
        "struct MutationPreviewStore",
        "struct JobIntakePreviewStore",
        "struct WorkflowPreviewStore",
        "struct DiscoveryPreviewStore",
        "struct ApplicationIntakePreviewStore",
        "mcp-preview-",
        "job-intake-preview-",
        "workflow-preview-",
        "discovery-preview-",
        "application-intake-preview-",
    ] {
        if adapter_sources.contains(forbidden) {
            return Err(format!(
                "duplicated or predictable adapter approval state returned: {forbidden}"
            ));
        }
    }
    Ok(())
}

#[derive(Debug)]
struct SemanticParityReport {
    shared_operations: usize,
    preview_pairs: usize,
    read_families: usize,
    qualified_bindings: usize,
    uncovered_bindings: Vec<Value>,
}

fn check_semantic_parity() -> Result<(), String> {
    let root = repository_root();
    let policy_path = root.join("crates/canisend-contracts/semantic-parity-v1.json");
    let policy: Value = serde_json::from_str(
        &fs::read_to_string(&policy_path)
            .map_err(|error| format!("could not read {}: {error}", policy_path.display()))?,
    )
    .map_err(|error| format!("semantic parity policy is invalid JSON: {error}"))?;
    let registry = OperationRegistry::built_in()
        .map_err(|error| format!("typed operation registry is invalid: {error}"))?;
    let report = validate_semantic_parity_policy(&policy, &registry, &root)?;
    println!(
        "semantic parity: ok ({} shared operations, {} preview/commit pairs, {} read families, {} qualified / {} explicitly uncovered bindings)",
        report.shared_operations,
        report.preview_pairs,
        report.read_families,
        report.qualified_bindings,
        report.uncovered_bindings.len()
    );
    Ok(())
}

fn list_uncovered_semantic_bindings() -> Result<(), String> {
    let root = repository_root();
    let policy_path = root.join("crates/canisend-contracts/semantic-parity-v1.json");
    let policy: Value = serde_json::from_str(
        &fs::read_to_string(&policy_path)
            .map_err(|error| format!("could not read {}: {error}", policy_path.display()))?,
    )
    .map_err(|error| format!("semantic parity policy is invalid JSON: {error}"))?;
    let registry = OperationRegistry::built_in()
        .map_err(|error| format!("typed operation registry is invalid: {error}"))?;
    let report = validate_semantic_parity_policy(&policy, &registry, &root)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "format": "canisend.semantic-parity-uncovered/v1",
            "operation_registry_format": registry.format,
            "bindings": report.uncovered_bindings,
        }))
        .map_err(|error| format!("could not serialize uncovered semantic bindings: {error}"))?
    );
    Ok(())
}

fn validate_semantic_parity_policy(
    policy: &Value,
    registry: &OperationRegistry,
    root: &Path,
) -> Result<SemanticParityReport, String> {
    let policy_object = policy
        .as_object()
        .ok_or_else(|| "semantic parity policy must be an object".to_owned())?;
    let expected_fields = BTreeSet::from([
        "format",
        "version",
        "operation_registry_format",
        "required_outcomes",
        "fixtures",
        "shared_operations",
        "pack_surface_cases",
        "revision_bound_operations",
        "preview_commit_pairs",
        "read_families",
        "uncovered_policy",
    ]);
    let actual_fields = policy_object
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if actual_fields != expected_fields {
        return Err(format!(
            "semantic parity policy fields must be {expected_fields:?}, found {actual_fields:?}"
        ));
    }
    if policy["format"] != SEMANTIC_PARITY_SCHEMA {
        return Err(format!(
            "semantic parity format must be {SEMANTIC_PARITY_SCHEMA}"
        ));
    }
    if policy["version"].as_u64() != Some(1) {
        return Err("semantic parity version must be 1".to_owned());
    }
    if policy["operation_registry_format"] != registry.format {
        return Err("semantic parity policy targets a different operation registry".to_owned());
    }

    let expected_outcomes = BTreeSet::from([
        "success".to_owned(),
        "stale".to_owned(),
        "replay".to_owned(),
        "wrong-pack".to_owned(),
        "wrong-context".to_owned(),
        "no-mutation".to_owned(),
        "recovery".to_owned(),
    ]);
    let required_outcomes = string_set(
        &policy["required_outcomes"],
        "semantic parity required outcomes",
    )?;
    if required_outcomes != expected_outcomes {
        return Err(format!(
            "semantic parity outcomes drifted: expected {expected_outcomes:?}, found {required_outcomes:?}"
        ));
    }

    let fixture_values = semantic_array(policy, "fixtures")?;
    let mut fixtures = BTreeSet::new();
    for fixture in fixture_values {
        semantic_exact_fields(fixture, "fixture", &["id", "path", "marker"])?;
        let id = semantic_string(fixture, "id", "fixture")?;
        let path = semantic_string(fixture, "path", "fixture")?;
        let marker = semantic_string(fixture, "marker", "fixture")?;
        if !fixtures.insert(id.to_owned()) {
            return Err(format!("duplicate semantic parity fixture {id}"));
        }
        let relative = Path::new(path);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(format!("semantic fixture {id} has unsafe path {path}"));
        }
        let source = fs::read_to_string(root.join(relative))
            .map_err(|error| format!("could not read semantic fixture {id} at {path}: {error}"))?;
        if !source.contains(marker) {
            return Err(format!(
                "semantic fixture {id} marker is missing from {path}: {marker}"
            ));
        }
    }
    if fixtures.is_empty() {
        return Err("semantic parity fixture registry cannot be empty".to_owned());
    }

    let resolved = registry
        .resolved_bindings()
        .map_err(|error| format!("cannot resolve operation bindings: {error}"))?;
    let known_operations = registry
        .operations
        .iter()
        .map(|operation| operation.id.as_str().to_owned())
        .chain(
            registry
                .compatibility_aliases
                .iter()
                .map(|operation| operation.id.as_str().to_owned()),
        )
        .chain(
            resolved
                .iter()
                .map(|binding| binding.operation.as_str().to_owned()),
        )
        .collect::<BTreeSet<_>>();

    let expected_shared = registry
        .operations
        .iter()
        .filter(|operation| operation.class == OperationClass::SharedLeaf)
        .map(|operation| operation.id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    let mut actual_shared = BTreeSet::new();
    let mut shared_kinds = BTreeMap::new();
    let mut covered_outcomes = BTreeSet::new();
    for entry in semantic_array(policy, "shared_operations")? {
        semantic_exact_fields(
            entry,
            "shared operation",
            &[
                "operation",
                "kind",
                "revision_bound",
                "outcomes",
                "fixtures",
            ],
        )?;
        let operation = semantic_string(entry, "operation", "shared operation")?;
        if !actual_shared.insert(operation.to_owned()) {
            return Err(format!("duplicate shared semantic operation {operation}"));
        }
        let kind = semantic_string(entry, "kind", "shared operation")?;
        if !matches!(kind, "read" | "mutation" | "preview" | "commit") {
            return Err(format!(
                "shared operation {operation} has unknown kind {kind}"
            ));
        }
        shared_kinds.insert(operation.to_owned(), kind.to_owned());
        let revision_bound = entry["revision_bound"].as_bool().ok_or_else(|| {
            format!("shared operation {operation} revision_bound must be boolean")
        })?;
        let outcomes = semantic_outcomes(entry, operation, &required_outcomes)?;
        semantic_fixture_references(entry, operation, &fixtures)?;
        if !outcomes.contains("success") {
            return Err(format!(
                "shared operation {operation} lacks success coverage"
            ));
        }
        if matches!(kind, "mutation" | "commit") && !outcomes.contains("no-mutation") {
            return Err(format!(
                "shared mutating operation {operation} lacks no-mutation coverage"
            ));
        }
        if revision_bound && !outcomes.contains("stale") {
            return Err(format!(
                "revision-bound shared operation {operation} lacks stale coverage"
            ));
        }
        if operation == "application.approve" && outcomes != required_outcomes {
            return Err(
                "application.approve must cover the complete approval outcome set".to_owned(),
            );
        }
        covered_outcomes.extend(outcomes);
    }
    if actual_shared != expected_shared {
        return Err(format!(
            "shared semantic operations drifted: missing={:?}, unexpected={:?}",
            expected_shared
                .difference(&actual_shared)
                .collect::<Vec<_>>(),
            actual_shared
                .difference(&expected_shared)
                .collect::<Vec<_>>()
        ));
    }

    let expected_pack_surfaces = ["generic-application", "academic-job"]
        .into_iter()
        .flat_map(|pack| {
            ["cli", "tauri", "mcp"]
                .into_iter()
                .map(move |surface| (pack.to_owned(), surface.to_owned()))
        })
        .collect::<BTreeSet<_>>();
    let mut actual_pack_surfaces = BTreeSet::new();
    let mut qualified = BTreeSet::new();
    for entry in semantic_array(policy, "pack_surface_cases")? {
        semantic_exact_fields(
            entry,
            "Pack surface case",
            &["pack", "surface", "operations", "fixtures"],
        )?;
        let pack = semantic_string(entry, "pack", "Pack surface case")?;
        let surface_name = semantic_string(entry, "surface", "Pack surface case")?;
        let surface = semantic_surface(surface_name)?;
        if !actual_pack_surfaces.insert((pack.to_owned(), surface_name.to_owned())) {
            return Err(format!(
                "duplicate semantic Pack/surface case {pack}/{surface_name}"
            ));
        }
        let operations = string_set(
            &entry["operations"],
            &format!("semantic operations for {pack}/{surface_name}"),
        )?;
        if operations.is_empty() {
            return Err(format!(
                "semantic Pack/surface case {pack}/{surface_name} has no operations"
            ));
        }
        semantic_fixture_references(entry, &format!("{pack}/{surface_name}"), &fixtures)?;
        for operation in operations {
            let binding = resolved
                .iter()
                .find(|binding| {
                    binding.surface == surface && binding.operation.as_str() == operation
                })
                .ok_or_else(|| {
                    format!(
                        "semantic case {pack}/{surface_name} references unbound operation {operation}"
                    )
                })?;
            if !semantic_pack_matches(pack, binding.pack_scope) {
                return Err(format!(
                    "semantic case {pack}/{surface_name} operation {operation} has {:?} Pack scope",
                    binding.pack_scope
                ));
            }
            qualified.insert(semantic_binding_key(binding));
        }
    }
    if actual_pack_surfaces != expected_pack_surfaces {
        return Err(format!(
            "semantic Pack/surface matrix drifted: missing={:?}, unexpected={:?}",
            expected_pack_surfaces
                .difference(&actual_pack_surfaces)
                .collect::<Vec<_>>(),
            actual_pack_surfaces
                .difference(&expected_pack_surfaces)
                .collect::<Vec<_>>()
        ));
    }

    for binding in resolved.iter().filter(|binding| {
        registry.operations.iter().any(|operation| {
            operation.id == binding.operation && operation.class == OperationClass::SharedLeaf
        })
    }) {
        if !qualified.contains(&semantic_binding_key(binding)) {
            return Err(format!(
                "shared {:?} leaf {} ({}) has no semantic surface fixture",
                binding.surface, binding.leaf, binding.operation
            ));
        }
    }

    let expected_revision_bound = BTreeSet::from([
        "application.approve".to_owned(),
        "application.export".to_owned(),
        "application.intake.commit".to_owned(),
        "evidence.association.commit".to_owned(),
        "profile.association.commit".to_owned(),
        "requirement.extract.commit".to_owned(),
        "requirement.confirm.commit".to_owned(),
        "plan.propose.commit".to_owned(),
        "plan.confirm.commit".to_owned(),
        "deliverable.draft.commit".to_owned(),
        "deliverable.revise.commit".to_owned(),
        "review.disposition.commit".to_owned(),
        "export.prepare.commit".to_owned(),
        "tauri.commit.workflow.rerun".to_owned(),
    ]);
    let mut revision_bound = BTreeSet::new();
    for entry in semantic_array(policy, "revision_bound_operations")? {
        semantic_exact_fields(
            entry,
            "revision-bound operation",
            &["operation", "fixtures"],
        )?;
        let operation = semantic_string(entry, "operation", "revision-bound operation")?;
        semantic_known_operation(operation, &known_operations)?;
        semantic_fixture_references(entry, operation, &fixtures)?;
        if !revision_bound.insert(operation.to_owned()) {
            return Err(format!("duplicate revision-bound operation {operation}"));
        }
    }
    if revision_bound != expected_revision_bound {
        return Err(format!(
            "revision-bound semantic matrix drifted: expected {expected_revision_bound:?}, found {revision_bound:?}"
        ));
    }

    let expected_pairs = BTreeSet::from([
        "generic-application-review".to_owned(),
        "desktop-application-intake".to_owned(),
        "desktop-discovery".to_owned(),
        "desktop-workflow-rerun".to_owned(),
        "evidence-association".to_owned(),
        "profile-association".to_owned(),
        "requirement-extract".to_owned(),
        "requirement-confirm".to_owned(),
        "plan-propose".to_owned(),
        "plan-confirm".to_owned(),
        "deliverable-draft".to_owned(),
        "deliverable-revise".to_owned(),
        "review-disposition".to_owned(),
        "export-prepare".to_owned(),
    ]);
    let mut preview_pairs = BTreeSet::new();
    let mut pair_operations = BTreeSet::new();
    for entry in semantic_array(policy, "preview_commit_pairs")? {
        semantic_exact_fields(
            entry,
            "preview/commit pair",
            &[
                "id",
                "preview_operations",
                "commit_operation",
                "outcomes",
                "fixtures",
            ],
        )?;
        let id = semantic_string(entry, "id", "preview/commit pair")?;
        if !preview_pairs.insert(id.to_owned()) {
            return Err(format!("duplicate preview/commit pair {id}"));
        }
        let previews = string_set(
            &entry["preview_operations"],
            &format!("preview operations for {id}"),
        )?;
        if previews.is_empty() {
            return Err(format!("preview/commit pair {id} has no preview operation"));
        }
        for operation in previews {
            semantic_known_operation(&operation, &known_operations)?;
            pair_operations.insert(operation);
        }
        let commit = semantic_string(entry, "commit_operation", "preview/commit pair")?;
        semantic_known_operation(commit, &known_operations)?;
        pair_operations.insert(commit.to_owned());
        let outcomes = semantic_outcomes(entry, id, &required_outcomes)?;
        semantic_fixture_references(entry, id, &fixtures)?;
        for required in [
            "success",
            "replay",
            "wrong-context",
            "no-mutation",
            "recovery",
        ] {
            if !outcomes.contains(required) {
                return Err(format!(
                    "preview/commit pair {id} lacks {required} coverage"
                ));
            }
        }
        if id != "desktop-discovery" && !outcomes.contains("stale") {
            return Err(format!(
                "revision-bound preview/commit pair {id} lacks stale coverage"
            ));
        }
        covered_outcomes.extend(outcomes);
    }
    if preview_pairs != expected_pairs {
        return Err(format!(
            "preview/commit pair matrix drifted: expected {expected_pairs:?}, found {preview_pairs:?}"
        ));
    }

    let expected_read_families = BTreeSet::from([
        "generic-application".to_owned(),
        "academic-application".to_owned(),
        "profile-source".to_owned(),
        "application-association".to_owned(),
    ]);
    let mut read_families = BTreeSet::new();
    let mut read_operations = BTreeSet::new();
    for entry in semantic_array(policy, "read_families")? {
        semantic_exact_fields(entry, "read family", &["id", "operations", "fixtures"])?;
        let id = semantic_string(entry, "id", "read family")?;
        if !read_families.insert(id.to_owned()) {
            return Err(format!("duplicate semantic read family {id}"));
        }
        let operations = string_set(&entry["operations"], &format!("read family {id}"))?;
        if operations.is_empty() {
            return Err(format!("semantic read family {id} has no operation"));
        }
        for operation in operations {
            semantic_known_operation(&operation, &known_operations)?;
            read_operations.insert(operation);
        }
        semantic_fixture_references(entry, id, &fixtures)?;
    }
    if read_families != expected_read_families {
        return Err(format!(
            "semantic read families drifted: expected {expected_read_families:?}, found {read_families:?}"
        ));
    }
    for operation in expected_shared.iter().filter(|operation| {
        shared_kinds
            .get(*operation)
            .is_some_and(|kind| kind == "read")
    }) {
        if !read_operations.contains(operation) {
            return Err(format!(
                "shared read operation {operation} is absent from every read family"
            ));
        }
    }

    if covered_outcomes != required_outcomes {
        return Err(format!(
            "semantic outcome matrix is incomplete: missing={:?}",
            required_outcomes
                .difference(&covered_outcomes)
                .collect::<Vec<_>>()
        ));
    }

    let uncovered_policy = &policy["uncovered_policy"];
    semantic_exact_fields(
        uncovered_policy,
        "uncovered policy",
        &["machine_list_command", "allowed_classes"],
    )?;
    if semantic_string(uncovered_policy, "machine_list_command", "uncovered policy")?
        != "cargo run -p xtask --locked -- semantics uncovered"
    {
        return Err("semantic uncovered command drifted".to_owned());
    }
    let allowed_uncovered = string_set(
        &uncovered_policy["allowed_classes"],
        "semantic uncovered classes",
    )?;
    let expected_uncovered = BTreeSet::from([
        "canonical-leaf".to_owned(),
        "compatibility-alias".to_owned(),
        "adapter-only".to_owned(),
    ]);
    if allowed_uncovered != expected_uncovered {
        return Err(format!(
            "semantic uncovered classes must be {expected_uncovered:?}"
        ));
    }

    for binding in &resolved {
        if pair_operations.contains(binding.operation.as_str())
            || read_operations.contains(binding.operation.as_str())
        {
            qualified.insert(semantic_binding_key(binding));
        }
    }
    let mut uncovered_bindings = Vec::new();
    for binding in &resolved {
        if qualified.contains(&semantic_binding_key(binding)) {
            continue;
        }
        let class = semantic_operation_class(binding.class);
        if !allowed_uncovered.contains(class) {
            return Err(format!(
                "unqualified semantic leaf {:?}/{} has unapproved class {class}",
                binding.surface, binding.leaf
            ));
        }
        uncovered_bindings.push(json!({
            "surface": semantic_surface_label(binding.surface),
            "leaf": binding.leaf,
            "operation": binding.operation.as_str(),
            "class": class,
            "pack_scope": semantic_pack_scope(binding.pack_scope),
        }));
    }
    uncovered_bindings.sort_by_key(|value| {
        format!(
            "{}:{}",
            value["surface"].as_str().unwrap_or_default(),
            value["leaf"].as_str().unwrap_or_default()
        )
    });

    Ok(SemanticParityReport {
        shared_operations: actual_shared.len(),
        preview_pairs: preview_pairs.len(),
        read_families: read_families.len(),
        qualified_bindings: qualified.len(),
        uncovered_bindings,
    })
}

fn semantic_array<'a>(value: &'a Value, field: &str) -> Result<&'a [Value], String> {
    value[field]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("semantic parity {field} must be an array"))
}

fn semantic_exact_fields(value: &Value, context: &str, expected: &[&str]) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("semantic parity {context} must be an object"))?;
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!(
            "semantic parity {context} fields must be {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

fn semantic_string<'a>(value: &'a Value, field: &str, context: &str) -> Result<&'a str, String> {
    value[field]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("semantic parity {context} {field} must be a non-empty string"))
}

fn semantic_fixture_references(
    value: &Value,
    context: &str,
    fixtures: &BTreeSet<String>,
) -> Result<BTreeSet<String>, String> {
    let references = string_set(
        &value["fixtures"],
        &format!("semantic fixture references for {context}"),
    )?;
    if references.is_empty() {
        return Err(format!("semantic matrix entry {context} has no fixture"));
    }
    let unknown = references.difference(fixtures).cloned().collect::<Vec<_>>();
    if !unknown.is_empty() {
        return Err(format!(
            "semantic matrix entry {context} references unknown fixtures {unknown:?}"
        ));
    }
    Ok(references)
}

fn semantic_outcomes(
    value: &Value,
    context: &str,
    required: &BTreeSet<String>,
) -> Result<BTreeSet<String>, String> {
    let outcomes = string_set(
        &value["outcomes"],
        &format!("semantic outcomes for {context}"),
    )?;
    if outcomes.is_empty() {
        return Err(format!("semantic matrix entry {context} has no outcome"));
    }
    let unknown = outcomes.difference(required).cloned().collect::<Vec<_>>();
    if !unknown.is_empty() {
        return Err(format!(
            "semantic matrix entry {context} has unknown outcomes {unknown:?}"
        ));
    }
    Ok(outcomes)
}

fn semantic_known_operation(
    operation: &str,
    known_operations: &BTreeSet<String>,
) -> Result<(), String> {
    if known_operations.contains(operation) {
        Ok(())
    } else {
        Err(format!(
            "semantic matrix references unknown operation {operation}"
        ))
    }
}

fn semantic_surface(value: &str) -> Result<OperationSurface, String> {
    match value {
        "cli" => Ok(OperationSurface::Cli),
        "tauri" => Ok(OperationSurface::Tauri),
        "mcp" => Ok(OperationSurface::Mcp),
        _ => Err(format!("unknown semantic surface {value}")),
    }
}

fn semantic_surface_label(surface: OperationSurface) -> &'static str {
    match surface {
        OperationSurface::Cli => "cli",
        OperationSurface::Tauri => "tauri",
        OperationSurface::Mcp => "mcp",
    }
}

fn semantic_pack_matches(value: &str, scope: OperationPackScope) -> bool {
    matches!(
        (value, scope),
        (
            "generic-application" | "academic-job",
            OperationPackScope::Any
        ) | (
            "generic-application",
            OperationPackScope::GenericApplication
        ) | ("academic-job", OperationPackScope::AcademicJob)
    )
}

fn semantic_pack_scope(scope: OperationPackScope) -> &'static str {
    match scope {
        OperationPackScope::Any => "any",
        OperationPackScope::GenericApplication => "generic-application",
        OperationPackScope::AcademicJob => "academic-job",
    }
}

fn semantic_operation_class(class: OperationClass) -> &'static str {
    match class {
        OperationClass::CanonicalLeaf => "canonical-leaf",
        OperationClass::SharedLeaf => "shared-leaf",
        OperationClass::CompatibilityAlias => "compatibility-alias",
        OperationClass::Composite => "composite",
        OperationClass::WildcardAlias => "wildcard-alias",
        OperationClass::AdapterOnly => "adapter-only",
    }
}

fn semantic_binding_key(binding: &canisend_contracts::ResolvedOperationBinding) -> String {
    format!(
        "{}:{}",
        semantic_surface_label(binding.surface),
        binding.leaf
    )
}

fn check_operation_registry() -> Result<(), String> {
    let root = repository_root();
    let registry = OperationRegistry::built_in()
        .map_err(|error| format!("typed operation registry is invalid: {error}"))?;

    let cli = canisend_cli::clap_leaf_paths()
        .into_iter()
        .collect::<BTreeSet<_>>();
    compare_operation_surface(
        OperationSurface::Cli,
        &cli,
        &registry
            .surface_leaves(OperationSurface::Cli)
            .map_err(|error| format!("cannot resolve CLI operation registry: {error}"))?,
    )?;

    let tauri_source = fs::read_to_string(root.join("crates/canisend-desktop/src/lib.rs"))
        .map_err(|error| format!("cannot read Tauri handler source: {error}"))?;
    let tauri = extract_tauri_handlers(&tauri_source)?;
    compare_operation_surface(
        OperationSurface::Tauri,
        &tauri,
        &registry
            .surface_leaves(OperationSurface::Tauri)
            .map_err(|error| format!("cannot resolve Tauri operation registry: {error}"))?,
    )?;
    let retired_tauri_operations = BTreeSet::from([
        "agent_capabilities",
        "agent_context",
        "archive_job",
        "cancel_task",
        "commit_job_source_preview",
        "commit_task_completion_preview",
        "create_job",
        "export_task_inputs",
        "latest_task",
        "list_jobs",
        "migrate_workspace_v3",
        "prepare_task",
        "prepare_task_again",
        "preview_local_job_source",
        "preview_workspace_v3_migration",
        "preview_task_completion",
        "show_job",
        "workflow_controls",
    ]);
    if let Some(handler) = retired_tauri_operations
        .iter()
        .find(|handler| tauri.contains(**handler))
    {
        return Err(format!(
            "retired pre-v4 Tauri operation `{handler}` must fail before desktop invocation"
        ));
    }
    let bridge_source = fs::read_to_string(root.join("apps/canisend-desktop/src/lib/bridge.ts"))
        .map_err(|error| format!("cannot inspect desktop bridge: {error}"))?;
    if let Some(handler) = retired_tauri_operations
        .iter()
        .find(|handler| bridge_source.contains(&format!("invoke(\"{handler}\"")))
    {
        return Err(format!(
            "retired pre-v4 desktop operation `{handler}` still reaches the Tauri bridge"
        ));
    }
    let actual_compatibility = registry
        .compatibility_aliases
        .iter()
        .map(|alias| alias.id.as_str())
        .collect::<BTreeSet<_>>();
    if !actual_compatibility.is_empty() {
        return Err(format!(
            "Alpha.7 clean-v4 operation registry must not export compatibility aliases: actual={actual_compatibility:?}"
        ));
    }
    let desktop_sources = fs::read_dir(root.join("crates/canisend-desktop/src"))
        .map_err(|error| format!("cannot inspect Tauri command sources: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .map(|path| {
            fs::read_to_string(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    for handler in &tauri {
        if !desktop_sources.contains(&format!("fn {handler}(")) {
            return Err(format!(
                "registered Tauri handler `{handler}` has no command function declaration"
            ));
        }
    }

    let mcp_source = fs::read_to_string(root.join("crates/canisend-mcp/src/lib.rs"))
        .map_err(|error| format!("cannot read MCP router source: {error}"))?;
    let mcp = extract_mcp_tools(&mcp_source)?;
    compare_operation_surface(
        OperationSurface::Mcp,
        &mcp,
        &registry
            .surface_leaves(OperationSurface::Mcp)
            .map_err(|error| format!("cannot resolve MCP operation registry: {error}"))?,
    )?;

    let bindings = registry
        .resolved_bindings()
        .map_err(|error| format!("cannot resolve operation bindings: {error}"))?;
    let adapter_only = bindings
        .iter()
        .filter(|binding| binding.class == canisend_contracts::OperationClass::AdapterOnly)
        .count();
    let compatibility = bindings
        .iter()
        .filter(|binding| binding.class == canisend_contracts::OperationClass::CompatibilityAlias)
        .count();
    println!(
        "operation registry: ok ({} CLI, {} Tauri, {} MCP leaves; {adapter_only} adapter-only, {compatibility} compatibility bindings)",
        cli.len(),
        tauri.len(),
        mcp.len()
    );
    Ok(())
}

fn compare_operation_surface(
    surface: OperationSurface,
    actual: &BTreeSet<String>,
    expected: &BTreeSet<String>,
) -> Result<(), String> {
    if actual == expected {
        return Ok(());
    }
    let unregistered = actual.difference(expected).cloned().collect::<Vec<_>>();
    let missing_from_source = expected.difference(actual).cloned().collect::<Vec<_>>();
    Err(format!(
        "{surface:?} operation leaves drifted: unregistered={unregistered:?}, missing_from_source={missing_from_source:?}"
    ))
}

fn extract_tauri_handlers(source: &str) -> Result<BTreeSet<String>, String> {
    let mut inside = false;
    let mut found = false;
    let mut handlers = BTreeSet::new();
    for line in source.lines() {
        if !inside {
            if line.contains("tauri::generate_handler![") {
                inside = true;
                found = true;
            }
            continue;
        }
        let trimmed = line.trim();
        if trimmed == "])" || trimmed == "]).run" {
            inside = false;
            break;
        }
        let candidate = trimmed.trim_end_matches(',');
        let Some((_, handler)) = candidate.rsplit_once("::") else {
            return Err(format!(
                "unexpected line in Tauri generate_handler inventory: `{trimmed}`"
            ));
        };
        if !handlers.insert(handler.to_owned()) {
            return Err(format!("duplicate Tauri handler registration: {handler}"));
        }
    }
    if !found || inside || handlers.is_empty() {
        return Err("could not extract a closed Tauri generate_handler inventory".to_owned());
    }
    Ok(handlers)
}

fn extract_mcp_tools(source: &str) -> Result<BTreeSet<String>, String> {
    let mut inside_router = false;
    let mut found_router = false;
    let mut tools = BTreeSet::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if !inside_router {
            if trimmed == "#[tool_router]" {
                inside_router = true;
                found_router = true;
            }
            continue;
        }
        if trimmed.starts_with("#[tool_handler") {
            inside_router = false;
            break;
        }
        let Some(rest) = trimmed.strip_prefix("fn canisend_") else {
            continue;
        };
        let name_tail = rest
            .split_once('(')
            .map(|(name, _)| name)
            .ok_or_else(|| format!("malformed MCP tool declaration: `{trimmed}`"))?;
        let name = format!("canisend_{name_tail}");
        if !tools.insert(name.clone()) {
            return Err(format!("duplicate MCP tool declaration: {name}"));
        }
    }
    if !found_router || inside_router || tools.is_empty() {
        return Err("could not extract a closed MCP #[tool_router] inventory".to_owned());
    }
    Ok(tools)
}

fn check_svelte_parity() -> Result<(), String> {
    let root = repository_root();
    let legacy_path = root.join("docs/contracts/cli-gui-parity-v1.json");
    let legacy: Value = serde_json::from_slice(
        &fs::read(&legacy_path)
            .map_err(|error| format!("CLI/GUI parity manifest is missing: {error}"))?,
    )
    .map_err(|error| format!("CLI/GUI parity manifest is invalid JSON: {error}"))?;
    let legacy_entries = legacy["entries"]
        .as_array()
        .ok_or_else(|| "CLI/GUI parity manifest entries must be an array".to_owned())?;
    let legacy_operations = legacy_entries
        .iter()
        .map(|entry| {
            entry["operation"]
                .as_str()
                .ok_or_else(|| "CLI/GUI parity entry is missing operation".to_owned())
        })
        .collect::<Result<BTreeSet<_>, _>>()?;

    let path = root.join("docs/contracts/svelte-parity-v1.json");
    let document: Value = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("Svelte parity ledger is missing: {error}"))?,
    )
    .map_err(|error| format!("Svelte parity ledger is invalid JSON: {error}"))?;
    if document["format"].as_str() != Some(SVELTE_PARITY_SCHEMA) {
        return Err(format!(
            "Svelte parity ledger format must be {SVELTE_PARITY_SCHEMA}"
        ));
    }
    let cutover_ready = document["cutover_ready"]
        .as_bool()
        .ok_or_else(|| "Svelte parity ledger must define cutover_ready".to_owned())?;
    let legacy_required = document["legacy_egui_required"]
        .as_bool()
        .ok_or_else(|| "Svelte parity ledger must define legacy_egui_required".to_owned())?;
    if cutover_ready == legacy_required {
        return Err(
            "Svelte cutover_ready and legacy_egui_required must be exact opposites".to_owned(),
        );
    }
    let entries = document["entries"]
        .as_array()
        .ok_or_else(|| "Svelte parity ledger entries must be an array".to_owned())?;
    let desktop_source = root.join("crates/canisend-desktop/src");
    let mut command_files = fs::read_dir(&desktop_source)
        .map_err(|error| format!("cannot inspect Tauri command sources: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect::<Vec<_>>();
    command_files.sort();
    let command_sources = command_files
        .into_iter()
        .map(|source| {
            fs::read_to_string(&source).map_err(|error| {
                format!(
                    "cannot read Tauri command source {}: {error}",
                    source.display()
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    let mut operations = BTreeSet::new();
    let mut implemented = 0_usize;
    for entry in entries {
        let operation = entry["operation"]
            .as_str()
            .ok_or_else(|| "Svelte parity entry is missing operation".to_owned())?;
        if !operations.insert(operation) {
            return Err(format!("duplicate Svelte parity operation: {operation}"));
        }
        match entry["status"].as_str() {
            Some("implemented") => {
                implemented += 1;
                for field in ["tauri_commands", "svelte_views", "tests"] {
                    let evidence = entry[field]
                        .as_array()
                        .filter(|items| !items.is_empty())
                        .ok_or_else(|| {
                            format!(
                                "implemented Svelte parity entry {operation} needs {field} evidence"
                            )
                        })?;
                    for item in evidence {
                        let value = item.as_str().ok_or_else(|| {
                            format!("Svelte parity {operation} {field} evidence must be a string")
                        })?;
                        if field == "tauri_commands" {
                            let declaration = format!("fn {value}(");
                            let registration = format!("::{value}");
                            if !command_sources.contains(&declaration)
                                || !command_sources.contains(&registration)
                            {
                                return Err(format!(
                                    "Svelte parity command evidence `{value}` for {operation} is not declared and registered"
                                ));
                            }
                        } else {
                            let evidence_path = Path::new(value);
                            if evidence_path.is_absolute()
                                || evidence_path.components().any(|component| {
                                    matches!(component, std::path::Component::ParentDir)
                                })
                                || !root.join(evidence_path).is_file()
                            {
                                return Err(format!(
                                    "Svelte parity evidence `{value}` for {operation} is not a repository file"
                                ));
                            }
                        }
                    }
                }
            }
            Some("pending") => {
                if entry["target_stage"]
                    .as_str()
                    .is_none_or(|stage| !matches!(stage, "TS3" | "TS4" | "TS5"))
                {
                    return Err(format!(
                        "pending Svelte parity entry {operation} needs a TS3, TS4, or TS5 target"
                    ));
                }
            }
            Some(other) => {
                return Err(format!(
                    "Svelte parity entry {operation} has unknown status {other}"
                ));
            }
            None => return Err(format!("Svelte parity entry {operation} is missing status")),
        }
    }
    if operations != legacy_operations {
        let missing = legacy_operations
            .difference(&operations)
            .collect::<Vec<_>>();
        let extra = operations
            .difference(&legacy_operations)
            .collect::<Vec<_>>();
        return Err(format!(
            "Svelte parity operation set differs from CLI/GUI authority; missing {missing:?}, extra {extra:?}"
        ));
    }
    if cutover_ready && implemented != entries.len() {
        return Err(
            "Svelte cutover cannot be ready while any operation family remains pending".to_owned(),
        );
    }
    if cutover_ready {
        let workspace_manifest = fs::read_to_string(root.join("Cargo.toml"))
            .map_err(|error| format!("cannot inspect workspace manifest for cutover: {error}"))?;
        for forbidden in ["eframe =", "rfd =", "\"crates/canisend-gui\""] {
            if workspace_manifest.contains(forbidden) {
                return Err(format!(
                    "Svelte cutover still contains legacy workspace entry `{forbidden}`"
                ));
            }
        }
        if root.join("crates/canisend-gui/Cargo.toml").exists() {
            return Err("Svelte cutover still contains the legacy egui crate".to_owned());
        }

        let desktop_manifest = fs::read_to_string(root.join("crates/canisend-desktop/Cargo.toml"))
            .map_err(|error| format!("cannot inspect Svelte desktop manifest: {error}"))?;
        for required in [
            "name = \"canisend-gui\"",
            "tauri.workspace = true",
            "tauri-plugin-dialog.workspace = true",
            "tauri-plugin-window-state.workspace = true",
            "custom-protocol = [\"tauri/custom-protocol\"]",
        ] {
            if !desktop_manifest.contains(required) {
                return Err(format!(
                    "Svelte cutover desktop manifest is missing `{required}`"
                ));
            }
        }

        let tauri_config = fs::read_to_string(root.join("crates/canisend-desktop/tauri.conf.json"))
            .map_err(|error| format!("cannot inspect Svelte desktop configuration: {error}"))?;
        for required in [
            "\"productName\": \"CanISend\"",
            "\"identifier\": \"io.github.jxpeng98.canisend\"",
            "\"frontendDist\": \"../../apps/canisend-desktop/dist\"",
        ] {
            if !tauri_config.contains(required) {
                return Err(format!(
                    "Svelte cutover desktop configuration is missing `{required}`"
                ));
            }
        }

        let capability_path = root.join("crates/canisend-desktop/capabilities/default.json");
        let capability: Value =
            serde_json::from_slice(&fs::read(&capability_path).map_err(|error| {
                format!("cannot inspect Svelte desktop capability policy: {error}")
            })?)
            .map_err(|error| {
                format!("Svelte desktop capability policy is invalid JSON: {error}")
            })?;
        let permissions = capability["permissions"]
            .as_array()
            .ok_or_else(|| "Svelte desktop capability permissions must be an array".to_owned())?
            .iter()
            .map(|permission| {
                permission.as_str().ok_or_else(|| {
                    "Svelte desktop capability permissions must contain only strings".to_owned()
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let expected_permissions = [
            "core:default",
            "core:window:allow-start-dragging",
            "dialog:allow-open",
        ];
        if permissions != expected_permissions {
            return Err(format!(
                "Svelte desktop capability policy must grant only the baseline, title-bar dragging, and file-open permissions; found {permissions:?}"
            ));
        }

        let vite_config = fs::read_to_string(root.join("apps/canisend-desktop/vite.config.ts"))
            .map_err(|error| format!("cannot inspect Svelte Vite configuration: {error}"))?;
        if !vite_config.contains("base: \"./\"") {
            return Err(
                "Svelte cutover Vite configuration must use relative production asset URLs"
                    .to_owned(),
            );
        }

        let stage_script = fs::read_to_string(root.join("scripts/stage_macos_gui_app.sh"))
            .map_err(|error| format!("cannot inspect macOS GUI staging: {error}"))?;
        for forbidden in ["epaint", "EGUI-FONT"] {
            if stage_script.contains(forbidden) {
                return Err(format!(
                    "Svelte cutover staging still contains legacy renderer reference `{forbidden}`"
                ));
            }
        }

        let design_preview = fs::read_to_string(root.join("scripts/build_macos_design_preview.sh"))
            .map_err(|error| format!("cannot inspect macOS Design Preview: {error}"))?;
        for required in [
            "CanISend Design Preview.app",
            "io.github.jxpeng98.canisend.design-preview",
            "application create",
            "org.canisend.generic-application",
            "org.canisend.academic-job",
            "bundle_identifier: \"io.github.jxpeng98.canisend.design-preview\"",
        ] {
            if !design_preview.contains(required) {
                return Err(format!(
                    "macOS Design Preview is missing clean-v4 isolation contract `{required}`"
                ));
            }
        }
        for forbidden in [" job create ", "workspace init --pack"] {
            if design_preview.contains(forbidden) {
                return Err(format!(
                    "macOS Design Preview still contains compatibility fixture `{forbidden}`"
                ));
            }
        }

        let fast_ci = fs::read_to_string(root.join(".github/workflows/fast-ci.yml"))
            .map_err(|error| format!("cannot inspect fast CI for Svelte cutover: {error}"))?;
        for required in [
            "Upload production desktop UI",
            "Download exact production desktop UI",
            "canisend-desktop-ui-${{ github.sha }}",
        ] {
            if !fast_ci.contains(required) {
                return Err(format!(
                    "Svelte cutover fast CI is missing frontend handoff `{required}`"
                ));
            }
        }

        for workflow in [
            ".github/workflows/release.yml",
            ".github/workflows/intel-gui-compile.yml",
        ] {
            let body = fs::read_to_string(root.join(workflow))
                .map_err(|error| format!("cannot inspect {workflow}: {error}"))?;
            for required in [
                "pnpm --dir apps/canisend-desktop install --frozen-lockfile",
                "pnpm --dir apps/canisend-desktop build",
                "-p canisend-gui",
                "--features canisend-gui/custom-protocol",
            ] {
                if !body.contains(required) {
                    return Err(format!(
                        "Svelte cutover workflow {workflow} is missing `{required}`"
                    ));
                }
            }
        }
    }
    println!(
        "Svelte parity: ok ({implemented}/{} implemented, cutover_ready={cutover_ready})",
        entries.len()
    );
    Ok(())
}

fn check_alpha_package_contract() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/alpha-package-contract.json");
    let contract: Value = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("Alpha package contract is missing: {error}"))?,
    )
    .map_err(|error| format!("Alpha package contract is invalid JSON: {error}"))?;
    let parsed_version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("source version is invalid SemVer: {error}"))?;
    check_alpha_package_contract_identity_and_bindings(&root, &parsed_version, &contract)?;
    let version = env!("CARGO_PKG_VERSION");
    for pointer in [
        "/standalone_cli/build_profile",
        "/desktop_macos/build_profile",
    ] {
        if contract.pointer(pointer).and_then(Value::as_str) != Some("release-alpha") {
            return Err(format!(
                "Alpha package contract {pointer} must be release-alpha"
            ));
        }
    }
    let assets = contract
        .pointer("/standalone_cli/assets")
        .and_then(Value::as_array)
        .ok_or_else(|| "Alpha package contract CLI assets must be an array".to_owned())?;
    let mut actual_targets = BTreeSet::new();
    for asset in assets {
        let target = asset
            .get("target")
            .and_then(Value::as_str)
            .ok_or_else(|| "Alpha CLI asset is missing target".to_owned())?;
        let file = asset
            .get("file")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("Alpha CLI asset {target} is missing file"))?;
        let extension = if target == "x86_64-pc-windows-msvc" {
            "zip"
        } else {
            "tar.gz"
        };
        let expected = format!("canisend-{version}-{target}.{extension}");
        if file != expected {
            return Err(format!(
                "Alpha CLI asset {target} must be named {expected}, found {file}"
            ));
        }
        if !actual_targets.insert(target) {
            return Err(format!("duplicate Alpha CLI package target: {target}"));
        }
    }
    let expected_targets = release_targets()?
        .into_iter()
        .map(|target| target.triple)
        .collect::<BTreeSet<_>>();
    let actual_targets = actual_targets
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if actual_targets != expected_targets {
        return Err(format!(
            "Alpha package target set differs: expected {expected_targets:?}, found {actual_targets:?}"
        ));
    }
    for (pointer, expected) in [
        (
            "/desktop_macos/archive",
            format!("CanISend-{version}-aarch64-apple-darwin.zip"),
        ),
        (
            "/desktop_macos/dmg",
            format!("CanISend-{version}-aarch64-apple-darwin.dmg"),
        ),
        (
            "/desktop_macos/applications_link",
            "/Applications".to_owned(),
        ),
        (
            "/desktop_macos/unified_host_executable",
            "Contents/MacOS/canisend-gui".to_owned(),
        ),
        (
            "/desktop_macos/bundle_schema",
            "canisend.macos-app-bundle/v3".to_owned(),
        ),
        (
            "/desktop_macos/companion_schema",
            "canisend.macos-app-integrity/v2".to_owned(),
        ),
    ] {
        let actual = contract.pointer(pointer).and_then(Value::as_str);
        if actual != Some(expected.as_str()) {
            return Err(format!(
                "Alpha package contract {pointer} must be {expected}, found {}",
                actual.unwrap_or("<missing>")
            ));
        }
    }
    if contract
        .pointer("/desktop_macos/full_native_host_count")
        .and_then(Value::as_u64)
        != Some(1)
        || contract
            .pointer("/desktop_macos/application_payload_budget_bytes")
            .and_then(Value::as_u64)
            != Some(75_497_472)
        || contract
            .pointer("/desktop_macos/host_entry_modes")
            .and_then(Value::as_array)
            != Some(&vec![
                Value::String("gui".to_owned()),
                Value::String("cli".to_owned()),
                Value::String("mcp".to_owned()),
            ])
    {
        return Err(
            "Alpha macOS package must freeze one unified GUI/CLI/MCP host and the 72 MiB payload budget"
                .to_owned(),
        );
    }
    let top_level = contract
        .pointer("/desktop_macos/archive_top_level")
        .and_then(Value::as_array)
        .ok_or_else(|| "Alpha macOS archive top-level contract must be an array".to_owned())?;
    if top_level
        != &vec![
            Value::String("CanISend.app".to_owned()),
            Value::String("CanISend.app.manifest.json".to_owned()),
        ]
    {
        return Err(
            "Alpha macOS archive top level must be exactly CanISend.app and its companion manifest"
                .to_owned(),
        );
    }
    let dmg_top_level = contract
        .pointer("/desktop_macos/dmg_top_level")
        .and_then(Value::as_array)
        .ok_or_else(|| "Alpha macOS DMG top-level contract must be an array".to_owned())?;
    if dmg_top_level
        != &vec![
            Value::String("Applications".to_owned()),
            Value::String("CanISend.app".to_owned()),
            Value::String("CanISend.app.manifest.json".to_owned()),
        ]
    {
        return Err(
            "Alpha macOS DMG top level must contain Applications, CanISend.app, and its companion manifest"
                .to_owned(),
        );
    }
    if contract
        .pointer("/desktop_macos/signing")
        .and_then(Value::as_str)
        != Some("apple-adhoc")
        || contract
            .pointer("/desktop_macos/developer_id")
            .and_then(Value::as_bool)
            != Some(false)
        || contract
            .pointer("/desktop_macos/notarized")
            .and_then(Value::as_bool)
            != Some(false)
    {
        return Err(
            "Alpha macOS package must freeze ad-hoc, non-Developer-ID, non-notarized signing"
                .to_owned(),
        );
    }
    if contract
        .pointer("/desktop_macos_intel/target")
        .and_then(Value::as_str)
        != Some("x86_64-apple-darwin")
        || contract
            .pointer("/desktop_macos_intel/status")
            .and_then(Value::as_str)
            != Some("not-published")
        || !contract
            .pointer("/desktop_macos_intel/evidence")
            .is_some_and(Value::is_null)
        || !contract
            .pointer("/desktop_macos_intel/archive")
            .is_some_and(Value::is_null)
        || contract
            .pointer("/desktop_macos_intel/native_runtime_qualified")
            .and_then(Value::as_bool)
            != Some(false)
        || contract
            .pointer("/desktop_macos_intel/support_claim")
            .and_then(Value::as_bool)
            != Some(false)
        || contract
            .pointer("/desktop_macos_intel/scheduled_compile_owner")
            .and_then(Value::as_str)
            != Some(".github/workflows/intel-gui-compile.yml")
    {
        return Err(
            "Alpha Intel macOS GUI contract must publish no archive or evidence and make no support claim"
                .to_owned(),
        );
    }
    let fixture_path = contract
        .pointer("/update_response_fixture/path")
        .and_then(Value::as_str)
        .ok_or_else(|| "Alpha update fixture path is missing".to_owned())?;
    let fixture = fs::read(root.join(fixture_path))
        .map_err(|error| format!("Alpha update fixture is missing: {error}"))?;
    serde_json::from_slice::<Value>(&fixture)
        .map_err(|error| format!("Alpha update fixture is invalid JSON: {error}"))?;
    let digest = hex::encode(Sha256::digest(&fixture));
    if contract
        .pointer("/update_response_fixture/sha256")
        .and_then(Value::as_str)
        != Some(digest.as_str())
    {
        return Err(format!(
            "Alpha update fixture digest drifted; expected contract digest {digest}"
        ));
    }
    let performance_path = root.join("docs/performance/macos-gui-alpha-baseline.json");
    let performance: Value =
        serde_json::from_slice(&fs::read(&performance_path).map_err(|error| {
            format!("macOS GUI Alpha performance baseline is missing: {error}")
        })?)
        .map_err(|error| {
            format!("macOS GUI Alpha performance baseline is invalid JSON: {error}")
        })?;
    if performance.get("schema").and_then(Value::as_str)
        != Some("canisend.macos-gui-performance/v2")
        || performance.get("version").and_then(Value::as_str) != Some(version)
        || performance.get("profile").and_then(Value::as_str) != Some("release-alpha")
        || performance.get("passed").and_then(Value::as_bool) != Some(true)
    {
        return Err(
            "macOS GUI Alpha performance baseline must match the version and record a passing v2 unified-host measurement"
                .to_owned(),
        );
    }
    for (pointer, expected) in [
        ("/budgets/maximum_startup_ms", 2_000_u64),
        ("/budgets/unified_host_bytes", 67_108_864),
        ("/budgets/application_payload_bytes", 75_497_472),
    ] {
        if performance.pointer(pointer).and_then(Value::as_u64) != Some(expected) {
            return Err(format!(
                "macOS GUI Alpha performance baseline {pointer} must remain {expected}"
            ));
        }
    }
    let maximum_ms = performance
        .get("maximum_ms")
        .and_then(Value::as_f64)
        .ok_or_else(|| "macOS GUI Alpha baseline maximum_ms is missing".to_owned())?;
    let gui_bytes = performance
        .pointer("/bytes/unified_host")
        .and_then(Value::as_u64)
        .ok_or_else(|| "macOS GUI Alpha baseline GUI byte count is missing".to_owned())?;
    let bundle_bytes = performance
        .pointer("/bytes/application_payload")
        .and_then(Value::as_u64)
        .ok_or_else(|| "macOS GUI Alpha baseline App byte count is missing".to_owned())?;
    if maximum_ms > 2_000.0 || gui_bytes > 67_108_864 || bundle_bytes > 75_497_472 {
        return Err("macOS GUI Alpha performance baseline exceeds a frozen budget".to_owned());
    }
    let size_path = root.join("docs/performance/desktop-size-aarch64-apple-darwin.json");
    let size: Value = serde_json::from_slice(
        &fs::read(&size_path)
            .map_err(|error| format!("macOS unified-host size baseline is missing: {error}"))?,
    )
    .map_err(|error| format!("macOS unified-host size baseline is invalid JSON: {error}"))?;
    if size["schema"] != "canisend.desktop-size/v1"
        || size["target"] != "aarch64-apple-darwin"
        || size["profile"] != "release-alpha"
        || size["package_format"] != "app"
        || size["package_class"] != "standard"
        || size["passed"] != true
        || size["budgets"]["unified_host_bytes"] != 67_108_864_u64
        || size["budgets"]["application_payload_bytes"] != 75_497_472_u64
        || size["budgets"]["frontend_bytes"] != 1_572_864_u64
        || size["budgets"]["full_native_host_count"] != 1_u64
        || size["bytes"]["unified_host"] != gui_bytes
        || size["bytes"]["application_payload"]
            .as_u64()
            .is_none_or(|bytes| bytes > 75_497_472)
        || size["bytes"]["frontend"]
            .as_u64()
            .is_none_or(|bytes| bytes > 1_572_864)
        || size["native_hosts"]
            != json!([{
                "bytes": gui_bytes,
                "path": "Contents/MacOS/canisend-gui"
            }])
        || size["sha256"]["unified_host"] != performance["sha256"]["unified_host"]
    {
        return Err(
            "macOS unified-host size baseline must bind one host and the frozen standard-package budgets"
                .to_owned(),
        );
    }
    println!(
        "Alpha package contract: ok ({} CLI assets, one-host macOS ZIP and DMG desktop artifacts, no Intel GUI release evidence, GUI performance and payload baselines)",
        assets.len()
    );
    Ok(())
}

fn check_alpha_package_contract_identity_and_bindings(
    root: &Path,
    version: &Version,
    contract: &Value,
) -> Result<(), String> {
    let expected_schema = alpha_package_contract_schema(version)?;
    if contract.get("schema").and_then(Value::as_str) != Some(expected_schema) {
        return Err(format!(
            "Alpha package contract schema must be {expected_schema} for {version}"
        ));
    }
    let version_text = version.to_string();
    if contract.get("version").and_then(Value::as_str) != Some(version_text.as_str())
        || contract.get("tag").and_then(Value::as_str) != Some(&format!("v{version}"))
    {
        return Err(format!(
            "Alpha package contract version and tag must be {version} and v{version}"
        ));
    }
    let expected_bindings = alpha_package_contract_bindings(root)?;
    if contract.get("contracts") != Some(&expected_bindings) {
        return Err(
            "Alpha package contract does not bind the exact v4 protocols, built-in Packs, resources, operation registry, and migrations"
                .to_owned(),
        );
    }
    Ok(())
}

fn check_beta_readiness() -> Result<(), String> {
    let path = repository_root().join("release/beta-readiness.json");
    check_beta_readiness_file(&path)
}

fn check_beta_readiness_file(path: &Path) -> Result<(), String> {
    let body = fs::read_to_string(path).map_err(|error| {
        format!(
            "Beta readiness ledger is missing at {}: {error}",
            path.display()
        )
    })?;
    let ledger: Value = serde_json::from_str(&body)
        .map_err(|error| format!("Beta readiness ledger is invalid JSON: {error}"))?;
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    if ledger["status"] == "pending-alpha-publication" {
        let expected = pending_beta_readiness(&version)?;
        if ReleaseStage::from_version(&version) != Ok(ReleaseStage::Alpha) || ledger != expected {
            return Err(
                "pending Beta readiness state is not canonical for the active Alpha".to_owned(),
            );
        }
        println!("beta readiness: ok (pending public Alpha evidence)");
        return Ok(());
    }
    if ledger["schema"] != BETA_READINESS_SCHEMA {
        return Err(
            "Beta readiness ledger does not identify the qualified native Alpha".to_owned(),
        );
    }
    let alpha_tag = required_string(&ledger["alpha_release"], "tag", "Alpha release")?;
    validate_alpha_baseline_tag(&version, alpha_tag)?;
    validate_lower_hex(
        "Beta readiness Alpha source commit",
        required_string(&ledger["alpha_release"], "source_commit", "Alpha release")?,
        40,
    )?;
    if alpha_tag.ends_with("-alpha.7")
        && (ledger["status"] != "qualified"
            || ledger["alpha_release"]["release_run"]
                .as_u64()
                .filter(|run| *run > 0)
                .is_none()
            || required_string(&ledger["alpha_release"], "release_url", "Alpha release")?
                != format!("https://github.com/jxpeng98/CanISend/releases/tag/{alpha_tag}")
            || ledger["contracts"] != beta_readiness_contracts(&repository_root())?)
    {
        return Err(
            "Alpha.7 Beta readiness must bind its tag, source, public run/URL, v3 contracts, and both Pack digests"
                .to_owned(),
        );
    }
    let audited_at = ledger["audited_at"]
        .as_str()
        .filter(|value| value.ends_with('Z') && value.contains('T'))
        .ok_or_else(|| "Beta readiness ledger has no UTC audit timestamp".to_owned())?;
    if ledger["default_telemetry"] != false {
        return Err("Beta readiness ledger must preserve disabled default telemetry".to_owned());
    }
    if ledger["github_issue_snapshot"]["open_issue_count"] != 0
        || ledger["unresolved_release_blockers"]
            .as_array()
            .is_none_or(|entries| !entries.is_empty())
    {
        return Err("Beta readiness ledger contains unresolved Alpha blockers".to_owned());
    }
    let expected_classes = BTreeSet::from([
        "data-loss",
        "protocol-compatibility",
        "rendering-corruption",
        "security-privacy",
    ]);
    let entries = ledger["blocker_classes"]
        .as_array()
        .ok_or_else(|| "Beta readiness blocker_classes must be an array".to_owned())?;
    let mut actual_classes = BTreeSet::new();
    for entry in entries {
        let class = entry["class"]
            .as_str()
            .ok_or_else(|| "Beta readiness blocker class is missing".to_owned())?;
        if !actual_classes.insert(class) {
            return Err(format!("duplicate Beta readiness blocker class `{class}`"));
        }
        if !matches!(entry["status"].as_str(), Some("clear" | "resolved")) {
            return Err(format!(
                "Beta readiness blocker class `{class}` is not clear"
            ));
        }
        if entry["open_issue_numbers"]
            .as_array()
            .is_none_or(|issues| !issues.is_empty())
        {
            return Err(format!(
                "Beta readiness blocker class `{class}` contains open issues"
            ));
        }
        if entry["evidence"].as_array().is_none_or(|evidence| {
            evidence.is_empty() || evidence.iter().any(|item| !item.is_string())
        }) {
            return Err(format!(
                "Beta readiness blocker class `{class}` has no evidence"
            ));
        }
    }
    if actual_classes != expected_classes {
        return Err(format!(
            "Beta readiness blocker classes differ: expected {expected_classes:?}, found {actual_classes:?}"
        ));
    }
    println!(
        "beta readiness: ok ({} blocker classes, audited {audited_at})",
        actual_classes.len()
    );
    Ok(())
}

fn check_beta_contract_freeze() -> Result<(), String> {
    let path = repository_root().join("release/beta-contract-freeze.json");
    let body = fs::read_to_string(&path).map_err(|error| {
        format!(
            "Beta contract freeze is missing at {}: {error}",
            path.display()
        )
    })?;
    let actual: Value = serde_json::from_str(&body)
        .map_err(|error| format!("Beta contract freeze is invalid JSON: {error}"))?;
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    if actual["status"] == "pending-alpha-publication" {
        let expected = pending_beta_contract_freeze(&version)?;
        if ReleaseStage::from_version(&version) != Ok(ReleaseStage::Alpha) || actual != expected {
            return Err(
                "pending Beta contract state is not canonical for the active Alpha".to_owned(),
            );
        }
        println!("beta contract freeze: ok (pending public Alpha baseline)");
        return Ok(());
    }
    let expected = build_beta_contract_freeze()?;
    if actual != expected {
        return Err(
            "Beta agent/workspace contract freeze drifted; review the compatibility impact and regenerate with \
             `cargo run -p xtask -- release freeze-candidate`"
                .to_owned(),
        );
    }
    println!(
        "beta contract freeze: ok ({} schemas, migrations frozen through {})",
        expected["agent"]["public_schema_files"], FROZEN_MIGRATIONS_THROUGH
    );
    Ok(())
}

fn check_signing_policy() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/signing-policy.json");
    let actual: Value =
        serde_json::from_slice(&fs::read(&path).map_err(|error| {
            format!("signing policy is missing at {}: {error}", path.display())
        })?)
        .map_err(|error| format!("signing policy is invalid JSON: {error}"))?;
    let expected = json!({
        "schema": SIGNING_POLICY_SCHEMA,
        "trust_tier": "community-build",
        "stage_boundary": {
            "alpha_may_be_unsigned": true,
            "beta_rc_stable_require_platform_integrity_signatures": true,
            "external_credentials_required": false,
            "missing_platform_tooling": "fail-closed"
        },
        "macos": {
            "targets": ["aarch64-apple-darwin", "x86_64-apple-darwin"],
            "service": "codesign-adhoc",
            "identity": "-",
            "code_identifier": "io.github.jxpeng98.canisend",
            "hardened_runtime": true,
            "secure_timestamp": false,
            "notarized": false,
            "operating_system_trust": "untrusted",
            "required_secrets": [],
            "required_variables": []
        },
        "windows": {
            "targets": ["x86_64-pc-windows-msvc"],
            "service": "powershell-self-signed-authenticode",
            "trust_model": "self-signed-untrusted",
            "certificate_subject": "CN=CanISend Community Build",
            "certificate_valid_days": 3650,
            "private_key_exportable": false,
            "key_algorithm": "RSA",
            "key_length": 3072,
            "file_digest": "SHA256",
            "timestamp_present": false,
            "required_secrets": [],
            "required_variables": []
        },
        "linux": {
            "targets": ["x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"],
            "code_signing": "none",
            "integrity": ["sha256sums", "github-oidc-provenance"]
        }
    });
    if actual != expected {
        return Err(
            "release signing policy drifted from the fail-closed community trust contract"
                .to_owned(),
        );
    }
    let readiness_path = root.join("scripts/check_signing_readiness.sh");
    let audit_path = root.join("scripts/audit_community_signing_configuration.sh");
    let macos_path = root.join("scripts/sign_macos_adhoc.sh");
    let windows_path = root.join("scripts/sign_windows_self_signed.ps1");
    let operations_path = root.join("docs/release/signing-operations.md");
    let readiness = fs::read_to_string(&readiness_path)
        .map_err(|error| format!("release signing readiness script is missing: {error}"))?;
    let audit = fs::read_to_string(&audit_path)
        .map_err(|error| format!("community signing configuration audit is missing: {error}"))?;
    let macos = fs::read_to_string(&macos_path)
        .map_err(|error| format!("macOS signing script is missing: {error}"))?;
    let windows = fs::read_to_string(&windows_path)
        .map_err(|error| format!("Windows signing verifier is missing: {error}"))?;
    let operations = fs::read_to_string(&operations_path)
        .map_err(|error| format!("release signing operations guide is missing: {error}"))?;
    for required in [
        "release/signing-policy.json",
        "alpha|beta|rc|stable",
        "requires no external credentials",
        "fail closed on missing tooling",
    ] {
        if !readiness.contains(required) {
            return Err(format!(
                "release signing readiness script is missing `{required}`"
            ));
        }
    }
    for required in [
        "external credentials are not required",
        "sign_macos_adhoc.sh",
        "sign_windows_self_signed.ps1",
    ] {
        if !audit.contains(required) {
            return Err(format!(
                "community signing configuration audit is missing `{required}`"
            ));
        }
    }
    for required in [
        "--identifier io.github.jxpeng98.canisend",
        "--options runtime",
        "--sign -",
        "--timestamp=none",
        "Signature=",
        "Authority=",
        "com.apple.security.get-task-allow",
        "canisend.code-signing-evidence/v2",
        "gatekeeper_trusted_publisher: false",
    ] {
        if !macos.contains(required) {
            return Err(format!(
                "macOS ad-hoc signing script is missing `{required}`"
            ));
        }
    }
    for required in [
        "New-SelfSignedCertificate",
        "Set-AuthenticodeSignature",
        "KeyExportPolicy NonExportable",
        "[System.IO.FileAttributes]::ReparsePoint",
        "Remove-Item",
        "NotTrusted",
        "UnknownError",
        "canisend.code-signing-evidence/v2",
        "certificate_trusted = $false",
        "timestamp_present = $false",
        "service = \"powershell-self-signed-authenticode\"",
    ] {
        if !windows.contains(required) {
            return Err(format!(
                "Windows self-signed signing script is missing `{required}`"
            ));
        }
    }
    for required in [
        "Community signing",
        "not a publisher identity",
        "Gatekeeper",
        "SmartScreen",
        "GitHub build provenance",
    ] {
        if !operations.contains(required) {
            return Err(format!(
                "release signing operations guide is missing `{required}`"
            ));
        }
    }
    let workflow_path = root.join(".github/workflows/release.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("release workflow is missing: {error}"))?;
    for forbidden in [
        "APPLE_DEVELOPER_ID_P12_BASE64",
        "APPLE_NOTARY_KEY_P8_BASE64",
        "AZURE_ARTIFACT_SIGNING_ACCOUNT",
        "azure/artifact-signing-action",
        "azure/login@",
    ] {
        if workflow.contains(forbidden) {
            return Err(format!(
                "release workflow still depends on paid signing configuration `{forbidden}`"
            ));
        }
    }
    for required in [
        "release/signing-policy.json",
        "check_signing_readiness.sh",
        "sign_macos_adhoc.sh",
        "sign_windows_self_signed.ps1",
        "bind-signing-evidence",
        "Ad-hoc sign macOS executable",
        "Self-sign Windows executable with Authenticode",
        "attest-build-provenance@0f67c3f4856b2e3261c31976d6725780e5e4c373",
        "id-token: write",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "release workflow is missing signing gate `{required}`"
            ));
        }
    }
    println!("signing policy: ok (community macOS ad-hoc + Windows self-signed Authenticode)");
    Ok(())
}

fn check_release_line_history() -> Result<(), String> {
    let root = repository_root();
    let policy_path = root.join("release/release-line-policy.json");
    let policy: Value = serde_json::from_slice(&fs::read(&policy_path).map_err(|error| {
        format!(
            "release-line policy is missing at {}: {error}",
            policy_path.display()
        )
    })?)
    .map_err(|error| format!("release-line policy is invalid JSON: {error}"))?;
    let expected_sources = [
        "release/RELEASE_NOTES.md",
        "release/beta-contract-freeze.json",
        "release/beta-readiness.json",
        "release/feature-freeze-exceptions.json",
        "release/feedback-snapshot.json",
        "release/qualification-ledger.json",
        "release/support-policy.json",
    ];
    let expected_policy = json!({
        "schema": RELEASE_LINE_POLICY_SCHEMA,
        "command": {
            "name": "cargo run -p xtask --locked -- release activate-line",
            "dry_run_default": true,
            "write_flag": "--write",
            "clean_worktree_required_for_write": true,
            "transactional_rollback": true
        },
        "target": {
            "tag": "v1.0.0-alpha.1",
            "new_line_prerelease": "X.Y.0-alpha.1"
        },
        "history": {
            "schema": RELEASE_HISTORY_SCHEMA,
            "source_line": "0.7",
            "destination": "release/history/0.7",
            "copied_files": expected_sources,
            "referenced_trees": ["packaging/candidates/v0.7.0-alpha.1"]
        },
        "external_actions": {
            "create_tag": false,
            "push": false,
            "publish_release": false,
            "modify_package_repository": false
        }
    });
    if policy != expected_policy {
        return Err(
            "release-line policy differs from the fail-closed activation contract".to_owned(),
        );
    }
    let runbook_path = root.join("docs/release/release-line-activation.md");
    let runbook = fs::read_to_string(&runbook_path)
        .map_err(|error| format!("release-line activation runbook is missing: {error}"))?;
    check_local_markdown_links(&root, &runbook_path, &runbook)?;
    for required in [
        "release activate-line v1.0.0-alpha.1",
        "--write",
        "release/history/0.7/manifest.json",
        "clean worktree",
        "does not create a tag",
        "transaction",
    ] {
        if !runbook.contains(required) {
            return Err(format!(
                "release-line activation runbook is missing `{required}`"
            ));
        }
    }

    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let history_root = root.join("release/history/0.7");
    if (version.major, version.minor) == (0, 7) {
        if history_root.exists() {
            return Err(
                "0.7 active source must not contain an already activated 0.7 history".to_owned(),
            );
        }
        println!("release-line history: ok (1.0 activation planned, 0.7 still active)");
        return Ok(());
    }
    validate_release_history(&root, &history_root, "0.7", &expected_sources)?;
    println!("release-line history: ok (0.7 archive + candidate-tree digest)");
    Ok(())
}

fn validate_release_history(
    root: &Path,
    history_root: &Path,
    release_line: &str,
    expected_sources: &[&str],
) -> Result<(), String> {
    let manifest_path = history_root.join("manifest.json");
    reject_symlink(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| {
        format!(
            "release history manifest is missing at {}: {error}",
            manifest_path.display()
        )
    })?)
    .map_err(|error| format!("release history manifest is invalid JSON: {error}"))?;
    if manifest["schema"] != RELEASE_HISTORY_SCHEMA || manifest["release_line"] != release_line {
        return Err("release history manifest identity is invalid".to_owned());
    }
    let archived_version = Version::parse(
        manifest["archived_from_version"]
            .as_str()
            .ok_or_else(|| "release history has no archived source version".to_owned())?,
    )
    .map_err(|error| format!("release history source version is invalid: {error}"))?;
    if format!("{}.{}", archived_version.major, archived_version.minor) != release_line {
        return Err("release history source version differs from its release line".to_owned());
    }
    let source_commit = required_string(&manifest, "activation_source_commit", "release history")?;
    validate_lower_hex(
        "release history activation source commit",
        source_commit,
        40,
    )?;
    run_git(
        root,
        &["cat-file", "-e", &format!("{source_commit}^{{commit}}")],
    )?;
    run_git(
        root,
        &["merge-base", "--is-ancestor", source_commit, "HEAD"],
    )?;

    let entries = manifest["files"]
        .as_array()
        .ok_or_else(|| "release history files must be an array".to_owned())?;
    if entries.len() != expected_sources.len() {
        return Err("release history file inventory is incomplete".to_owned());
    }
    for (entry, source_path) in entries.iter().zip(expected_sources) {
        let name = Path::new(source_path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "release history source filename is invalid".to_owned())?;
        let archive_path = format!("release/history/{release_line}/{name}");
        let digest = required_string(entry, "sha256", "release history file")?;
        validate_lower_hex("release history file digest", digest, 64)?;
        let canonical = json!({
            "archive_path": archive_path,
            "sha256": digest,
            "source_path": source_path
        });
        if entry != &canonical || sha256_file(&root.join(&archive_path))? != digest {
            return Err(format!(
                "release history archive differs for `{source_path}`"
            ));
        }
    }
    let tree_digest = digest_regular_file_tree(&root.join("packaging/candidates/v0.7.0-alpha.1"))?;
    let expected_references = json!([{
        "kind": "repository-tree",
        "path": "packaging/candidates/v0.7.0-alpha.1",
        "tree_sha256": tree_digest
    }]);
    if manifest["references"] != expected_references {
        return Err("release history candidate-tree reference drifted".to_owned());
    }
    let canonical = json!({
        "schema": RELEASE_HISTORY_SCHEMA,
        "release_line": release_line,
        "archived_from_version": archived_version.to_string(),
        "activation_source_commit": source_commit,
        "files": entries,
        "references": expected_references
    });
    if manifest != canonical {
        return Err("release history manifest contains unknown or noncanonical fields".to_owned());
    }
    Ok(())
}

fn check_support_policy() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/support-policy.json");
    let actual: Value =
        serde_json::from_slice(&fs::read(&path).map_err(|error| {
            format!("support policy is missing at {}: {error}", path.display())
        })?)
        .map_err(|error| format!("support policy is invalid JSON: {error}"))?;
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let publication_status = support_policy_publication_status(&version);
    let target_count = release_targets()?.len();
    let expected = build_support_policy(&version)?;
    if actual != expected {
        return Err(
            "release/support-policy.json differs from the current product, contract, workspace, or platform policy"
                .to_owned(),
        );
    }

    let document_path = root.join("docs/release/support-policy.md");
    let document = fs::read_to_string(&document_path).map_err(|error| {
        format!(
            "support policy documentation is missing at {}: {error}",
            document_path.display()
        )
    })?;
    check_local_markdown_links(&root, &document_path, &document)?;
    let release_line = format!("{}.{}", version.major, version.minor);
    let support_heading = format!("# CanISend {release_line} Support Policy");
    for required in [
        support_heading.as_str(),
        AGENT_V4_PROTOCOL,
        AGENT_V4_SCHEMA_VERSION,
        canisend_resources::AGENT_HOST_RESOURCE_FORMAT,
        WORKSPACE_V4_FORMAT,
        "current-only-until-superseded",
        "current-minor-latest-patch",
        "restore into a new path",
        "Linux arm64",
        "No service-level agreement",
    ] {
        if !document.contains(required) {
            return Err(format!(
                "support policy documentation is missing `{required}`"
            ));
        }
    }
    println!("support policy: ok ({publication_status}, {target_count} targets)");
    Ok(())
}

fn build_support_policy(version: &Version) -> Result<Value, String> {
    let target_count = release_targets()?.len();
    Ok(json!({
        "schema": SUPPORT_POLICY_SCHEMA,
        "publication_status": support_policy_publication_status(version),
        "release_line": format!("{}.{}", version.major, version.minor),
        "version_support": {
            "prerelease": "current-only-until-superseded",
            "stable": "current-minor-latest-patch",
            "long_term_support": false,
            "service_level_agreement": false,
            "python_0_6_line": "archived-unsupported"
        },
        "contracts": {
            "agent_protocol": AGENT_V4_PROTOCOL,
            "public_schema_version": AGENT_V4_SCHEMA_VERSION,
            "resource_format": canisend_resources::AGENT_HOST_RESOURCE_FORMAT,
            "beta_freeze": "release/beta-contract-freeze.json",
            "breaking_agent_change": "new-protocol-and-schema-major"
        },
        "workspace": {
            "format": WORKSPACE_V4_FORMAT,
            "current_database_schema_version": declared_database_schema_version()?,
            "frozen_migrations_through": FROZEN_MIGRATIONS_THROUGH,
            "migration_policy": "append-only",
            "future_schema": "reject-without-mutation",
            "downgrade": "restore-verified-pre-upgrade-backup-to-new-path"
        },
        "platforms": {
            "authority": "release/targets.json",
            "target_count": target_count,
            "linux_arm64": format!("unsupported-in-{}.{}", version.major, version.minor),
            "runtime_requirements": {
                "python": false,
                "node": false,
                "java": false,
                "external_typst": false,
                "external_sqlite": false
            }
        },
        "host_assets": {
            "codex": "generated-by-installed-binary",
            "claude": "generated-by-installed-binary",
            "refresh_after_upgrade": true,
            "private_workspace_bodies_included_by_default": false
        },
        "security": {
            "reporting": "SECURITY.md",
            "default_telemetry": false,
            "private_issue_content": "prohibited"
        }
    }))
}

fn support_policy_publication_status(version: &Version) -> &'static str {
    if version.pre.is_empty() {
        "published"
    } else {
        "pre-stable-draft"
    }
}

fn check_release_feedback() -> Result<(), String> {
    let root = repository_root();
    let snapshot_path = root.join("release/feedback-snapshot.json");
    let snapshot: Value = serde_json::from_slice(&fs::read(&snapshot_path).map_err(|error| {
        format!(
            "release feedback snapshot is missing at {}: {error}",
            snapshot_path.display()
        )
    })?)
    .map_err(|error| format!("release feedback snapshot is invalid JSON: {error}"))?;
    let roadmap = feedback_roadmap_relative(&snapshot)?;
    check_release_feedback_files(&snapshot_path, &root.join(roadmap))?;
    check_final_rc_feedback_binding(&root, &snapshot)
}

fn feedback_roadmap_relative(snapshot: &Value) -> Result<String, String> {
    let relative = required_string(
        &snapshot["next_roadmap"],
        "path",
        "release feedback next roadmap",
    )?;
    let path = Path::new(relative);
    if path.is_absolute()
        || relative.contains('\\')
        || path.extension().and_then(|value| value.to_str()) != Some("md")
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
        || !relative.starts_with("docs/superpowers/plans/")
    {
        return Err(
            "release feedback next-roadmap path is unsafe or outside the plan registry".to_owned(),
        );
    }
    Ok(relative.to_owned())
}

fn check_final_rc_feedback_binding(root: &Path, snapshot: &Value) -> Result<(), String> {
    if snapshot["snapshot_stage"] != "rc" {
        return Ok(());
    }
    let ledger: Value = serde_json::from_slice(
        &fs::read(root.join("release/qualification-ledger.json"))
            .map_err(|error| format!("qualification ledger is missing for RC feedback: {error}"))?,
    )
    .map_err(|error| format!("qualification ledger is invalid JSON: {error}"))?;
    let candidates = ledger["release_candidates"]
        .as_array()
        .ok_or_else(|| "RC feedback has no qualification candidate list".to_owned())?;
    let latest = candidates
        .last()
        .ok_or_else(|| "RC feedback requires a recorded release candidate".to_owned())?;
    let (tag, source, run) =
        validate_qualification_release(latest, ReleaseStage::ReleaseCandidate, "latest RC")?;
    if snapshot["release"]["tag"] != tag {
        return Err(format!(
            "final feedback must bind the latest recorded RC `{tag}`, not `{}`",
            snapshot["release"]["tag"].as_str().unwrap_or("missing")
        ));
    }
    let workspace_path = root.join("Cargo.toml");
    if workspace_path.is_file() {
        let workspace: toml::Value = fs::read_to_string(&workspace_path)
            .map_err(|error| format!("could not read workspace version for RC feedback: {error}"))?
            .parse()
            .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
        let version = Version::parse(
            workspace["workspace"]["package"]["version"]
                .as_str()
                .ok_or_else(|| "workspace manifest has no package version".to_owned())?,
        )
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
        if ReleaseStage::from_version(&version) == Ok(ReleaseStage::ReleaseCandidate)
            && tag != format!("v{version}")
        {
            return Err(format!(
                "RC feedback for `{tag}` is stale after preparing workspace `{version}`"
            ));
        }
    }
    if let Some(review) = ledger["release_notes"]["review"].as_object()
        && (review.get("tag") != Some(&Value::String(tag.clone()))
            || review.get("source_commit") != Some(&Value::String(source))
            || review.get("signed_matrix_run") != Some(&Value::Number(run.into())))
    {
        return Err(
            "final feedback and release-notes review do not bind the same latest RC".to_owned(),
        );
    }
    Ok(())
}

fn check_release_feedback_files(
    snapshot_path: &Path,
    roadmap_candidate: &Path,
) -> Result<(), String> {
    let root = repository_root();
    let snapshot: Value = serde_json::from_slice(&fs::read(snapshot_path).map_err(|error| {
        format!(
            "release feedback snapshot is missing at {}: {error}",
            snapshot_path.display()
        )
    })?)
    .map_err(|error| format!("release feedback snapshot is invalid JSON: {error}"))?;
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    if snapshot["status"] == "pending-alpha-publication" {
        if ReleaseStage::from_version(&version) != Ok(ReleaseStage::Alpha)
            || !pending_release_feedback_is_canonical(&snapshot, &version)?
        {
            return Err(
                "pending release feedback is not canonical for the active Alpha".to_owned(),
            );
        }
        let roadmap_path = feedback_roadmap_relative(&snapshot)?;
        let roadmap = fs::read_to_string(roadmap_candidate).map_err(|error| {
            format!(
                "active release roadmap is missing at {}: {error}",
                roadmap_candidate.display()
            )
        })?;
        check_local_markdown_links(&root, &root.join(&roadmap_path), &roadmap)?;
        if !roadmap.contains("**Status:** Active") {
            return Err("active release roadmap has no Active status marker".to_owned());
        }
        println!("release feedback: ok (pending public Alpha snapshot)");
        return Ok(());
    }
    if snapshot["schema"] != FEEDBACK_SNAPSHOT_SCHEMA
        || snapshot["default_telemetry"] != false
        || snapshot["privacy_boundary"] != "public-metadata-only"
    {
        return Err("release feedback snapshot identity or privacy boundary is invalid".to_owned());
    }
    let captured_at = snapshot["captured_at"]
        .as_str()
        .filter(|value| value.contains('T') && value.ends_with('Z'))
        .ok_or_else(|| "release feedback snapshot has no UTC captured_at".to_owned())?;
    let snapshot_stage = snapshot["snapshot_stage"]
        .as_str()
        .ok_or_else(|| "release feedback snapshot has no stage".to_owned())?;
    if !matches!(snapshot_stage, "alpha-baseline" | "beta" | "rc") {
        return Err(format!(
            "unsupported release feedback snapshot stage `{snapshot_stage}`"
        ));
    }
    let release = &snapshot["release"];
    let repository = env!("CARGO_PKG_REPOSITORY").trim_start_matches("https://github.com/");
    let published_at = release["published_at"]
        .as_str()
        .filter(|value| value.contains('T') && value.ends_with('Z'))
        .ok_or_else(|| "release feedback snapshot has no release publication time".to_owned())?;
    let release_tag = release["tag"]
        .as_str()
        .and_then(|value| value.strip_prefix('v'))
        .ok_or_else(|| "release feedback snapshot has no valid release tag".to_owned())?;
    let release_version = Version::parse(release_tag)
        .map_err(|error| format!("release feedback tag is invalid SemVer: {error}"))?;
    let expected_prerelease_prefix = match snapshot_stage {
        "alpha-baseline" => "alpha.",
        "beta" => "beta.",
        "rc" => "rc.",
        _ => unreachable!("snapshot stage was validated"),
    };
    if release["repository"] != repository
        || !release_version
            .pre
            .as_str()
            .starts_with(expected_prerelease_prefix)
        || published_at > captured_at
    {
        return Err("release feedback snapshot does not match its public release stage".to_owned());
    }

    let feedback = &snapshot["public_feedback"];
    let open = feedback["open_issue_count"]
        .as_u64()
        .ok_or_else(|| "release feedback open issue count is invalid".to_owned())?;
    let closed = feedback["closed_issue_count"]
        .as_u64()
        .ok_or_else(|| "release feedback closed issue count is invalid".to_owned())?;
    let total = feedback["total_issue_count"]
        .as_u64()
        .ok_or_else(|| "release feedback total issue count is invalid".to_owned())?;
    let issue_numbers = feedback["issue_numbers"]
        .as_array()
        .ok_or_else(|| "release feedback issue_numbers must be an array".to_owned())?;
    let unique_issue_numbers = issue_numbers
        .iter()
        .filter_map(Value::as_u64)
        .collect::<BTreeSet<_>>();
    if open + closed != total
        || usize::try_from(total).ok() != Some(issue_numbers.len())
        || unique_issue_numbers.len() != issue_numbers.len()
        || unique_issue_numbers.contains(&0)
    {
        return Err("release feedback issue counts are inconsistent".to_owned());
    }

    let downloads = &snapshot["release_downloads"];
    let asset_count = downloads["asset_count"]
        .as_u64()
        .ok_or_else(|| "release feedback asset count is invalid".to_owned())?;
    let total_downloads = downloads["total_downloads"]
        .as_u64()
        .ok_or_else(|| "release feedback total downloads are invalid".to_owned())?;
    let native_archive_count = downloads["native_archive_count"]
        .as_u64()
        .ok_or_else(|| "release feedback native archive count is invalid".to_owned())?;
    let native_archive_downloads = downloads["native_archive_downloads"]
        .as_u64()
        .ok_or_else(|| "release feedback native archive downloads are invalid".to_owned())?;
    if native_archive_count > asset_count
        || native_archive_downloads > total_downloads
        || downloads["maintainer_verification_included"] != true
    {
        return Err("release download evidence overclaims independent adoption".to_owned());
    }

    let findings = snapshot["qualification_findings"]
        .as_array()
        .filter(|findings| !findings.is_empty())
        .ok_or_else(|| "release feedback snapshot has no qualification findings".to_owned())?;
    for finding in findings {
        for field in ["id", "evidence", "resolution"] {
            if finding[field]
                .as_str()
                .is_none_or(|value| value.trim().is_empty())
            {
                return Err(format!(
                    "release qualification finding is missing `{field}`"
                ));
            }
        }
    }

    let roadmap = &snapshot["next_roadmap"];
    let roadmap_path = feedback_roadmap_relative(&snapshot)?;
    let roadmap_status = roadmap["status"]
        .as_str()
        .ok_or_else(|| "release feedback snapshot has no next-roadmap status".to_owned())?;
    let (required_stage, required_status) =
        feedback_publication_requirements(&version, snapshot_stage);
    if required_stage.is_some_and(|required| snapshot_stage != required)
        || roadmap_status != required_status
    {
        return Err(format!(
            "release feedback snapshot must be stage {} with roadmap status `{required_status}` for version {version}",
            required_stage.unwrap_or("alpha, beta, or rc")
        ));
    }
    let roadmap_file = root.join(&roadmap_path);
    let roadmap_body = fs::read_to_string(roadmap_candidate).map_err(|error| {
        format!(
            "next roadmap candidate is missing at {}: {error}",
            roadmap_candidate.display()
        )
    })?;
    check_local_markdown_links(&root, &roadmap_file, &roadmap_body)?;
    let required_status_marker = match required_status {
        "published" => "**Status:** Published",
        "reviewed" => "**Status:** Reviewed",
        _ => "**Status:** Draft",
    };
    if !roadmap_body.contains(required_status_marker) {
        return Err(format!(
            "next roadmap is missing status marker `{required_status_marker}`"
        ));
    }
    let normalized_roadmap = roadmap_body
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for expected in [
        format!(
            "Snapshot stage: `{snapshot_stage}`; captured at `{captured_at}`; public GitHub issues: **{open}** open, **{closed}** closed, **{total}** total."
        ),
        format!(
            "Release: `v{release_tag}`; public assets: **{asset_count}**; downloads: **{total_downloads}** total, **{native_archive_downloads}** across **{native_archive_count}** native archives."
        ),
    ] {
        if !normalized_roadmap.contains(&expected) {
            return Err(format!(
                "next roadmap measured baseline is inconsistent with feedback snapshot: missing `{expected}`"
            ));
        }
    }
    for required in [
        "Measured baseline",
        "<!-- release-feedback-measured:start -->",
        "<!-- release-feedback-measured:end -->",
        "maintainer verification",
        "Beta/RC refresh gate",
    ] {
        if !roadmap_body.contains(required) {
            return Err(format!("next roadmap is missing `{required}`"));
        }
    }
    if total == 0 && !roadmap_body.contains("No public user issue") {
        return Err("zero-issue roadmap baseline must state `No public user issue`".to_owned());
    }
    let refresh = fs::read_to_string(root.join("scripts/refresh_release_feedback.sh"))
        .map_err(|error| format!("release feedback refresh script is missing: {error}"))?;
    for required in [
        "gh api --paginate --slurp",
        "{number, state}",
        "gh release view",
        "verify-feedback-candidate",
        "privacy_boundary",
        "--write",
    ] {
        if !refresh.contains(required) {
            return Err(format!(
                "release feedback refresh script is missing `{required}`"
            ));
        }
    }
    println!(
        "release feedback: ok ({snapshot_stage}, {total} public issues, {total_downloads} downloads, captured {captured_at})"
    );
    Ok(())
}

fn feedback_publication_requirements(
    version: &Version,
    snapshot_stage: &str,
) -> (Option<&'static str>, &'static str) {
    if version.pre.is_empty() {
        (Some("rc"), "published")
    } else if ReleaseStage::from_version(version) == Ok(ReleaseStage::ReleaseCandidate)
        && snapshot_stage == "rc"
    {
        (Some("rc"), "reviewed")
    } else {
        (None, "draft")
    }
}

fn print_release_status() -> Result<(), String> {
    let status = derive_release_status(&repository_root())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&status)
            .map_err(|error| format!("could not serialize derived release status: {error}"))?
    );
    Ok(())
}

fn check_release_status() -> Result<(), String> {
    let status = derive_release_status(&repository_root())?;
    let drift = &status["drift"];
    let count = drift["count"]
        .as_u64()
        .ok_or_else(|| "derived release status has no drift count".to_owned())?;
    let blocking = drift["blocking_count"]
        .as_u64()
        .ok_or_else(|| "derived release status has no blocking drift count".to_owned())?;
    println!("release status: ok ({count} drift items, {blocking} stage-blocking)");
    Ok(())
}

fn derive_release_status(root: &Path) -> Result<Value, String> {
    let (workspace_version, workspace_license) = release_status_workspace_identity(root)?;
    let sources = ReleaseStatusSources {
        git: release_status_git_facts(root, &workspace_version)?,
        workspace_version,
        workspace_license,
        qualification: read_release_status_json(
            root,
            "release/qualification-ledger.json",
            "release qualification ledger",
        )?,
        support: read_release_status_json(root, "release/support-policy.json", "support policy")?,
        targets: read_release_status_json(root, "release/targets.json", "release targets")?,
        alpha_package: read_release_status_json(
            root,
            "release/alpha-package-contract.json",
            "Alpha package contract",
        )?,
        beta_readiness: read_release_status_json(
            root,
            "release/beta-readiness.json",
            "Beta readiness",
        )?,
        beta_freeze: read_release_status_json(
            root,
            "release/beta-contract-freeze.json",
            "Beta contract freeze",
        )?,
        feedback: read_release_status_json(
            root,
            "release/feedback-snapshot.json",
            "release feedback snapshot",
        )?,
        cli_gui_parity: read_release_status_json(
            root,
            "docs/contracts/cli-gui-parity-v1.json",
            "CLI/GUI parity contract",
        )?,
        svelte_parity: read_release_status_json(
            root,
            "docs/contracts/svelte-parity-v1.json",
            "Svelte parity contract",
        )?,
        signing: read_release_status_json(root, "release/signing-policy.json", "signing policy")?,
    };
    build_release_status_document(&sources)
}

fn read_release_status_json(root: &Path, relative: &str, context: &str) -> Result<Value, String> {
    let path = root.join(relative);
    serde_json::from_slice(
        &fs::read(&path)
            .map_err(|error| format!("{context} is missing at {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("{context} is invalid JSON: {error}"))
}

fn release_status_workspace_identity(root: &Path) -> Result<(Version, String), String> {
    let path = root.join("Cargo.toml");
    let manifest: toml::Value = fs::read_to_string(&path)
        .map_err(|error| {
            format!(
                "workspace manifest is missing at {}: {error}",
                path.display()
            )
        })?
        .parse()
        .map_err(|error| format!("workspace manifest is invalid TOML: {error}"))?;
    let package = &manifest["workspace"]["package"];
    let version = Version::parse(
        package["version"]
            .as_str()
            .ok_or_else(|| "workspace manifest has no package version".to_owned())?,
    )
    .map_err(|error| format!("workspace version is invalid SemVer: {error}"))?;
    let license = package["license"]
        .as_str()
        .filter(|license| !license.is_empty())
        .ok_or_else(|| "workspace manifest has no package license".to_owned())?
        .to_owned();
    Ok((version, license))
}

fn release_status_git_facts(
    root: &Path,
    workspace_version: &Version,
) -> Result<ReleaseStatusGitFacts, String> {
    let head_commit = run_git_lines(root, &["rev-parse", "HEAD"])?
        .into_iter()
        .next()
        .ok_or_else(|| "Git HEAD did not resolve to a commit".to_owned())?;
    validate_lower_hex("release-status HEAD commit", &head_commit, 40)?;
    let pattern = format!("v{}.{}.*", workspace_version.major, workspace_version.minor);
    let tags = run_git_lines(root, &["tag", "--merged", "HEAD", "--list", &pattern])?;
    let (public_version, public_tag) = tags
        .into_iter()
        .filter_map(|tag| {
            let version = tag
                .strip_prefix('v')
                .and_then(|value| Version::parse(value).ok())?;
            ReleaseStage::from_version(&version).ok()?;
            Some((version, tag))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .ok_or_else(|| format!("no reachable release tag matches `{pattern}`"))?;
    let public_commit = run_git_lines(root, &["rev-list", "-n", "1", &public_tag])?
        .into_iter()
        .next()
        .ok_or_else(|| format!("release tag `{public_tag}` did not resolve to a commit"))?;
    validate_lower_hex(
        "release-status public checkpoint commit",
        &public_commit,
        40,
    )?;
    let revision_range = format!("{public_tag}..HEAD");
    let source_commits_ahead = run_git_lines(root, &["rev-list", "--count", &revision_range])?
        .into_iter()
        .next()
        .ok_or_else(|| "Git did not report commits since the public checkpoint".to_owned())?
        .parse::<u64>()
        .map_err(|error| format!("Git commit count is invalid: {error}"))?;
    let worktree_dirty =
        !run_git_lines(root, &["status", "--porcelain", "--untracked-files=normal"])?.is_empty();
    Ok(ReleaseStatusGitFacts {
        head_commit,
        worktree_dirty,
        public_tag,
        public_version,
        public_commit,
        source_commits_ahead,
    })
}

fn build_release_status_document(sources: &ReleaseStatusSources) -> Result<Value, String> {
    let version = &sources.workspace_version;
    let stage = ReleaseStage::from_version(version)?;
    let release_line = format!("{}.{}", version.major, version.minor);
    let source_tag = format!("v{version}");

    if sources.qualification["schema"] != RELEASE_QUALIFICATION_SCHEMA {
        return Err("release-status qualification ledger schema is invalid".to_owned());
    }
    let ledger_stage = required_string(
        &sources.qualification,
        "workspace_stage",
        "release-status qualification ledger",
    )?;
    let qualification_status = required_string(
        &sources.qualification,
        "status",
        "release-status qualification ledger",
    )?;
    if ledger_stage != stage.as_str()
        || qualification_status != qualification_status_for_stage(stage)
    {
        return Err(format!(
            "release-status stage disagreement: Cargo is `{}`, ledger is `{ledger_stage}` / `{qualification_status}`",
            stage.as_str()
        ));
    }
    let stable_authorized = sources.qualification["stable_authorized"]
        .as_bool()
        .ok_or_else(|| {
            "release-status qualification ledger has no Stable authorization".to_owned()
        })?;

    if sources.support["schema"] != SUPPORT_POLICY_SCHEMA {
        return Err("release-status support policy schema is invalid".to_owned());
    }
    let support_line = required_string(&sources.support, "release_line", "support policy")?;
    if support_line != release_line {
        return Err(format!(
            "release-status source/support disagreement: Cargo line is `{release_line}`, support line is `{support_line}`"
        ));
    }
    let support_publication = required_string(
        &sources.support,
        "publication_status",
        "release-status support policy",
    )?;

    if sources.targets["schema"] != RELEASE_TARGET_SCHEMA {
        return Err("release-status target schema is invalid".to_owned());
    }
    let target_entries = sources.targets["targets"]
        .as_array()
        .ok_or_else(|| "release-status targets must be an array".to_owned())?;
    let target_triples = target_entries
        .iter()
        .map(|entry| required_string(entry, "triple", "release-status target").map(str::to_owned))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if target_triples.len() != target_entries.len() {
        return Err("release-status targets contain duplicate triples".to_owned());
    }
    let support_target_count = sources.support["platforms"]["target_count"]
        .as_u64()
        .ok_or_else(|| "release-status support policy has no target count".to_owned())?;
    if support_target_count != target_triples.len() as u64 {
        return Err(format!(
            "release-status platform disagreement: support declares {support_target_count} targets, target authority contains {}",
            target_triples.len()
        ));
    }

    if sources.alpha_package["schema"] != alpha_package_contract_schema(version)? {
        return Err("release-status Alpha package contract schema is invalid".to_owned());
    }
    let package_version = Version::parse(required_string(
        &sources.alpha_package,
        "version",
        "release-status Alpha package contract",
    )?)
    .map_err(|error| format!("Alpha package version is invalid SemVer: {error}"))?;
    let package_tag = required_string(
        &sources.alpha_package,
        "tag",
        "release-status Alpha package contract",
    )?;
    if package_version != *version || package_tag != source_tag {
        return Err(format!(
            "release-status source/package disagreement: Cargo is `{version}`, package contract is `{package_version}` / `{package_tag}`"
        ));
    }
    let package_assets = sources.alpha_package["standalone_cli"]["assets"]
        .as_array()
        .ok_or_else(|| "release-status Alpha package contract has no CLI assets".to_owned())?;
    let package_targets = package_assets
        .iter()
        .map(|asset| {
            required_string(asset, "target", "release-status Alpha package asset")
                .map(str::to_owned)
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if package_targets.len() != package_assets.len() || package_targets != target_triples {
        return Err(
            "release-status platform disagreement: package and target-authority sets differ"
                .to_owned(),
        );
    }
    let desktop_public_target = sources.alpha_package["desktop_macos"]["target"]
        .as_str()
        .filter(|target| !target.is_empty())
        .ok_or_else(|| "release-status Alpha package has no public desktop target".to_owned())?;
    if !target_triples.contains(desktop_public_target) {
        return Err("release-status public desktop target is not a release target".to_owned());
    }
    let desktop_nonpublishing_target = sources.alpha_package["desktop_macos_intel"]["target"]
        .as_str()
        .filter(|target| !target.is_empty())
        .ok_or_else(|| "release-status Alpha package has no Intel desktop target".to_owned())?;
    if !target_triples.contains(desktop_nonpublishing_target)
        || sources.alpha_package["desktop_macos_intel"]["status"] != "not-published"
    {
        return Err(
            "release-status Intel desktop target must remain a nonpublishing release target"
                .to_owned(),
        );
    }

    let public_tag = &sources.git.public_tag;
    if public_tag != &format!("v{}", sources.git.public_version) {
        return Err("release-status public tag does not match its SemVer".to_owned());
    }
    if (
        sources.git.public_version.major,
        sources.git.public_version.minor,
    ) != (version.major, version.minor)
    {
        return Err(format!(
            "release-status source/public disagreement: source line is `{release_line}`, public checkpoint is `{public_tag}`"
        ));
    }
    if sources.git.public_version > *version {
        return Err(format!(
            "release-status source/public disagreement: public checkpoint `{public_tag}` is newer than source `{source_tag}`"
        ));
    }
    validate_lower_hex("release-status source commit", &sources.git.head_commit, 40)?;
    validate_lower_hex(
        "release-status public commit",
        &sources.git.public_commit,
        40,
    )?;
    if (sources.git.public_commit == sources.git.head_commit)
        != (sources.git.source_commits_ahead == 0)
    {
        return Err(
            "release-status Git checkpoint identity disagrees with its commit distance".to_owned(),
        );
    }

    if sources.beta_readiness["schema"] != BETA_READINESS_SCHEMA {
        return Err("release-status Beta readiness schema is invalid".to_owned());
    }
    if sources.beta_freeze["schema"] != BETA_CONTRACT_FREEZE_SCHEMA {
        return Err("release-status Beta freeze schema is invalid".to_owned());
    }
    if sources.feedback["schema"] != FEEDBACK_SNAPSHOT_SCHEMA {
        return Err("release-status feedback schema is invalid".to_owned());
    }
    if required_string(&sources.feedback, "release_line", "release-status feedback")?
        != release_line
    {
        return Err("release-status feedback release line differs from Cargo".to_owned());
    }
    let readiness_tag = required_string(
        &sources.beta_readiness["alpha_release"],
        "tag",
        "release-status Beta readiness",
    )?;
    let freeze_tag = required_string(
        &sources.beta_freeze["baseline"],
        "release",
        "release-status Beta freeze",
    )?;
    let feedback_tag = release_status_feedback_tag(&sources.feedback)?;
    for (context, tag) in [
        ("Beta readiness", readiness_tag),
        ("Beta freeze", freeze_tag),
        ("feedback", feedback_tag),
    ] {
        let checkpoint = Version::parse(tag.strip_prefix('v').unwrap_or(tag))
            .map_err(|error| format!("release-status {context} tag is invalid SemVer: {error}"))?;
        if (checkpoint.major, checkpoint.minor) != (version.major, version.minor) {
            return Err(format!(
                "release-status {context} checkpoint `{tag}` is outside source line `{release_line}`"
            ));
        }
    }

    if sources.cli_gui_parity["format"] != "canisend.cli-gui-parity/v1" {
        return Err("release-status CLI/GUI parity format is invalid".to_owned());
    }
    if sources.svelte_parity["format"] != SVELTE_PARITY_SCHEMA {
        return Err("release-status Svelte parity format is invalid".to_owned());
    }
    let cli_operations = release_status_operation_set(&sources.cli_gui_parity, "CLI/GUI")?;
    let svelte_operations = release_status_operation_set(&sources.svelte_parity, "Svelte")?;
    if cli_operations != svelte_operations {
        return Err("release-status operation-family authorities disagree".to_owned());
    }
    let svelte_cutover_ready = sources.svelte_parity["cutover_ready"]
        .as_bool()
        .ok_or_else(|| "release-status Svelte parity has no cutover state".to_owned())?;
    if sources.signing["schema"] != SIGNING_POLICY_SCHEMA {
        return Err("release-status signing policy schema is invalid".to_owned());
    }
    let signing_tier = required_string(
        &sources.signing,
        "trust_tier",
        "release-status signing policy",
    )?;

    let mut drift = Vec::new();
    if sources.git.public_version < *version {
        push_release_status_drift(
            &mut drift,
            "source-version-ahead-of-public-checkpoint",
            "pending",
            &["Cargo.toml", "Git tags"],
            &format!("source `{source_tag}` is newer than public `{public_tag}`"),
        );
    } else if sources.git.source_commits_ahead > 0 {
        push_release_status_drift(
            &mut drift,
            "source-commit-ahead-of-public-checkpoint",
            "pending",
            &["Git HEAD", "Git tags"],
            &format!(
                "source is {} commits ahead of `{public_tag}`",
                sources.git.source_commits_ahead
            ),
        );
    }
    if package_tag != public_tag {
        push_release_status_drift(
            &mut drift,
            "package-contract-not-public",
            "pending",
            &["release/alpha-package-contract.json", "Git tags"],
            &format!("package contract `{package_tag}` is not public checkpoint `{public_tag}`"),
        );
    }
    for (code, authority, checkpoint) in [
        (
            "beta-readiness-not-current-public",
            "release/beta-readiness.json",
            readiness_tag,
        ),
        (
            "beta-freeze-not-current-public",
            "release/beta-contract-freeze.json",
            freeze_tag,
        ),
        (
            "feedback-not-current-public",
            "release/feedback-snapshot.json",
            feedback_tag,
        ),
    ] {
        if checkpoint != public_tag {
            push_release_status_drift(
                &mut drift,
                code,
                "blocking",
                &[authority, "Git tags"],
                &format!("checkpoint `{checkpoint}` differs from public `{public_tag}`"),
            );
        }
    }
    let blocking_count = drift
        .iter()
        .filter(|item| item["severity"] == "blocking")
        .count() as u64;

    let release_candidates = sources.qualification["release_candidates"]
        .as_array()
        .ok_or_else(|| "release-status qualification ledger has no RC array".to_owned())?;
    let target_list = target_triples.into_iter().collect::<Vec<_>>();
    Ok(json!({
        "schema": RELEASE_STATUS_SCHEMA,
        "derived": true,
        "authoritative": false,
        "hard_consistent": true,
        "authorities": {
            "source": ["Cargo.toml", "Git HEAD"],
            "public_checkpoint": "latest reachable SemVer Git tag on the active release line",
            "qualification": "release/qualification-ledger.json",
            "support": ["release/support-policy.json", "release/targets.json", "release/alpha-package-contract.json"],
            "contracts": ["docs/contracts/cli-gui-parity-v1.json", "docs/contracts/svelte-parity-v1.json"],
            "stage_evidence": ["release/beta-readiness.json", "release/beta-contract-freeze.json", "release/feedback-snapshot.json"]
        },
        "source": {
            "version": version.to_string(),
            "candidate_tag": source_tag,
            "release_line": release_line,
            "stage": stage.as_str(),
            "license": sources.workspace_license,
            "head_commit": sources.git.head_commit,
            "worktree_dirty": sources.git.worktree_dirty
        },
        "public_checkpoint": {
            "tag": public_tag,
            "version": sources.git.public_version.to_string(),
            "stage": ReleaseStage::from_version(&sources.git.public_version)?.as_str(),
            "source_commit": sources.git.public_commit,
            "source_commits_ahead": sources.git.source_commits_ahead,
            "head_matches_checkpoint": sources.git.public_commit == sources.git.head_commit
        },
        "qualification": {
            "workspace_stage": ledger_stage,
            "status": qualification_status,
            "stable_authorized": stable_authorized,
            "feature_freeze": required_string(&sources.qualification["feature_freeze"], "status", "release-status feature freeze")?,
            "beta": required_string(&sources.qualification["beta"], "status", "release-status Beta qualification")?,
            "release_candidate_count": release_candidates.len()
        },
        "support": {
            "publication_status": support_publication,
            "release_line": support_line,
            "cli_target_count": target_list.len(),
            "cli_targets": target_list,
            "desktop_public_targets": [desktop_public_target],
            "desktop_nonpublishing_targets": [desktop_nonpublishing_target],
            "desktop_nonpublishing_workflow": ".github/workflows/desktop-platform-qualification.yml",
            "signing_trust_tier": signing_tier
        },
        "contracts": {
            "agent_protocol": required_string(&sources.support["contracts"], "agent_protocol", "release-status support contracts")?,
            "public_schema_version": required_string(&sources.support["contracts"], "public_schema_version", "release-status support contracts")?,
            "workspace_format": required_string(&sources.support["workspace"], "format", "release-status support workspace")?,
            "database_schema_version": sources.support["workspace"]["current_database_schema_version"],
            "alpha_package_version": package_version.to_string(),
            "alpha_package_tag": package_tag,
            "operation_family_count": cli_operations.len(),
            "svelte_cutover_ready": svelte_cutover_ready
        },
        "stage_evidence": {
            "beta_readiness": {
                "status": sources.beta_readiness["status"].as_str().unwrap_or("recorded"),
                "checkpoint": readiness_tag,
                "audited_at": sources.beta_readiness["audited_at"]
            },
            "beta_freeze": {
                "status": sources.beta_freeze["status"].as_str().unwrap_or("recorded"),
                "checkpoint": freeze_tag
            },
            "feedback": {
                "status": required_string(&sources.feedback, "status", "release-status feedback")?,
                "checkpoint": feedback_tag,
                "roadmap": required_string(&sources.feedback["next_roadmap"], "path", "release-status feedback roadmap")?
            }
        },
        "drift": {
            "count": drift.len(),
            "blocking_count": blocking_count,
            "blocks_stage_transition": blocking_count > 0,
            "items": drift
        }
    }))
}

fn release_status_feedback_tag(feedback: &Value) -> Result<&str, String> {
    match (
        feedback.pointer("/release/tag").and_then(Value::as_str),
        feedback
            .pointer("/expected_release/tag")
            .and_then(Value::as_str),
    ) {
        (Some(tag), None) | (None, Some(tag)) if !tag.is_empty() => Ok(tag),
        _ => Err(
            "release-status feedback must contain exactly one release or expected-release tag"
                .to_owned(),
        ),
    }
}

fn release_status_operation_set(
    document: &Value,
    context: &str,
) -> Result<BTreeSet<String>, String> {
    let entries = document["entries"]
        .as_array()
        .ok_or_else(|| format!("release-status {context} entries must be an array"))?;
    let operations = entries
        .iter()
        .map(|entry| {
            required_string(
                entry,
                "operation",
                &format!("release-status {context} entry"),
            )
            .map(str::to_owned)
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if operations.len() != entries.len() {
        return Err(format!(
            "release-status {context} entries contain duplicate operations"
        ));
    }
    Ok(operations)
}

fn push_release_status_drift(
    drift: &mut Vec<Value>,
    code: &str,
    severity: &str,
    authorities: &[&str],
    detail: &str,
) {
    drift.push(json!({
        "code": code,
        "severity": severity,
        "authorities": authorities,
        "detail": detail
    }));
}

fn check_release_qualification() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/qualification-ledger.json");
    let ledger: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "release qualification ledger is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("release qualification ledger is invalid JSON: {error}"))?;
    if ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA {
        return Err("release qualification ledger schema is invalid".to_owned());
    }
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let stage = ReleaseStage::from_version(&version)?;
    let required_status = qualification_status_for_stage(stage);
    if ledger["workspace_stage"] != stage.as_str() || ledger["status"] != required_status {
        return Err(format!(
            "release qualification ledger must be `{required_status}` for {} stage",
            stage.as_str()
        ));
    }

    let feature_freeze = &ledger["feature_freeze"];
    let freeze_status = required_string(feature_freeze, "status", "feature freeze")?;
    if !matches!(freeze_status, "planned" | "frozen") {
        return Err("release feature-freeze status is invalid".to_owned());
    }
    if freeze_status == "planned" && !feature_freeze["baseline_commit"].is_null() {
        return Err("a planned feature freeze cannot claim a baseline commit".to_owned());
    }
    if freeze_status == "frozen" {
        let baseline = required_string(feature_freeze, "baseline_commit", "feature freeze")?;
        validate_lower_hex("feature-freeze baseline commit", baseline, 40)?;
    }
    if matches!(stage, ReleaseStage::ReleaseCandidate | ReleaseStage::Stable)
        && freeze_status != "frozen"
    {
        return Err("RC and Stable stages require a frozen feature baseline".to_owned());
    }
    let allowed_change_classes = feature_freeze["allowed_change_classes"]
        .as_array()
        .ok_or_else(|| "feature freeze has no allowed change classes".to_owned())?;
    let expected_change_classes =
        BTreeSet::from(["release-blocker", "release-evidence", "documentation"]);
    let actual_change_classes = allowed_change_classes
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if actual_change_classes != expected_change_classes
        || actual_change_classes.len() != allowed_change_classes.len()
    {
        return Err("feature-freeze allowed change classes differ".to_owned());
    }
    check_feature_freeze_exceptions(feature_freeze)?;

    let package_managers = &ledger["package_managers"];
    let channels = package_managers["channels"]
        .as_array()
        .ok_or_else(|| "release qualification package-manager channels are missing".to_owned())?;
    let expected_channels = BTreeSet::from(["homebrew-cask", "scoop", "winget"]);
    let actual_channels = channels
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if actual_channels != expected_channels || actual_channels.len() != channels.len() {
        return Err("release qualification package-manager channels differ".to_owned());
    }
    let beta_status = required_string(&ledger["beta"], "status", "Beta qualification")?;
    if !matches!(beta_status, "pending" | "qualified") {
        return Err("Beta qualification status is invalid".to_owned());
    }
    if matches!(stage, ReleaseStage::ReleaseCandidate | ReleaseStage::Stable)
        && beta_status != "qualified"
    {
        return Err("RC and Stable stages require a qualified Beta".to_owned());
    }
    let release_candidates = ledger["release_candidates"]
        .as_array()
        .ok_or_else(|| "release candidate qualification must be an array".to_owned())?;
    for (section, allowed) in [
        ("upgrade_matrix", &["pending", "passed"][..]),
        (
            "documentation_uninstall",
            &["prepared-local", "prepared-native", "passed"][..],
        ),
        ("package_managers", &["candidates-only", "passed"][..]),
    ] {
        let status = required_string(&ledger[section], "status", section)?;
        if !allowed.contains(&status) {
            return Err(format!(
                "release qualification `{section}` status is invalid"
            ));
        }
    }
    if package_managers["status"] == "passed" {
        validate_package_manager_qualification_record(
            package_managers,
            &ledger["beta"],
            release_candidates,
        )?;
    } else if *package_managers
        != json!({
            "channels": ["homebrew-cask", "scoop", "winget"],
            "evidence": [],
            "status": "candidates-only"
        })
    {
        return Err(
            "pending package-manager qualification contains preauthorized evidence".to_owned(),
        );
    }
    validate_documentation_uninstall_progress(&ledger["documentation_uninstall"])?;

    let release_notes = &ledger["release_notes"];
    let release_notes_status = required_string(release_notes, "status", "release notes")?;
    if !matches!(
        release_notes_status,
        "alpha-current" | "beta-current" | "rc-final" | "stable-final"
    ) {
        return Err("release notes qualification status is invalid".to_owned());
    }
    let expected_release_notes_status = release_notes_status_for_stage(stage);
    if release_notes_status != expected_release_notes_status {
        return Err(format!(
            "release notes status must be `{expected_release_notes_status}` for {} stage",
            stage.as_str()
        ));
    }
    for (field, expected_path) in [
        ("notes", "release/RELEASE_NOTES.md"),
        ("rollback", "docs/guides/upgrade-and-rollback.md"),
    ] {
        if release_notes[field] != expected_path || !root.join(expected_path).is_file() {
            return Err(format!(
                "release qualification {field} path must be `{expected_path}`"
            ));
        }
    }
    check_release_notes_review(&root, &version, stage, release_notes, release_candidates)?;

    if matches!(stage, ReleaseStage::Stable) {
        validate_stable_qualification(&ledger)?;
    } else if ledger["stable_authorized"] != false {
        return Err("a prerelease qualification ledger cannot authorize Stable".to_owned());
    }
    if matches!(stage, ReleaseStage::ReleaseCandidate) && release_candidates.is_empty() {
        println!(
            "release qualification: RC evidence collection has not recorded a clean-tag matrix yet"
        );
    }
    let documentation_path = root.join("docs/release/qualification-ledger.md");
    let documentation = fs::read_to_string(&documentation_path)
        .map_err(|error| format!("qualification-ledger documentation is missing: {error}"))?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    for required in [
        "record-beta-qualification",
        "record-rc-qualification",
        "DOWNLOADED_ASSET_DIRECTORY",
        "gh attestation verify",
        "--write",
    ] {
        if !documentation.contains(required) {
            return Err(format!(
                "qualification-ledger documentation is missing `{required}`"
            ));
        }
    }
    println!(
        "release qualification: ok ({required_status}, stage {})",
        stage.as_str()
    );
    Ok(())
}

fn validate_documentation_uninstall_progress(value: &Value) -> Result<(), String> {
    let status = required_string(value, "status", "documentation/uninstall qualification")?;
    let run = value["native_matrix_run"].as_u64().filter(|run| *run > 0);
    let evidence_is_complete = value["evidence"].as_array().is_some_and(|items| {
        !items.is_empty()
            && items
                .iter()
                .all(|item| item.as_str().is_some_and(|item| !item.is_empty()))
    });
    match status {
        "prepared-local" if run.is_some() => Err(
            "local documentation/uninstall preparation cannot claim a native matrix run".to_owned(),
        ),
        "prepared-local" => Ok(()),
        "prepared-native" | "passed" if run.is_none() => Err(format!(
            "`{status}` documentation/uninstall evidence requires a native matrix run"
        )),
        "prepared-native" | "passed" if !evidence_is_complete => Err(format!(
            "`{status}` documentation/uninstall evidence requires non-empty evidence"
        )),
        "prepared-native" | "passed" => Ok(()),
        _ => Err("documentation/uninstall qualification status is invalid".to_owned()),
    }
}

fn check_feature_freeze_exceptions(feature_freeze: &Value) -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/feature-freeze-exceptions.json");
    let record: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "feature-freeze exception record is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("feature-freeze exception record is invalid JSON: {error}"))?;
    validate_feature_freeze_exception_record(feature_freeze, &record)?;
    let status = required_string(feature_freeze, "status", "feature freeze")?;
    let exceptions = record["exceptions"]
        .as_array()
        .ok_or_else(|| "feature-freeze exceptions must be an array".to_owned())?;
    let documentation_path = root.join("docs/release/feature-freeze.md");
    let documentation = fs::read_to_string(&documentation_path)
        .map_err(|error| format!("feature-freeze documentation is missing: {error}"))?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    for required in [
        "activate-feature-freeze FULL_HEAD_COMMIT",
        "--write",
        "equal to current `HEAD`",
    ] {
        if !documentation.contains(required) {
            return Err(format!(
                "feature-freeze documentation is missing `{required}`"
            ));
        }
    }
    if status == "planned" {
        println!("feature freeze: ok (planned, no preauthorized exceptions)");
        return Ok(());
    }

    let baseline = required_string(feature_freeze, "baseline_commit", "feature freeze")?;
    validate_feature_freeze_history(&root, baseline, exceptions)?;
    println!(
        "feature freeze: ok (frozen at {baseline}, {} exceptions)",
        exceptions.len()
    );
    Ok(())
}

fn validate_feature_freeze_exception_record(
    feature_freeze: &Value,
    record: &Value,
) -> Result<(), String> {
    if record["schema"] != FEATURE_FREEZE_EXCEPTIONS_SCHEMA {
        return Err("feature-freeze exception schema is invalid".to_owned());
    }
    let status = required_string(feature_freeze, "status", "feature freeze")?;
    if record["status"] != status || record["baseline_commit"] != feature_freeze["baseline_commit"]
    {
        return Err(
            "feature-freeze exception record differs from the qualification ledger".to_owned(),
        );
    }
    record["exceptions"]
        .as_array()
        .ok_or_else(|| "feature-freeze exceptions must be an array".to_owned())?;
    if status == "planned" {
        let expected = json!({
            "schema": FEATURE_FREEZE_EXCEPTIONS_SCHEMA,
            "status": "planned",
            "baseline_commit": null,
            "exceptions": []
        });
        if record != &expected {
            return Err("planned feature freeze cannot pre-authorize exceptions".to_owned());
        }
        return Ok(());
    }

    let baseline = required_string(feature_freeze, "baseline_commit", "feature freeze")?;
    validate_lower_hex("feature-freeze baseline commit", baseline, 40)?;
    if status != "frozen" {
        return Err("feature-freeze status must be `planned` or `frozen`".to_owned());
    }
    Ok(())
}

fn validate_feature_freeze_history(
    root: &Path,
    baseline: &str,
    exceptions: &[Value],
) -> Result<(), String> {
    run_git(root, &["cat-file", "-e", &format!("{baseline}^{{commit}}")])?;
    run_git(root, &["merge-base", "--is-ancestor", baseline, "HEAD"])?;
    let range = format!("{baseline}..HEAD");
    let commits = run_git_lines(root, &["rev-list", "--reverse", &range])?;
    let mut changed_by_commit = BTreeMap::new();
    for commit in &commits {
        let paths = run_git_lines(
            root,
            &[
                "diff-tree",
                "--first-parent",
                "-m",
                "--no-commit-id",
                "--name-only",
                "-r",
                commit,
            ],
        )?;
        let nonautomatic = paths
            .into_iter()
            .filter(|path| !is_automatic_feature_freeze_path(path))
            .collect::<BTreeSet<_>>();
        if !nonautomatic.is_empty() {
            changed_by_commit.insert(commit.clone(), nonautomatic);
        }
    }

    let mut recorded_commits = Vec::new();
    for entry in exceptions {
        let commit = required_string(entry, "commit", "feature-freeze exception")?;
        validate_lower_hex("feature-freeze exception commit", commit, 40)?;
        let class = required_string(entry, "class", "feature-freeze exception")?;
        if !matches!(class, "release-blocker" | "release-evidence") {
            return Err("feature-freeze exception class is invalid".to_owned());
        }
        let reason = required_string(entry, "reason", "feature-freeze exception")?;
        if reason.len() > 500 || reason.chars().any(char::is_control) {
            return Err("feature-freeze exception reason is invalid".to_owned());
        }
        let paths = entry["paths"]
            .as_array()
            .ok_or_else(|| "feature-freeze exception paths are missing".to_owned())?
            .iter()
            .map(|path| {
                path.as_str()
                    .filter(|path| !path.is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| "feature-freeze exception path is invalid".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let path_set = paths.iter().cloned().collect::<BTreeSet<_>>();
        if path_set.len() != paths.len()
            || paths.iter().ne(path_set.iter())
            || paths.iter().any(|path| {
                path.starts_with('/')
                    || path.contains('\\')
                    || path.split('/').any(|part| matches!(part, "" | "." | ".."))
                    || path.chars().any(char::is_control)
            })
        {
            return Err(
                "feature-freeze exception paths must be unique sorted repository paths".to_owned(),
            );
        }
        let actual = changed_by_commit.get(commit).ok_or_else(|| {
            format!("feature-freeze exception commit `{commit}` has no exceptional changed paths")
        })?;
        if &path_set != actual {
            return Err(format!(
                "feature-freeze exception paths differ for commit `{commit}`"
            ));
        }
        let canonical = json!({
            "commit": commit,
            "class": class,
            "reason": reason,
            "paths": paths
        });
        if *entry != canonical {
            return Err(
                "feature-freeze exception contains unknown or noncanonical fields".to_owned(),
            );
        }
        recorded_commits.push(commit.to_owned());
    }
    let expected_commits = commits
        .into_iter()
        .filter(|commit| changed_by_commit.contains_key(commit))
        .collect::<Vec<_>>();
    if recorded_commits != expected_commits {
        return Err(
            "feature-freeze exceptions do not cover the exact post-baseline commit order"
                .to_owned(),
        );
    }
    Ok(())
}

fn is_automatic_feature_freeze_path(path: &str) -> bool {
    path.starts_with("docs/")
        || path.starts_with("packaging/candidates/")
        || path.starts_with("release/evidence/")
        || matches!(
            path,
            "README.md"
                | "CONTRIBUTING.md"
                | "SECURITY.md"
                | "CHANGELOG.md"
                | "release/RELEASE_NOTES.md"
                | "release/qualification-ledger.json"
                | "release/feedback-snapshot.json"
                | "release/support-policy.json"
                | "release/feature-freeze-exceptions.json"
        )
}

fn run_git(root: &Path, arguments: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not execute Git repository check: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Git repository command `git {}` failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn run_git_lines(root: &Path, arguments: &[&str]) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not execute Git repository check: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Git repository command `git {}` failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| "Git repository output is not UTF-8".to_owned())?;
    Ok(stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

fn qualification_status_for_stage(stage: ReleaseStage) -> &'static str {
    match stage {
        ReleaseStage::Alpha => "pre-beta",
        ReleaseStage::Beta => "beta-qualifying",
        ReleaseStage::ReleaseCandidate => "rc-qualifying",
        ReleaseStage::Stable => "qualified",
    }
}

fn check_release_notes_review(
    root: &Path,
    version: &Version,
    stage: ReleaseStage,
    release_notes: &Value,
    candidates: &[Value],
) -> Result<(), String> {
    let review = &release_notes["review"];
    if review.is_null() {
        if matches!(stage, ReleaseStage::Stable) {
            return Err("Stable requires a reviewed final RC release-notes record".to_owned());
        }
        return Ok(());
    }
    if matches!(stage, ReleaseStage::Alpha | ReleaseStage::Beta) {
        return Err("Alpha and Beta cannot preapprove a final RC notes review".to_owned());
    }
    let (tag, _, _) = validate_release_notes_review_record(review, candidates)?;
    if matches!(stage, ReleaseStage::ReleaseCandidate) && tag != format!("v{version}") {
        return Err("RC release-notes review must match the current workspace tag".to_owned());
    }
    let notes = fs::read(root.join("release/RELEASE_NOTES.md"))
        .map_err(|error| format!("could not read reviewed release notes: {error}"))?;
    if review["release_notes_body_sha256"] != release_notes_body_sha256(&notes)? {
        return Err("reviewed release-notes body changed after qualification".to_owned());
    }
    if review["rollback_sha256"] != sha256_file(&root.join("docs/guides/upgrade-and-rollback.md"))?
    {
        return Err("reviewed rollback guidance changed after qualification".to_owned());
    }
    Ok(())
}

fn validate_release_notes_review_record(
    review: &Value,
    candidates: &[Value],
) -> Result<(String, String, u64), String> {
    if review["status"] != "reviewed" {
        return Err("release-notes review status must be `reviewed`".to_owned());
    }
    let tag = required_string(review, "tag", "release-notes review")?;
    let source_commit = required_string(review, "source_commit", "release-notes review")?;
    let reviewer = required_string(review, "reviewer", "release-notes review")?;
    validate_github_login(reviewer)?;
    validate_lower_hex("release-notes RC source commit", source_commit, 40)?;
    for (field, context) in [
        ("release_manifest_sha256", "release-notes manifest SHA-256"),
        ("release_notes_body_sha256", "release-notes body SHA-256"),
        ("rollback_sha256", "release-notes rollback SHA-256"),
    ] {
        validate_lower_hex(
            context,
            required_string(review, field, "release-notes review")?,
            64,
        )?;
    }
    let run = review["signed_matrix_run"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| "release-notes review has no signed matrix run".to_owned())?;
    let candidate = candidates
        .last()
        .ok_or_else(|| "release-notes review has no final RC matrix".to_owned())?;
    let (candidate_tag, candidate_source, candidate_run) =
        validate_qualification_release(candidate, ReleaseStage::ReleaseCandidate, "final RC")?;
    if tag != candidate_tag || source_commit != candidate_source || run != candidate_run {
        return Err("release-notes review differs from the final recorded RC matrix".to_owned());
    }
    let evidence = json!([
        format!("{tag} release notes and rollback guidance reviewed by {reviewer}"),
        format!(
            "signed RC matrix run {run} manifest, public issues, assets, limitations, and package-channel state reviewed"
        )
    ]);
    let canonical = json!({
        "evidence": evidence,
        "release_manifest_sha256": review["release_manifest_sha256"],
        "release_notes_body_sha256": review["release_notes_body_sha256"],
        "reviewer": reviewer,
        "rollback_sha256": review["rollback_sha256"],
        "signed_matrix_run": run,
        "source_commit": source_commit,
        "status": "reviewed",
        "tag": tag
    });
    if *review != canonical {
        return Err("release-notes review contains unknown or non-canonical fields".to_owned());
    }
    Ok((tag.to_owned(), source_commit.to_owned(), run))
}

fn validate_stable_qualification(ledger: &Value) -> Result<(), String> {
    let feature_freeze = &ledger["feature_freeze"];
    if feature_freeze["status"] != "frozen" {
        return Err("Stable requires a frozen feature baseline".to_owned());
    }
    let freeze_commit = required_string(feature_freeze, "baseline_commit", "feature freeze")?;
    validate_lower_hex("feature-freeze baseline commit", freeze_commit, 40)?;

    let beta = &ledger["beta"];
    if beta["status"] != "qualified" {
        return Err("Stable requires a qualified signed Beta".to_owned());
    }
    validate_qualification_release(beta, ReleaseStage::Beta, "Beta")?;
    let signing_targets = beta["signing_evidence_targets"]
        .as_array()
        .ok_or_else(|| "qualified Beta signing targets are missing".to_owned())?;
    let expected_signing_targets = BTreeSet::from([
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
    ]);
    let actual_signing_targets = signing_targets
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if actual_signing_targets != expected_signing_targets
        || actual_signing_targets.len() != signing_targets.len()
    {
        return Err("qualified Beta signing evidence targets differ".to_owned());
    }

    let candidates = ledger["release_candidates"]
        .as_array()
        .filter(|entries| entries.len() >= 2)
        .ok_or_else(|| "Stable requires two successful clean-tag RC matrices".to_owned())?;
    let mut tags = BTreeSet::new();
    let mut commits = BTreeSet::new();
    let mut runs = BTreeSet::new();
    for candidate in candidates {
        if candidate["status"] != "success" {
            return Err("every Stable RC matrix must have success status".to_owned());
        }
        let (tag, commit, run) =
            validate_qualification_release(candidate, ReleaseStage::ReleaseCandidate, "RC")?;
        if !tags.insert(tag) || !commits.insert(commit) || !runs.insert(run) {
            return Err(
                "Stable RC matrices must use distinct tags, commits, and run IDs".to_owned(),
            );
        }
    }
    validate_package_manager_qualification_record(&ledger["package_managers"], beta, candidates)?;

    for (section, expected_status) in [
        ("upgrade_matrix", "passed"),
        ("documentation_uninstall", "passed"),
        ("package_managers", "passed"),
    ] {
        let evidence = &ledger[section];
        if evidence["status"] != expected_status
            || evidence["evidence"].as_array().is_none_or(|items| {
                items.is_empty()
                    || items
                        .iter()
                        .any(|item| item.as_str().is_none_or(str::is_empty))
            })
        {
            return Err(format!("Stable requires passed `{section}` evidence"));
        }
    }
    let documentation_run = ledger["documentation_uninstall"]["native_matrix_run"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| "Stable documentation/uninstall evidence has no native run ID".to_owned())?;
    if !runs.contains(&documentation_run) {
        return Err(
            "Stable documentation/uninstall evidence must come from one qualified RC matrix"
                .to_owned(),
        );
    }
    validate_release_notes_review_record(&ledger["release_notes"]["review"], candidates)?;
    if ledger["release_notes"]["status"] != "stable-final" || ledger["stable_authorized"] != true {
        return Err("Stable release notes or authorization are incomplete".to_owned());
    }
    Ok(())
}

fn validate_qualification_release(
    value: &Value,
    expected_stage: ReleaseStage,
    context: &str,
) -> Result<(String, String, u64), String> {
    let tag = required_string(value, "tag", context)?;
    let version = Version::parse(
        tag.strip_prefix('v')
            .ok_or_else(|| format!("{context} tag must start with `v`"))?,
    )
    .map_err(|error| format!("{context} tag is invalid SemVer: {error}"))?;
    if ReleaseStage::from_version(&version)? != expected_stage {
        return Err(format!("{context} tag has the wrong release stage"));
    }
    let commit = required_string(value, "source_commit", context)?;
    validate_lower_hex(&format!("{context} source commit"), commit, 40)?;
    let run = value["signed_matrix_run"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| format!("{context} has no signed matrix run ID"))?;
    Ok((tag.to_owned(), commit.to_owned(), run))
}

fn build_beta_contract_freeze() -> Result<Value, String> {
    let root = repository_root();
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let readiness_path = root.join("release/beta-readiness.json");
    let readiness: Value = serde_json::from_slice(
        &fs::read(&readiness_path)
            .map_err(|error| format!("could not read qualified Alpha identity: {error}"))?,
    )
    .map_err(|error| format!("qualified Alpha identity is invalid JSON: {error}"))?;
    if readiness["status"] == "pending-alpha-publication" {
        return Err(
            "public Alpha qualification is required before freezing Beta contracts".to_owned(),
        );
    }
    let alpha_tag = required_string(&readiness["alpha_release"], "tag", "Alpha release")?;
    validate_alpha_baseline_tag(&version, alpha_tag)?;
    let alpha_source = required_string(
        &readiness["alpha_release"],
        "source_commit",
        "Alpha release",
    )?;
    validate_lower_hex("Beta contract Alpha source commit", alpha_source, 40)?;
    let schema_root = schema_directory();
    let schema_names = json_files(&schema_root)?.into_iter().collect::<Vec<_>>();
    if schema_names.len() != generate_public_schemas().len() {
        return Err("public schema inventory is incomplete before Beta freeze".to_owned());
    }
    let schema_entries = schema_names
        .iter()
        .map(|name| {
            read_frozen_contract_text(&schema_root.join(name), "schema")
                .map(|bytes| (name.clone(), bytes))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let snapshot_names = ["agent-capabilities.json", "agent-context.json"];
    let snapshot_root = root.join("crates/canisend-cli/tests/snapshots");
    let snapshot_entries = snapshot_names
        .iter()
        .map(|name| {
            let path = snapshot_root.join(name);
            let mut value: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
                format!(
                    "could not read frozen agent snapshot {}: {error}",
                    path.display()
                )
            })?)
            .map_err(|error| format!("agent snapshot `{name}` is invalid JSON: {error}"))?;
            normalize_product_version(&mut value);
            serde_json::to_vec(&value)
                .map(|bytes| ((*name).to_owned(), bytes))
                .map_err(|error| format!("could not normalize agent snapshot `{name}`: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let migrations = migration_inventory()?;
    let current_schema_version = migrations
        .last()
        .map(|(version, _, _)| *version)
        .ok_or_else(|| "workspace migration inventory is empty".to_owned())?;
    let declared_schema_version = declared_database_schema_version()?;
    if current_schema_version != declared_schema_version {
        return Err(format!(
            "database schema constant {declared_schema_version} does not match migration inventory {current_schema_version}"
        ));
    }
    let frozen_migrations = migrations
        .iter()
        .filter(|(version, _, _)| *version <= FROZEN_MIGRATIONS_THROUGH)
        .map(|(_, name, path)| {
            read_frozen_contract_text(path, "migration").map(|bytes| (name.clone(), bytes))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if frozen_migrations.len() != FROZEN_MIGRATIONS_THROUGH as usize {
        return Err(format!(
            "expected migrations 1 through {FROZEN_MIGRATIONS_THROUGH} to exist"
        ));
    }

    Ok(json!({
        "schema": BETA_CONTRACT_FREEZE_SCHEMA,
        "baseline": {
            "release": alpha_tag,
            "source_commit": alpha_source
        },
        "agent": {
            "protocol": AGENT_PROTOCOL,
            "public_schema_version": PUBLIC_SCHEMA_VERSION,
            "public_schema_files": schema_entries.len(),
            "public_schema_tree_sha256": digest_named_bytes(&schema_entries),
            "normalized_snapshot_files": snapshot_names,
            "normalized_snapshot_tree_sha256": digest_named_bytes(&snapshot_entries),
            "product_version_is_excluded_from_snapshot_digest": true
        },
        "workspace": {
            "format": WORKSPACE_FORMAT,
            "current_database_schema_version": current_schema_version,
            "frozen_migrations_through": FROZEN_MIGRATIONS_THROUGH,
            "frozen_migration_tree_sha256": digest_named_bytes(&frozen_migrations),
            "migration_policy": "append-only",
            "reject_future_schema_versions": true
        }
    }))
}

fn read_frozen_contract_text(path: &Path, kind: &str) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("could not read frozen {kind} `{}`: {error}", path.display()))?;
    canonicalize_frozen_contract_text(&bytes).map_err(|error| {
        format!(
            "frozen {kind} `{}` is not canonical text: {error}",
            path.display()
        )
    })
}

fn canonicalize_frozen_contract_text(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| format!("invalid UTF-8: {error}"))?;
    let normalized = text.replace("\r\n", "\n");
    if normalized.contains('\r') {
        return Err("bare carriage returns are not supported".to_owned());
    }
    Ok(normalized.into_bytes())
}

fn migration_inventory() -> Result<Vec<(u32, String, PathBuf)>, String> {
    migration_inventory_at(&repository_root())
}

fn migration_inventory_at(root: &Path) -> Result<Vec<(u32, String, PathBuf)>, String> {
    let directory = root.join("crates/canisend-store/migrations");
    let mut migrations = Vec::new();
    for entry in fs::read_dir(&directory).map_err(|error| {
        format!(
            "could not inspect migrations at {}: {error}",
            directory.display()
        )
    })? {
        let entry = entry.map_err(|error| format!("could not inspect migration: {error}"))?;
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "sql") {
            continue;
        }
        reject_symlink(&path)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let prefix = name
            .split_once('_')
            .map(|(prefix, _)| prefix)
            .filter(|prefix| prefix.len() == 4 && prefix.bytes().all(|byte| byte.is_ascii_digit()))
            .ok_or_else(|| format!("migration name is not versioned: `{name}`"))?;
        let version = prefix
            .parse::<u32>()
            .map_err(|error| format!("migration version is invalid in `{name}`: {error}"))?;
        migrations.push((version, name, path));
    }
    migrations.sort_by_key(|(version, _, _)| *version);
    for (index, (version, name, _)) in migrations.iter().enumerate() {
        let expected =
            u32::try_from(index + 1).map_err(|_| "migration inventory exceeds u32".to_owned())?;
        if *version != expected {
            return Err(format!(
                "migration inventory is not contiguous at `{name}`: expected {expected}, found {version}"
            ));
        }
    }
    Ok(migrations)
}

fn declared_database_schema_version() -> Result<u32, String> {
    declared_database_schema_version_at(&repository_root())
}

fn declared_database_schema_version_at(root: &Path) -> Result<u32, String> {
    let path = root.join("crates/canisend-store/src/database.rs");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let prefix = "pub const DATABASE_SCHEMA_VERSION: u32 = ";
    source
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .and_then(|value| value.strip_suffix(';'))
        .ok_or_else(|| "DATABASE_SCHEMA_VERSION declaration is missing".to_owned())?
        .parse()
        .map_err(|error| format!("DATABASE_SCHEMA_VERSION is invalid: {error}"))
}

fn normalize_product_version(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(normalize_product_version),
        Value::Object(fields) => {
            for (name, field) in fields {
                if name == "product_version" {
                    *field = Value::String("<release-version>".to_owned());
                } else {
                    normalize_product_version(field);
                }
            }
        }
        _ => {}
    }
}

fn digest_named_bytes(entries: &[(String, Vec<u8>)]) -> String {
    let mut digest = Sha256::new();
    for (name, bytes) in entries {
        digest.update((name.len() as u64).to_be_bytes());
        digest.update(name.as_bytes());
        digest.update((bytes.len() as u64).to_be_bytes());
        digest.update(bytes);
    }
    hex::encode(digest.finalize())
}

fn release_targets() -> Result<Vec<ReleaseTarget>, String> {
    let path = repository_root().join("release/targets.json");
    let body = fs::read_to_string(&path)
        .map_err(|error| format!("release targets are missing at {}: {error}", path.display()))?;
    let document: Value = serde_json::from_str(&body)
        .map_err(|error| format!("release targets are invalid JSON: {error}"))?;
    if document["schema"] != RELEASE_TARGET_SCHEMA {
        return Err(format!(
            "release target schema must be `{RELEASE_TARGET_SCHEMA}`"
        ));
    }
    let entries = document["targets"]
        .as_array()
        .ok_or_else(|| "release targets must contain an array".to_owned())?;
    let mut targets = Vec::with_capacity(entries.len());
    let mut triples = BTreeSet::new();
    for entry in entries {
        let field = |name: &str| -> Result<String, String> {
            entry[name]
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("release target field `{name}` is missing"))
        };
        let target = ReleaseTarget {
            triple: field("triple")?,
            runner: field("runner")?,
            executable: field("executable")?,
            archive: field("archive")?,
            signing: field("signing")?,
        };
        if !triples.insert(target.triple.clone()) {
            return Err(format!("duplicate release target `{}`", target.triple));
        }
        if !matches!(target.archive.as_str(), "tar.gz" | "zip") {
            return Err(format!(
                "unsupported release archive `{}` for {}",
                target.archive, target.triple
            ));
        }
        targets.push(target);
    }
    if targets.is_empty() {
        return Err("release target list is empty".to_owned());
    }
    Ok(targets)
}

fn write_channel_candidates(tag: &str, assets: &Path, output: &Path) -> Result<(), String> {
    verify_release(tag, assets)?;
    if output.exists() {
        return Err(format!(
            "channel candidate output must not already exist: {}",
            output.display()
        ));
    }
    let version = env!("CARGO_PKG_VERSION");
    let manifest_path = assets.join(format!("canisend-{version}-manifest.json"));
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| {
        format!(
            "release manifest is missing at {}: {error}",
            manifest_path.display()
        )
    })?)
    .map_err(|error| format!("release manifest is invalid JSON: {error}"))?;
    let source = build_channel_candidate_source(tag, &manifest_path, &manifest)?;
    let files = render_channel_candidates(&source)?;

    fs::create_dir_all(output).map_err(|error| {
        format!(
            "could not create channel candidate output {}: {error}",
            output.display()
        )
    })?;
    write_pretty_json(&output.join("candidate-source.json"), &source.to_value())?;
    for (relative, body) in files {
        let path = output.join(&relative);
        let parent = path
            .parent()
            .ok_or_else(|| format!("candidate file has no parent: {relative}"))?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        fs::write(&path, body)
            .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    }
    check_channel_candidate_directory(output)?;
    println!(
        "channel candidates: wrote Homebrew, Scoop, and WinGet candidates to {}",
        output.display()
    );
    Ok(())
}

fn build_channel_candidate_source(
    tag: &str,
    manifest_path: &Path,
    manifest: &Value,
) -> Result<ChannelCandidateSource, String> {
    let version = required_string(manifest, "version", "release manifest")?.to_owned();
    let parsed = Version::parse(&version)
        .map_err(|error| format!("release manifest version is invalid SemVer: {error}"))?;
    let stage = ReleaseStage::from_version(&parsed)?;
    if manifest["tag"] != tag || manifest["stage"] != stage.as_str() {
        return Err("release manifest stage does not match the channel candidate tag".to_owned());
    }
    let source_commit = required_string(&manifest["source"], "commit", "release source")?;
    let repository = required_string(&manifest["source"], "repository", "release source")?;
    let entries = manifest["artifacts"]
        .as_array()
        .ok_or_else(|| "release manifest artifacts are missing".to_owned())?;
    let required_targets = BTreeSet::from([
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
    ]);
    let mut artifacts = BTreeMap::new();
    for entry in entries {
        let target = required_string(entry, "target", "release artifact")?;
        if !required_targets.contains(target) {
            continue;
        }
        let artifact = ChannelArtifact {
            target: target.to_owned(),
            archive: required_string(entry, "archive", "release artifact")?.to_owned(),
            sha256: required_string(entry, "sha256", "release artifact")?.to_owned(),
            size: entry["size"]
                .as_u64()
                .ok_or_else(|| format!("release artifact `{target}` has no size"))?,
        };
        if artifacts.insert(target.to_owned(), artifact).is_some() {
            return Err(format!("duplicate channel artifact `{target}`"));
        }
    }
    if artifacts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != required_targets
    {
        return Err("release manifest does not contain all channel candidate artifacts".to_owned());
    }
    let manifest_file = manifest_path
        .file_name()
        .ok_or_else(|| "release manifest has no file name".to_owned())?
        .to_string_lossy()
        .into_owned();
    let candidate = ChannelCandidateSource {
        tag: tag.to_owned(),
        version,
        stage,
        source_commit: source_commit.to_owned(),
        repository: repository.to_owned(),
        manifest_file,
        manifest_sha256: sha256_file(manifest_path)?,
        artifacts,
    };
    channel_candidate_source_from_value(&candidate.to_value())
}

fn channel_candidate_source_from_value(value: &Value) -> Result<ChannelCandidateSource, String> {
    if value["schema"] != CHANNEL_CANDIDATE_SOURCE_SCHEMA
        || value["candidate_only"] != true
        || value["publication_authorized"] != false
    {
        return Err(
            "channel candidate source must be candidate-only and must not authorize publication"
                .to_owned(),
        );
    }
    let release = &value["release"];
    let tag = required_string(release, "tag", "channel release")?.to_owned();
    let version = required_string(release, "version", "channel release")?.to_owned();
    let parsed = Version::parse(&version)
        .map_err(|error| format!("channel candidate version is invalid SemVer: {error}"))?;
    if tag != format!("v{version}") {
        return Err(format!(
            "channel candidate tag `{tag}` does not match version `{version}`"
        ));
    }
    let stage = ReleaseStage::from_version(&parsed)?;
    if release["stage"] != stage.as_str() {
        return Err("channel candidate stage does not match its version".to_owned());
    }
    let source_commit = required_string(release, "source_commit", "channel release")?.to_owned();
    validate_lower_hex("channel source commit", &source_commit, 40)?;
    let repository = required_string(release, "repository", "channel release")?.to_owned();
    if repository != env!("CARGO_PKG_REPOSITORY") {
        return Err(format!(
            "channel candidate repository must be `{}`",
            env!("CARGO_PKG_REPOSITORY")
        ));
    }
    let manifest_file = required_string(release, "manifest_file", "channel release")?.to_owned();
    if manifest_file != format!("canisend-{version}-manifest.json") {
        return Err("channel candidate manifest file does not match its version".to_owned());
    }
    let manifest_sha256 =
        required_string(release, "manifest_sha256", "channel release")?.to_owned();
    validate_lower_hex("channel release manifest SHA-256", &manifest_sha256, 64)?;

    let entries = value["artifacts"]
        .as_array()
        .ok_or_else(|| "channel candidate artifacts must be an array".to_owned())?;
    let expected = BTreeMap::from([
        ("aarch64-apple-darwin", "tar.gz"),
        ("x86_64-apple-darwin", "tar.gz"),
        ("x86_64-pc-windows-msvc", "zip"),
    ]);
    let mut artifacts = BTreeMap::new();
    for entry in entries {
        let target = required_string(entry, "target", "channel artifact")?.to_owned();
        let archive = required_string(entry, "archive", "channel artifact")?.to_owned();
        let sha256 = required_string(entry, "sha256", "channel artifact")?.to_owned();
        let size = entry["size"]
            .as_u64()
            .filter(|size| *size > 0)
            .ok_or_else(|| format!("channel artifact `{target}` has no positive size"))?;
        let extension = expected
            .get(target.as_str())
            .ok_or_else(|| format!("unsupported channel artifact target `{target}`"))?;
        let expected_archive = format!("canisend-{version}-{target}.{extension}");
        if archive != expected_archive {
            return Err(format!(
                "channel artifact `{target}` must be named `{expected_archive}`"
            ));
        }
        validate_lower_hex(&format!("channel artifact `{target}` SHA-256"), &sha256, 64)?;
        let artifact = ChannelArtifact {
            target: target.clone(),
            archive,
            sha256,
            size,
        };
        if artifacts.insert(target.clone(), artifact).is_some() {
            return Err(format!("duplicate channel artifact `{target}`"));
        }
    }
    if artifacts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected.keys().copied().collect()
    {
        return Err("channel candidate source has an incomplete artifact set".to_owned());
    }
    let source = ChannelCandidateSource {
        tag,
        version,
        stage,
        source_commit,
        repository,
        manifest_file,
        manifest_sha256,
        artifacts,
    };
    if source.to_value() != *value {
        return Err("channel candidate source contains unknown or non-canonical fields".to_owned());
    }
    Ok(source)
}

fn render_channel_candidates(
    source: &ChannelCandidateSource,
) -> Result<BTreeMap<String, String>, String> {
    render_channel_manifest_files(
        &source.version,
        &source.tag,
        &source.repository,
        &source.artifacts,
    )
}

fn channel_license(version: &str) -> Result<&'static str, String> {
    let version = Version::parse(version)
        .map_err(|error| format!("channel candidate version is invalid SemVer: {error}"))?;
    let first_gpl = Version::parse(FIRST_GPL_PUBLIC_VERSION)
        .expect("the first GPL public version constant must be valid SemVer");
    Ok(if version < first_gpl {
        "MIT"
    } else {
        GPL_LICENSE
    })
}

fn render_channel_manifest_files(
    version: &str,
    tag: &str,
    repository: &str,
    artifacts: &BTreeMap<String, ChannelArtifact>,
) -> Result<BTreeMap<String, String>, String> {
    let artifact = |target: &str| {
        artifacts
            .get(target)
            .ok_or_else(|| format!("channel manifest source has no `{target}` artifact"))
    };
    let arm = artifact("aarch64-apple-darwin")?;
    let intel = artifact("x86_64-apple-darwin")?;
    let windows = artifact("x86_64-pc-windows-msvc")?;
    let license = channel_license(version)?;
    let download = |archive: &str| format!("{repository}/releases/download/{tag}/{archive}");
    let homebrew = format!(
        r##"cask "canisend" do
  arch arm: "aarch64", intel: "x86_64"

  version "{version}"
  sha256 arm:   "{arm_sha256}",
         intel: "{intel_sha256}"

  url "{repository}/releases/download/v#{{version}}/canisend-#{{version}}-#{{arch}}-apple-darwin.tar.gz"
  name "CanISend"
  desc "Prepare evidence-backed academic job applications with agent hosts"
  homepage "{repository}"

  binary "canisend-#{{version}}-#{{arch}}-apple-darwin/canisend"
end
"##,
        repository = repository,
        version = version,
        arm_sha256 = arm.sha256,
        intel_sha256 = intel.sha256,
    );
    let scoop = serde_json::to_string_pretty(&json!({
        "version": version,
        "description": "Prepare evidence-backed academic job applications with agent hosts",
        "homepage": repository,
        "license": license,
        "architecture": {
            "64bit": {
                "url": download(&windows.archive),
                "hash": windows.sha256,
            }
        },
        "extract_dir": format!("canisend-{version}-x86_64-pc-windows-msvc"),
        "bin": "canisend.exe",
    }))
    .map_err(|error| format!("could not render Scoop candidate: {error}"))?
        + "\n";

    let identifier = "PengJiaxin.CanISend";
    let winget_base = format!("winget/manifests/p/PengJiaxin/CanISend/{version}/");
    let winget_version = format!(
        r#"# yaml-language-server: $schema=https://aka.ms/winget-manifest.version.{schema}.schema.json

PackageIdentifier: {identifier}
PackageVersion: {version}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: {schema}
"#,
        schema = WINGET_MANIFEST_VERSION,
        version = version,
    );
    let winget_locale = format!(
        r#"# yaml-language-server: $schema=https://aka.ms/winget-manifest.defaultLocale.{schema}.schema.json

PackageIdentifier: {identifier}
PackageVersion: {version}
PackageLocale: en-US
Publisher: Peng Jiaxin
PublisherUrl: https://github.com/jxpeng98
PublisherSupportUrl: {repository}/issues
PackageName: CanISend
PackageUrl: {repository}
License: {license}
LicenseUrl: {repository}/blob/{tag}/LICENSE
ShortDescription: Prepare evidence-backed academic job applications with agent hosts
Moniker: canisend
Tags:
- academic-jobs
- agent
- cli
ReleaseNotesUrl: {repository}/releases/tag/{tag}
ManifestType: defaultLocale
ManifestVersion: {schema}
"#,
        schema = WINGET_MANIFEST_VERSION,
        version = version,
        repository = repository,
        tag = tag,
        license = license,
    );
    let winget_installer = format!(
        r#"# yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.{schema}.schema.json

PackageIdentifier: {identifier}
PackageVersion: {version}
InstallerType: zip
NestedInstallerType: portable
NestedInstallerFiles:
- RelativeFilePath: canisend-{version}-x86_64-pc-windows-msvc\canisend.exe
  PortableCommandAlias: canisend
UpgradeBehavior: install
Installers:
- Architecture: x64
  InstallerUrl: {url}
  InstallerSha256: {sha256}
ManifestType: installer
ManifestVersion: {schema}
"#,
        schema = WINGET_MANIFEST_VERSION,
        version = version,
        url = download(&windows.archive),
        sha256 = windows.sha256,
    );

    Ok(BTreeMap::from([
        ("homebrew/Casks/canisend.rb".to_owned(), homebrew),
        ("scoop/bucket/canisend.json".to_owned(), scoop),
        (format!("{winget_base}{identifier}.yaml"), winget_version),
        (
            format!("{winget_base}{identifier}.locale.en-US.yaml"),
            winget_locale,
        ),
        (
            format!("{winget_base}{identifier}.installer.yaml"),
            winget_installer,
        ),
    ]))
}

fn stable_channel_asset_names(version: &str) -> BTreeSet<String> {
    BTreeSet::from([
        format!("canisend-{version}-channel-publication.json"),
        format!("canisend-{version}-homebrew-cask.rb"),
        format!("canisend-{version}-scoop.json"),
        format!("canisend-{version}-winget-installer.yaml"),
        format!("canisend-{version}-winget-locale.yaml"),
        format!("canisend-{version}-winget-version.yaml"),
    ])
}

fn stable_channel_asset_identity(
    version: &str,
    relative: &str,
) -> Result<(String, &'static str, String), String> {
    match relative {
        "homebrew/Casks/canisend.rb" => Ok((
            format!("canisend-{version}-homebrew-cask.rb"),
            "homebrew-cask",
            "Casks/canisend.rb".to_owned(),
        )),
        "scoop/bucket/canisend.json" => Ok((
            format!("canisend-{version}-scoop.json"),
            "scoop",
            "bucket/canisend.json".to_owned(),
        )),
        path if path.ends_with("PengJiaxin.CanISend.installer.yaml") => Ok((
            format!("canisend-{version}-winget-installer.yaml"),
            "winget",
            path.strip_prefix("winget/")
                .ok_or_else(|| "WinGet publication path has no channel prefix".to_owned())?
                .to_owned(),
        )),
        path if path.ends_with("PengJiaxin.CanISend.locale.en-US.yaml") => Ok((
            format!("canisend-{version}-winget-locale.yaml"),
            "winget",
            path.strip_prefix("winget/")
                .ok_or_else(|| "WinGet publication path has no channel prefix".to_owned())?
                .to_owned(),
        )),
        path if path.ends_with("PengJiaxin.CanISend.yaml") => Ok((
            format!("canisend-{version}-winget-version.yaml"),
            "winget",
            path.strip_prefix("winget/")
                .ok_or_else(|| "WinGet publication path has no channel prefix".to_owned())?
                .to_owned(),
        )),
        _ => Err(format!(
            "unknown stable package-manager manifest path `{relative}`"
        )),
    }
}

fn channel_artifacts_from_release_entries(
    version: &str,
    entries: &[Value],
) -> Result<BTreeMap<String, ChannelArtifact>, String> {
    let required = BTreeSet::from([
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
    ]);
    let mut artifacts = BTreeMap::new();
    for entry in entries {
        let target = required_string(entry, "target", "stable channel artifact")?;
        if !required.contains(target) {
            continue;
        }
        let archive = required_string(entry, "archive", "stable channel artifact")?.to_owned();
        let sha256 = required_string(entry, "sha256", "stable channel artifact")?.to_owned();
        validate_lower_hex(&format!("stable channel `{target}` SHA-256"), &sha256, 64)?;
        let size = entry["size"]
            .as_u64()
            .filter(|size| *size > 0)
            .ok_or_else(|| format!("stable channel artifact `{target}` has no size"))?;
        let extension = if target == "x86_64-pc-windows-msvc" {
            "zip"
        } else {
            "tar.gz"
        };
        let expected_archive = format!("canisend-{version}-{target}.{extension}");
        if archive != expected_archive {
            return Err(format!(
                "stable channel artifact `{target}` must be `{expected_archive}`"
            ));
        }
        let artifact = ChannelArtifact {
            target: target.to_owned(),
            archive,
            sha256,
            size,
        };
        if artifacts.insert(target.to_owned(), artifact).is_some() {
            return Err(format!("duplicate stable channel artifact `{target}`"));
        }
    }
    if artifacts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != required
    {
        return Err("stable release lacks a complete package-manager artifact set".to_owned());
    }
    Ok(artifacts)
}

fn render_stable_channel_publication(
    tag: &str,
    commit: &str,
    archive_entries: &[Value],
    ledger: &Value,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let (version, stage) = parse_release_tag(tag)?;
    if stage != ReleaseStage::Stable
        || ledger["schema"] != RELEASE_QUALIFICATION_SCHEMA
        || ledger["workspace_stage"] != "stable"
        || ledger["status"] != "qualified"
        || ledger["stable_authorized"] != true
    {
        return Err(
            "stable channel publication requires the canonical qualified Stable ledger".to_owned(),
        );
    }
    validate_lower_hex("stable channel source commit", commit, 40)?;
    validate_stable_qualification(ledger)?;
    let candidates = ledger["release_candidates"]
        .as_array()
        .ok_or_else(|| "stable channel publication has no RC matrices".to_owned())?;
    let (beta_tag, package_rc_tag, package_run) = validate_package_manager_qualification_record(
        &ledger["package_managers"],
        &ledger["beta"],
        candidates,
    )?;
    let final_rc = candidates
        .last()
        .ok_or_else(|| "stable channel publication has no final RC".to_owned())?;
    let (final_rc_tag, final_rc_source, final_rc_run) =
        validate_qualification_release(final_rc, ReleaseStage::ReleaseCandidate, "final RC")?;
    let version = version.to_string();
    let artifacts = channel_artifacts_from_release_entries(&version, archive_entries)?;
    let rendered =
        render_channel_manifest_files(&version, tag, env!("CARGO_PKG_REPOSITORY"), &artifacts)?;
    let mut files = BTreeMap::new();
    let mut manifest_entries = Vec::new();
    for (relative, body) in rendered {
        let (asset, channel, repository_path) = stable_channel_asset_identity(&version, &relative)?;
        manifest_entries.push(json!({
            "asset": asset,
            "channel": channel,
            "repository_path": repository_path,
            "sha256": sha256(body.as_bytes()),
            "size": body.len()
        }));
        files.insert(asset, body.into_bytes());
    }
    manifest_entries.sort_by(|left, right| left["asset"].as_str().cmp(&right["asset"].as_str()));
    let artifact_entries = artifacts
        .values()
        .map(|artifact| {
            json!({
                "archive": artifact.archive,
                "sha256": artifact.sha256,
                "size": artifact.size,
                "target": artifact.target
            })
        })
        .collect::<Vec<_>>();
    let source = json!({
        "schema": STABLE_CHANNEL_PUBLICATION_SCHEMA,
        "publication": {
            "authorized": true,
            "external_index_submission": false,
            "scope": "github-release-assets"
        },
        "release": {
            "repository": env!("CARGO_PKG_REPOSITORY"),
            "source_commit": commit,
            "stage": "stable",
            "tag": tag,
            "version": version
        },
        "qualification": {
            "final_rc_run": final_rc_run,
            "final_rc_source_commit": final_rc_source,
            "final_rc_tag": final_rc_tag,
            "package_manager_beta_tag": beta_tag,
            "package_manager_rc_tag": package_rc_tag,
            "package_manager_records": 4,
            "package_manager_run": package_run
        },
        "artifacts": artifact_entries,
        "manifests": manifest_entries
    });
    let source_name = format!("canisend-{version}-channel-publication.json");
    files.insert(source_name, pretty_json_bytes(&source)?);
    Ok(files)
}

fn verify_stable_channel_publication(
    directory: &Path,
    tag: &str,
    release_manifest: &Value,
) -> Result<(), String> {
    let (version, stage) = parse_release_tag(tag)?;
    if stage != ReleaseStage::Stable {
        return Err("stable channel publication cannot verify a prerelease".to_owned());
    }
    let version = version.to_string();
    let source_name = format!("canisend-{version}-channel-publication.json");
    let source_path = directory.join(&source_name);
    reject_symlink(&source_path)?;
    let source: Value = serde_json::from_slice(
        &fs::read(&source_path)
            .map_err(|error| format!("stable channel publication source is missing: {error}"))?,
    )
    .map_err(|error| format!("stable channel publication source is invalid JSON: {error}"))?;
    if source["schema"] != STABLE_CHANNEL_PUBLICATION_SCHEMA
        || source["publication"]
            != json!({
                "authorized": true,
                "external_index_submission": false,
                "scope": "github-release-assets"
            })
        || source["release"]["repository"] != env!("CARGO_PKG_REPOSITORY")
        || source["release"]["source_commit"] != release_manifest["source"]["commit"]
        || source["release"]["stage"] != "stable"
        || source["release"]["tag"] != tag
        || source["release"]["version"] != version
    {
        return Err("stable channel publication identity is invalid".to_owned());
    }
    let qualification = &source["qualification"];
    let final_rc_tag = required_string(
        qualification,
        "final_rc_tag",
        "stable channel qualification",
    )?;
    let final_rc_source = required_string(
        qualification,
        "final_rc_source_commit",
        "stable channel qualification",
    )?;
    let beta_tag = required_string(
        qualification,
        "package_manager_beta_tag",
        "stable channel qualification",
    )?;
    let package_rc_tag = required_string(
        qualification,
        "package_manager_rc_tag",
        "stable channel qualification",
    )?;
    let (final_rc_version, final_rc_stage) = parse_release_tag(final_rc_tag)?;
    let (beta_version, beta_stage) = parse_release_tag(beta_tag)?;
    let (package_rc_version, package_rc_stage) = parse_release_tag(package_rc_tag)?;
    if final_rc_stage != ReleaseStage::ReleaseCandidate
        || beta_stage != ReleaseStage::Beta
        || package_rc_stage != ReleaseStage::ReleaseCandidate
        || (
            final_rc_version.major,
            final_rc_version.minor,
            final_rc_version.patch,
        ) != (version_major_minor_patch(&version)?)
        || (beta_version.major, beta_version.minor, beta_version.patch)
            != version_major_minor_patch(&version)?
        || (
            package_rc_version.major,
            package_rc_version.minor,
            package_rc_version.patch,
        ) != version_major_minor_patch(&version)?
    {
        return Err("stable channel qualification tags differ from the release line".to_owned());
    }
    validate_lower_hex("stable channel final RC source", final_rc_source, 40)?;
    let final_rc_run = qualification["final_rc_run"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| "stable channel qualification has no final RC run".to_owned())?;
    let package_run = qualification["package_manager_run"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| "stable channel qualification has no package-manager run".to_owned())?;
    if qualification["package_manager_records"] != 4 {
        return Err("stable channel qualification must bind four package records".to_owned());
    }

    let release_entries = release_manifest["artifacts"]
        .as_array()
        .ok_or_else(|| "stable release manifest has no artifacts".to_owned())?;
    let artifacts = channel_artifacts_from_release_entries(&version, release_entries)?;
    let artifact_entries = artifacts
        .values()
        .map(|artifact| {
            json!({
                "archive": artifact.archive,
                "sha256": artifact.sha256,
                "size": artifact.size,
                "target": artifact.target
            })
        })
        .collect::<Vec<_>>();
    if source["artifacts"].as_array() != Some(&artifact_entries) {
        return Err("stable channel artifacts differ from the release manifest".to_owned());
    }
    let rendered =
        render_channel_manifest_files(&version, tag, env!("CARGO_PKG_REPOSITORY"), &artifacts)?;
    let mut manifest_entries = Vec::new();
    for (relative, body) in rendered {
        let (asset, channel, repository_path) = stable_channel_asset_identity(&version, &relative)?;
        let path = directory.join(&asset);
        reject_symlink(&path)?;
        if fs::read(&path)
            .map_err(|error| format!("stable channel manifest `{asset}` is missing: {error}"))?
            != body.as_bytes()
        {
            return Err(format!(
                "stable channel manifest `{asset}` differs from canonical rendering"
            ));
        }
        manifest_entries.push(json!({
            "asset": asset,
            "channel": channel,
            "repository_path": repository_path,
            "sha256": sha256(body.as_bytes()),
            "size": body.len()
        }));
    }
    manifest_entries.sort_by(|left, right| left["asset"].as_str().cmp(&right["asset"].as_str()));
    if source["manifests"].as_array() != Some(&manifest_entries) {
        return Err("stable channel manifest inventory is not canonical".to_owned());
    }
    let canonical = json!({
        "schema": STABLE_CHANNEL_PUBLICATION_SCHEMA,
        "publication": {
            "authorized": true,
            "external_index_submission": false,
            "scope": "github-release-assets"
        },
        "release": {
            "repository": env!("CARGO_PKG_REPOSITORY"),
            "source_commit": release_manifest["source"]["commit"],
            "stage": "stable",
            "tag": tag,
            "version": version
        },
        "qualification": {
            "final_rc_run": final_rc_run,
            "final_rc_source_commit": final_rc_source,
            "final_rc_tag": final_rc_tag,
            "package_manager_beta_tag": beta_tag,
            "package_manager_rc_tag": package_rc_tag,
            "package_manager_records": 4,
            "package_manager_run": package_run
        },
        "artifacts": artifact_entries,
        "manifests": manifest_entries
    });
    if source != canonical {
        return Err(
            "stable channel publication contains unknown or non-canonical fields".to_owned(),
        );
    }
    Ok(())
}

fn version_major_minor_patch(version: &str) -> Result<(u64, u64, u64), String> {
    let version = Version::parse(version)
        .map_err(|error| format!("stable channel version is invalid: {error}"))?;
    Ok((version.major, version.minor, version.patch))
}

fn check_channel_candidates() -> Result<(), String> {
    let root = repository_root().join("packaging/candidates");
    let mut entries = fs::read_dir(&root)
        .map_err(|error| format!("channel candidate directory is missing: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("could not inspect channel candidates: {error}"))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    if entries.is_empty() {
        return Err("no package-manager channel candidates exist".to_owned());
    }
    let mut has_alpha_baseline = false;
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(format!(
                "channel candidate entry must be a regular directory: {}",
                path.display()
            ));
        }
        let source = check_channel_candidate_directory(&path)?;
        if path.file_name().and_then(|name| name.to_str()) != Some(source.tag.as_str()) {
            return Err(format!(
                "channel candidate directory {} must be named `{}`",
                path.display(),
                source.tag
            ));
        }
        if source.tag == NATIVE_ALPHA_TAG && source.source_commit == NATIVE_ALPHA_SOURCE {
            has_alpha_baseline = true;
        }
    }
    if !has_alpha_baseline {
        return Err(
            "channel candidates do not retain the qualified native Alpha baseline".to_owned(),
        );
    }
    println!("channel candidates: ok");
    Ok(())
}

fn check_package_manager_qualification_policy() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/package-manager-qualification-policy.json");
    let actual: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "package-manager qualification policy is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("package-manager qualification policy is invalid JSON: {error}"))?;
    let lifecycle = [
        "install-beta",
        "run-version-and-doctor",
        "create-external-workspace",
        "upgrade-to-rc",
        "run-version-and-doctor",
        "uninstall",
        "prove-workspace-retained",
    ];
    let expected = json!({
        "schema": PACKAGE_MANAGER_QUALIFICATION_POLICY_SCHEMA,
        "publication_authorized": false,
        "release_pair": {
            "from_stage": "beta",
            "to_stage": "rc",
            "same_release_line": true,
            "require_public_signed_candidates": true
        },
        "channels": [
            {
                "id": "homebrew-cask",
                "targets": ["aarch64-apple-darwin", "x86_64-apple-darwin"],
                "official_validators": ["brew-style", "brew-audit-strict-cask"],
                "lifecycle": lifecycle
            },
            {
                "id": "scoop",
                "targets": ["x86_64-pc-windows-msvc"],
                "official_validators": ["scoop-manifest-install"],
                "lifecycle": lifecycle
            },
            {
                "id": "winget",
                "targets": ["x86_64-pc-windows-msvc"],
                "official_validators": ["winget-validate", "winget-sandbox-test"],
                "lifecycle": [
                    "validate-beta-manifest",
                    "validate-rc-manifest",
                    "sandbox-install-beta",
                    "run-version-and-doctor",
                    "create-external-workspace",
                    "upgrade-to-rc",
                    "run-version-and-doctor",
                    "uninstall",
                    "prove-workspace-retained"
                ]
            }
        ],
        "evidence": {
            "schema": "canisend.package-manager-qualification/v1",
            "required_records": [
                "homebrew-aarch64-apple-darwin",
                "homebrew-x86_64-apple-darwin",
                "scoop-x86_64-pc-windows-msvc",
                "winget-x86_64-pc-windows-msvc"
            ],
            "bind_candidate_source_sha256": true,
            "bind_github_run_id": true,
            "required_checks": [
                "candidate-sources-verified",
                "official-validation",
                "install",
                "from-version",
                "from-doctor",
                "workspace-created",
                "upgrade",
                "to-version",
                "to-doctor",
                "uninstall",
                "workspace-retained",
                "no-publication"
            ],
            "all_checks_must_pass": true
        }
    });
    if actual != expected {
        return Err(
            "package-manager qualification policy differs from the native release contract"
                .to_owned(),
        );
    }
    let documentation_path = root.join("docs/release/package-manager-qualification.md");
    let documentation = fs::read_to_string(&documentation_path).map_err(|error| {
        format!("package-manager qualification documentation is missing: {error}")
    })?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    for required in [
        "verify-package-evidence",
        "record-package-qualification",
        "fresh-Sandbox record",
        "--write",
    ] {
        if !documentation.contains(required) {
            return Err(format!(
                "package-manager qualification documentation is missing `{required}`"
            ));
        }
    }

    let workflow_path = root.join(".github/workflows/package-manager-qualification.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("package-manager qualification workflow is missing: {error}"))?;
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let release = format!("v{}.{}.{}", version.major, version.minor, version.patch);
    for required in [
        "name: package-manager-prequalification",
        "workflow_dispatch:",
        &format!("default: \"{release}-beta.1\""),
        &format!("default: \"{release}-rc.1\""),
        "verify-package-candidates",
        "macos-15-intel",
        "windows-2025",
        "b0ee913725139b816f9178163af0aecdba07a7ed",
        "48f6ea398b3a3fa26fae0093d37bd85b13e7eaa5d1d4a3e208408768408e35ae",
        "winget-sandbox-kit",
        "No external package repository was changed.",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "package-manager qualification workflow is missing `{required}`"
            ));
        }
    }
    let homebrew = fs::read_to_string(root.join("scripts/qualify_homebrew_packages.sh"))
        .map_err(|error| format!("Homebrew qualification script is missing: {error}"))?;
    for required in [
        "brew tap-new --no-git",
        "mkdir -p \"$(dirname \"$tap_cask\")\"",
        "brew audit --strict --cask",
        "brew install --cask",
        "brew upgrade --cask",
        "brew uninstall --cask",
        "brew untap",
        "workspace-retained",
        "no-publication",
    ] {
        if !homebrew.contains(required) {
            return Err(format!(
                "Homebrew qualification script is missing `{required}`"
            ));
        }
    }
    let windows = fs::read_to_string(root.join("scripts/qualify_windows_packages.ps1"))
        .map_err(|error| format!("Windows package qualification script is missing: {error}"))?;
    for required in [
        "([System.Uri]::new((Resolve-Path -LiteralPath $bucketRoot).Path)).AbsoluteUri",
        "Invoke-Checked -Command scoop -Arguments @(\"bucket\", \"add\", $bucketName, $bucketUri)",
        "$bucketAdded = $true",
        "$bucketAdded = $false",
        "if ($Channel -eq \"scoop\" -and $bucketAdded)",
        "Invoke-Checked -Command scoop -Arguments @(\"update\", \"canisend\")",
        "Invoke-Checked -Command winget -Arguments @(\"validate\"",
        "Invoke-Checked -Command winget -Arguments @(\"install\"",
        "Invoke-Checked -Command winget -Arguments @(\"upgrade\"",
        "Invoke-Checked -Command winget -Arguments @(\"uninstall\"",
        "@(\"settings\", \"--enable\", \"LocalManifestFiles\")",
        "@(\"settings\", \"--disable\", \"LocalManifestFiles\")",
        "workspace-retained",
        "no-publication",
    ] {
        if !windows.contains(required) {
            return Err(format!(
                "Windows qualification script is missing `{required}`"
            ));
        }
    }
    let sandbox_path = root.join("docs/release/winget-sandbox-qualification.md");
    let sandbox = fs::read_to_string(&sandbox_path)
        .map_err(|error| format!("WinGet Sandbox qualification guide is missing: {error}"))?;
    check_local_markdown_links(&root, &sandbox_path, &sandbox)?;
    println!("package-manager qualification policy: ok (4 native records)");
    Ok(())
}

fn check_upgrade_qualification_policy() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/upgrade-qualification-policy.json");
    let actual: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "upgrade qualification policy is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("upgrade qualification policy is invalid JSON: {error}"))?;
    let required_checks = [
        "verified-release-pair",
        "from-version-and-doctor",
        "workspace-created-and-checked",
        "verified-pre-upgrade-backup",
        "to-version-and-doctor",
        "workspace-upgraded-and-checked",
        "old-binary-behavior-verified",
        "backup-restored-to-new-path",
        "restored-workspace-checked-by-old-binary",
        "host-pack-regenerated",
        "installed-binary-and-notices-uninstalled",
        "workspace-backup-and-restore-retained",
        "no-publication",
    ];
    let expected = json!({
        "schema": UPGRADE_QUALIFICATION_POLICY_SCHEMA,
        "release_pair": {
            "from_stage": "beta",
            "to_stage": "rc",
            "same_release_line": true,
            "public_signed_assets_required": true
        },
        "records": [
            {
                "record": "upgrade-aarch64-apple-darwin",
                "target": "aarch64-apple-darwin",
                "environment": "macos-15"
            },
            {
                "record": "upgrade-x86_64-apple-darwin",
                "target": "x86_64-apple-darwin",
                "environment": "macos-15-intel"
            },
            {
                "record": "upgrade-x86_64-unknown-linux-gnu",
                "target": "x86_64-unknown-linux-gnu",
                "environment": "ubuntu-24.04"
            },
            {
                "record": "upgrade-x86_64-unknown-linux-musl",
                "target": "x86_64-unknown-linux-musl",
                "environment": "ubuntu-24.04"
            },
            {
                "record": "upgrade-x86_64-pc-windows-msvc",
                "target": "x86_64-pc-windows-msvc",
                "environment": "windows-2025"
            }
        ],
        "allowed_old_binary_behavior": [
            "same-schema-accepted",
            "future-schema-rejected-without-mutation"
        ],
        "required_checks": required_checks,
        "evidence": {
            "schema": UPGRADE_QUALIFICATION_SCHEMA,
            "one_github_run": true,
            "one_manifest_pair": true,
            "all_checks_must_pass": true,
            "exact_record_set": true
        },
        "publication_authorized": false
    });
    if actual != expected {
        return Err(
            "upgrade qualification policy differs from the native release contract".to_owned(),
        );
    }

    let documentation_path = root.join("docs/release/upgrade-qualification.md");
    let documentation = fs::read_to_string(&documentation_path)
        .map_err(|error| format!("upgrade qualification documentation is missing: {error}"))?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    for required in [
        "native-upgrade-qualification",
        "verify-upgrade-evidence",
        "record-upgrade-qualification",
        "gh attestation verify",
        "--write",
    ] {
        if !documentation.contains(required) {
            return Err(format!(
                "upgrade qualification documentation is missing `{required}`"
            ));
        }
    }

    let workflow_path = root.join(".github/workflows/upgrade-qualification.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("upgrade qualification workflow is missing: {error}"))?;
    let version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("workspace version is invalid: {error}"))?;
    let release = format!("v{}.{}.{}", version.major, version.minor, version.patch);
    for required in [
        "name: native-upgrade-qualification",
        "workflow_dispatch:",
        &format!("default: \"{release}-beta.1\""),
        &format!("default: \"{release}-rc.1\""),
        "gh attestation verify",
        "qualify_archive_upgrade.sh",
        "macos-15-intel",
        "ubuntu-24.04",
        "windows-2025",
        "verify-upgrade-evidence",
        "No release or package channel was changed.",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "upgrade qualification workflow is missing `{required}`"
            ));
        }
    }
    let script = fs::read_to_string(root.join("scripts/qualify_archive_upgrade.sh"))
        .map_err(|error| format!("archive upgrade qualification script is missing: {error}"))?;
    for required in [
        ".canisend/state.sqlite3",
        "workspace backup",
        "workspace restore",
        "workspace.conflict",
        "agent assets export --host codex",
        "installed-binary-and-notices-uninstalled",
        "no-publication",
    ] {
        if !script.contains(required) {
            return Err(format!(
                "archive upgrade qualification script is missing `{required}`"
            ));
        }
    }
    println!("upgrade qualification policy: ok (5 native records)");
    Ok(())
}

fn check_documentation_uninstall_policy() -> Result<(), String> {
    let root = repository_root();
    let path = root.join("release/documentation-uninstall-policy.json");
    let actual: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "documentation/uninstall policy is missing at {}: {error}",
            path.display()
        )
    })?)
    .map_err(|error| format!("documentation/uninstall policy is invalid JSON: {error}"))?;
    let required_checks = [
        "exact-binary-match",
        "complete-notice-bundle",
        "version-and-doctor",
        "documented-quickstart",
        "agent-v4-host-smoke",
        "agent-v4-mcp-lifecycle-smoke",
        "isolated-install",
        "uninstall",
        "workspace-retained",
        "no-publication",
    ];
    let expected = json!({
        "schema": DOCUMENTATION_UNINSTALL_POLICY_SCHEMA,
        "release_stage": "rc",
        "same_run_as_qualified_rc": true,
        "records": [
            {
                "record": "documentation-uninstall-aarch64-apple-darwin",
                "target": "aarch64-apple-darwin",
                "environment": "macos-15"
            },
            {
                "record": "documentation-uninstall-x86_64-apple-darwin",
                "target": "x86_64-apple-darwin",
                "environment": "macos-15-intel"
            },
            {
                "record": "documentation-uninstall-x86_64-unknown-linux-gnu",
                "target": "x86_64-unknown-linux-gnu",
                "environment": "ubuntu-24.04"
            },
            {
                "record": "documentation-uninstall-x86_64-unknown-linux-musl",
                "target": "x86_64-unknown-linux-musl",
                "environment": "ubuntu-24.04"
            },
            {
                "record": "documentation-uninstall-x86_64-pc-windows-msvc",
                "target": "x86_64-pc-windows-msvc",
                "environment": "windows-2025"
            }
        ],
        "required_checks": required_checks,
        "evidence": {
            "schema": DOCUMENTATION_UNINSTALL_SCHEMA,
            "exact_record_set": true,
            "one_github_run": true,
            "bind_verified_archive_sha256": true,
            "all_checks_must_pass": true
        },
        "publication_authorized": false
    });
    if actual != expected {
        return Err(
            "documentation/uninstall policy differs from the native release contract".to_owned(),
        );
    }
    let documentation_path = root.join("docs/release/documentation-uninstall-qualification.md");
    let documentation = fs::read_to_string(&documentation_path).map_err(|error| {
        format!("documentation/uninstall qualification guide is missing: {error}")
    })?;
    check_local_markdown_links(&root, &documentation_path, &documentation)?;
    for required in [
        "verify-documentation-evidence",
        "record-documentation-qualification",
        "same RC run",
        "--write",
    ] {
        if !documentation.contains(required) {
            return Err(format!(
                "documentation/uninstall qualification guide is missing `{required}`"
            ));
        }
    }
    let workflow = fs::read_to_string(root.join(".github/workflows/release.yml"))
        .map_err(|error| format!("release workflow is missing: {error}"))?;
    for required in [
        "documentation-uninstall-${{ matrix.target }}.json",
        "verify-documentation-evidence",
        "documentation-uninstall-evidence",
        "needs.release-identity.outputs.stage == 'rc'",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "release workflow is missing documentation/uninstall gate `{required}`"
            ));
        }
    }
    let script = fs::read_to_string(root.join("scripts/smoke_release_archive.sh"))
        .map_err(|error| format!("release archive smoke is missing: {error}"))?;
    for required in [
        "canisend.documentation-uninstall/v1",
        "exact-binary-match",
        "complete-notice-bundle",
        "workspace-retained",
        "no-publication",
    ] {
        if !script.contains(required) {
            return Err(format!(
                "release archive smoke is missing evidence field `{required}`"
            ));
        }
    }
    println!("documentation/uninstall policy: ok (5 same-RC-run records)");
    Ok(())
}

fn verify_package_manager_evidence(
    from_tag: &str,
    to_tag: &str,
    directory: &Path,
) -> Result<PackageManagerQualificationSummary, String> {
    let (from_version, from_stage) = parse_release_tag(from_tag)?;
    let (to_version, to_stage) = parse_release_tag(to_tag)?;
    if from_stage != ReleaseStage::Beta || to_stage != ReleaseStage::ReleaseCandidate {
        return Err("package-manager qualification requires a Beta-to-RC tag pair".to_owned());
    }
    if (from_version.major, from_version.minor, from_version.patch)
        != (to_version.major, to_version.minor, to_version.patch)
    {
        return Err("package-manager qualification tags must use the same release line".to_owned());
    }
    let expected = BTreeMap::from([
        (
            "homebrew-aarch64-apple-darwin.json",
            (
                "homebrew-aarch64-apple-darwin",
                "homebrew-cask",
                "aarch64-apple-darwin",
                "macos-15",
            ),
        ),
        (
            "homebrew-x86_64-apple-darwin.json",
            (
                "homebrew-x86_64-apple-darwin",
                "homebrew-cask",
                "x86_64-apple-darwin",
                "macos-15-intel",
            ),
        ),
        (
            "scoop-x86_64-pc-windows-msvc.json",
            (
                "scoop-x86_64-pc-windows-msvc",
                "scoop",
                "x86_64-pc-windows-msvc",
                "windows-2025",
            ),
        ),
        (
            "winget-x86_64-pc-windows-msvc.json",
            (
                "winget-x86_64-pc-windows-msvc",
                "winget",
                "x86_64-pc-windows-msvc",
                "windows-sandbox",
            ),
        ),
    ]);
    let mut actual_paths = BTreeSet::new();
    collect_relative_files(directory, directory, &mut actual_paths)?;
    if actual_paths != expected.keys().map(|name| (*name).to_owned()).collect() {
        return Err(format!(
            "package-manager evidence file set differs: expected {:?}, found {actual_paths:?}",
            expected.keys().collect::<Vec<_>>()
        ));
    }

    let mut run_ids = BTreeSet::new();
    let mut from_digests = BTreeSet::new();
    let mut to_digests = BTreeSet::new();
    let expected_records = expected.len();
    for (file, (record, channel, target, environment)) in expected {
        let path = directory.join(file);
        reject_symlink(&path)?;
        let value: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
            format!(
                "could not read package-manager evidence {}: {error}",
                path.display()
            )
        })?)
        .map_err(|error| format!("package-manager evidence `{file}` is invalid JSON: {error}"))?;
        let (run_id, from_digest, to_digest) = validate_package_manager_evidence_record(
            &value,
            record,
            channel,
            target,
            environment,
            from_tag,
            to_tag,
        )?;
        run_ids.insert(run_id);
        from_digests.insert(from_digest);
        to_digests.insert(to_digest);
    }
    if run_ids.len() != 1 || from_digests.len() != 1 || to_digests.len() != 1 {
        return Err(
            "package-manager evidence records must bind one run and one shared candidate pair"
                .to_owned(),
        );
    }
    if from_digests == to_digests {
        return Err("Beta and RC candidate-source digests must differ".to_owned());
    }
    let summary = PackageManagerQualificationSummary {
        run_id: *run_ids.first().expect("one checked run ID"),
        records: expected_records,
    };
    println!(
        "package-manager evidence: ok ({from_tag} -> {to_tag}, run {})",
        summary.run_id
    );
    Ok(summary)
}

fn verify_upgrade_qualification_evidence(
    from_tag: &str,
    to_tag: &str,
    directory: &Path,
) -> Result<UpgradeQualificationSummary, String> {
    let (from_version, from_stage) = parse_release_tag(from_tag)?;
    let (to_version, to_stage) = parse_release_tag(to_tag)?;
    if from_stage != ReleaseStage::Beta || to_stage != ReleaseStage::ReleaseCandidate {
        return Err("upgrade qualification requires a Beta-to-RC tag pair".to_owned());
    }
    if (from_version.major, from_version.minor, from_version.patch)
        != (to_version.major, to_version.minor, to_version.patch)
    {
        return Err("upgrade qualification tags must use the same release line".to_owned());
    }
    let expected = BTreeMap::from([
        (
            "upgrade-aarch64-apple-darwin.json",
            (
                "upgrade-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
            ),
        ),
        (
            "upgrade-x86_64-apple-darwin.json",
            (
                "upgrade-x86_64-apple-darwin",
                "x86_64-apple-darwin",
                "macos-15-intel",
            ),
        ),
        (
            "upgrade-x86_64-unknown-linux-gnu.json",
            (
                "upgrade-x86_64-unknown-linux-gnu",
                "x86_64-unknown-linux-gnu",
                "ubuntu-24.04",
            ),
        ),
        (
            "upgrade-x86_64-unknown-linux-musl.json",
            (
                "upgrade-x86_64-unknown-linux-musl",
                "x86_64-unknown-linux-musl",
                "ubuntu-24.04",
            ),
        ),
        (
            "upgrade-x86_64-pc-windows-msvc.json",
            (
                "upgrade-x86_64-pc-windows-msvc",
                "x86_64-pc-windows-msvc",
                "windows-2025",
            ),
        ),
    ]);
    let mut actual_paths = BTreeSet::new();
    collect_relative_files(directory, directory, &mut actual_paths)?;
    if actual_paths != expected.keys().map(|name| (*name).to_owned()).collect() {
        return Err(format!(
            "upgrade evidence file set differs: expected {:?}, found {actual_paths:?}",
            expected.keys().collect::<Vec<_>>()
        ));
    }

    let mut run_ids = BTreeSet::new();
    let mut from_manifests = BTreeSet::new();
    let mut to_manifests = BTreeSet::new();
    let mut from_archives = BTreeSet::new();
    let mut to_archives = BTreeSet::new();
    for (file, (record, target, environment)) in &expected {
        let path = directory.join(file);
        reject_symlink(&path)?;
        let value: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
            format!(
                "could not read upgrade evidence {}: {error}",
                path.display()
            )
        })?)
        .map_err(|error| format!("upgrade evidence `{file}` is invalid JSON: {error}"))?;
        let (run, from_manifest, to_manifest, from_archive, to_archive) =
            validate_upgrade_qualification_record(
                &value,
                record,
                target,
                environment,
                from_tag,
                to_tag,
            )?;
        run_ids.insert(run);
        from_manifests.insert(from_manifest);
        to_manifests.insert(to_manifest);
        from_archives.insert(from_archive);
        to_archives.insert(to_archive);
    }
    if run_ids.len() != 1 || from_manifests.len() != 1 || to_manifests.len() != 1 {
        return Err(
            "upgrade evidence records must bind one run and one shared release manifest pair"
                .to_owned(),
        );
    }
    if from_manifests == to_manifests {
        return Err("Beta and RC upgrade manifest digests must differ".to_owned());
    }
    if from_archives.len() != expected.len()
        || to_archives.len() != expected.len()
        || !from_archives.is_disjoint(&to_archives)
    {
        return Err(
            "upgrade evidence must bind distinct Beta and RC archives for all five targets"
                .to_owned(),
        );
    }
    let summary = UpgradeQualificationSummary {
        run_id: *run_ids.first().expect("one checked run ID"),
        from_manifest_sha256: from_manifests
            .first()
            .expect("one checked Beta manifest")
            .to_owned(),
        to_manifest_sha256: to_manifests
            .first()
            .expect("one checked RC manifest")
            .to_owned(),
        records: expected.len(),
    };
    println!(
        "upgrade evidence: ok ({from_tag} -> {to_tag}, run {}, {} targets)",
        summary.run_id, summary.records
    );
    Ok(summary)
}

#[allow(clippy::too_many_arguments)]
fn validate_upgrade_qualification_record(
    value: &Value,
    expected_record: &str,
    expected_target: &str,
    expected_environment: &str,
    from_tag: &str,
    to_tag: &str,
) -> Result<(u64, String, String, String, String), String> {
    let context = format!("upgrade evidence `{expected_record}`");
    let run_id = value["github_run_id"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| format!("{context} has no positive GitHub run ID"))?;
    let from_manifest = required_string(&value["manifests"], "from_sha256", &context)?.to_owned();
    let to_manifest = required_string(&value["manifests"], "to_sha256", &context)?.to_owned();
    let from_archive = required_string(&value["archives"], "from_sha256", &context)?.to_owned();
    let to_archive = required_string(&value["archives"], "to_sha256", &context)?.to_owned();
    for (name, digest) in [
        ("Beta manifest", &from_manifest),
        ("RC manifest", &to_manifest),
        ("Beta archive", &from_archive),
        ("RC archive", &to_archive),
    ] {
        validate_lower_hex(&format!("{context} {name} digest"), digest, 64)?;
    }
    if from_manifest == to_manifest || from_archive == to_archive {
        return Err(format!("{context} must bind distinct Beta and RC bytes"));
    }
    let before_schema = value["database_schemas"]["before"]
        .as_u64()
        .filter(|schema| *schema > 0 && *schema <= u64::from(u32::MAX))
        .ok_or_else(|| format!("{context} has an invalid pre-upgrade schema"))?;
    let after_schema = value["database_schemas"]["after"]
        .as_u64()
        .filter(|schema| *schema > 0 && *schema <= u64::from(u32::MAX))
        .ok_or_else(|| format!("{context} has an invalid post-upgrade schema"))?;
    let old_binary_behavior = required_string(value, "old_binary_behavior", &context)?;
    match old_binary_behavior {
        "same-schema-accepted" if before_schema == after_schema => {}
        "future-schema-rejected-without-mutation" if after_schema > before_schema => {}
        _ => {
            return Err(format!(
                "{context} old-binary behavior does not match the observed schemas"
            ));
        }
    }
    let completed_at = required_string(value, "completed_at", &context)?;
    if !completed_at.ends_with('Z') || OffsetDateTime::parse(completed_at, &Rfc3339).is_err() {
        return Err(format!("{context} completion timestamp must be valid UTC"));
    }
    let checks = value["checks"]
        .as_object()
        .ok_or_else(|| format!("{context} checks are missing"))?;
    let required_checks = [
        "verified-release-pair",
        "from-version-and-doctor",
        "workspace-created-and-checked",
        "verified-pre-upgrade-backup",
        "to-version-and-doctor",
        "workspace-upgraded-and-checked",
        "old-binary-behavior-verified",
        "backup-restored-to-new-path",
        "restored-workspace-checked-by-old-binary",
        "host-pack-regenerated",
        "installed-binary-and-notices-uninstalled",
        "workspace-backup-and-restore-retained",
        "no-publication",
    ];
    if checks.len() != required_checks.len()
        || required_checks
            .iter()
            .any(|check| checks.get(*check) != Some(&Value::Bool(true)))
    {
        return Err(format!("{context} does not pass every required check"));
    }
    let expected = json!({
        "schema": UPGRADE_QUALIFICATION_SCHEMA,
        "record": expected_record,
        "target": expected_target,
        "environment": expected_environment,
        "from_tag": from_tag,
        "to_tag": to_tag,
        "manifests": {
            "from_sha256": from_manifest,
            "to_sha256": to_manifest
        },
        "archives": {
            "from_sha256": from_archive,
            "to_sha256": to_archive
        },
        "github_run_id": run_id,
        "observed_versions": {
            "from": from_tag.trim_start_matches('v'),
            "to": to_tag.trim_start_matches('v')
        },
        "database_schemas": {
            "before": before_schema,
            "after": after_schema
        },
        "old_binary_behavior": old_binary_behavior,
        "checks": checks,
        "completed_at": completed_at
    });
    if *value != expected {
        return Err(format!(
            "{context} contains unknown, noncanonical, or mismatched fields"
        ));
    }
    Ok((run_id, from_manifest, to_manifest, from_archive, to_archive))
}

fn verify_documentation_uninstall_evidence(
    tag: &str,
    assets: &Path,
    directory: &Path,
) -> Result<DocumentationUninstallSummary, String> {
    let (version, stage) = parse_release_tag(tag)?;
    if stage != ReleaseStage::ReleaseCandidate {
        return Err("documentation/uninstall evidence requires an RC tag".to_owned());
    }
    verify_release(tag, assets)?;
    let manifest_path = assets.join(format!("canisend-{version}-manifest.json"));
    let manifest: Value = serde_json::from_slice(
        &fs::read(&manifest_path)
            .map_err(|error| format!("could not read verified RC release manifest: {error}"))?,
    )
    .map_err(|error| format!("verified RC release manifest is invalid JSON: {error}"))?;
    let artifacts = manifest["artifacts"]
        .as_array()
        .ok_or_else(|| "verified RC release manifest artifacts are missing".to_owned())?;
    let expected = BTreeMap::from([
        (
            "documentation-uninstall-aarch64-apple-darwin.json",
            (
                "documentation-uninstall-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
            ),
        ),
        (
            "documentation-uninstall-x86_64-apple-darwin.json",
            (
                "documentation-uninstall-x86_64-apple-darwin",
                "x86_64-apple-darwin",
                "macos-15-intel",
            ),
        ),
        (
            "documentation-uninstall-x86_64-unknown-linux-gnu.json",
            (
                "documentation-uninstall-x86_64-unknown-linux-gnu",
                "x86_64-unknown-linux-gnu",
                "ubuntu-24.04",
            ),
        ),
        (
            "documentation-uninstall-x86_64-unknown-linux-musl.json",
            (
                "documentation-uninstall-x86_64-unknown-linux-musl",
                "x86_64-unknown-linux-musl",
                "ubuntu-24.04",
            ),
        ),
        (
            "documentation-uninstall-x86_64-pc-windows-msvc.json",
            (
                "documentation-uninstall-x86_64-pc-windows-msvc",
                "x86_64-pc-windows-msvc",
                "windows-2025",
            ),
        ),
    ]);
    let mut actual_paths = BTreeSet::new();
    collect_relative_files(directory, directory, &mut actual_paths)?;
    if actual_paths != expected.keys().map(|name| (*name).to_owned()).collect() {
        return Err(format!(
            "documentation/uninstall evidence file set differs: expected {:?}, found {actual_paths:?}",
            expected.keys().collect::<Vec<_>>()
        ));
    }
    let mut run_ids = BTreeSet::new();
    let mut archive_digests = BTreeSet::new();
    for (file, (record, target, environment)) in &expected {
        let manifest_artifact = artifacts
            .iter()
            .find(|artifact| artifact["target"] == *target)
            .ok_or_else(|| format!("verified RC manifest has no `{target}` archive"))?;
        let expected_archive_sha =
            required_string(manifest_artifact, "sha256", "verified RC archive")?;
        let path = directory.join(file);
        reject_symlink(&path)?;
        let value: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
            format!(
                "could not read documentation/uninstall evidence {}: {error}",
                path.display()
            )
        })?)
        .map_err(|error| {
            format!("documentation/uninstall evidence `{file}` is invalid JSON: {error}")
        })?;
        let (run, archive_digest) = validate_documentation_uninstall_record(
            &value,
            record,
            target,
            environment,
            tag,
            expected_archive_sha,
        )?;
        run_ids.insert(run);
        archive_digests.insert(archive_digest);
    }
    if run_ids.len() != 1 || archive_digests.len() != expected.len() {
        return Err(
            "documentation/uninstall evidence must bind one run and five distinct verified archives"
                .to_owned(),
        );
    }
    let summary = DocumentationUninstallSummary {
        run_id: *run_ids.first().expect("one checked run ID"),
        records: expected.len(),
    };
    println!(
        "documentation/uninstall evidence: ok ({tag}, run {}, {} targets)",
        summary.run_id, summary.records
    );
    Ok(summary)
}

#[allow(clippy::too_many_arguments)]
fn validate_documentation_uninstall_record(
    value: &Value,
    expected_record: &str,
    expected_target: &str,
    expected_environment: &str,
    tag: &str,
    expected_archive_sha: &str,
) -> Result<(u64, String), String> {
    let context = format!("documentation/uninstall evidence `{expected_record}`");
    validate_lower_hex(
        &format!("{context} verified archive digest"),
        expected_archive_sha,
        64,
    )?;
    let archive_sha = required_string(value, "archive_sha256", &context)?.to_owned();
    validate_lower_hex(&format!("{context} archive digest"), &archive_sha, 64)?;
    if archive_sha != expected_archive_sha {
        return Err(format!(
            "{context} archive digest differs from the verified RC manifest"
        ));
    }
    let run_id = value["github_run_id"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| format!("{context} has no positive GitHub run ID"))?;
    let completed_at = required_string(value, "completed_at", &context)?;
    if !completed_at.ends_with('Z') || OffsetDateTime::parse(completed_at, &Rfc3339).is_err() {
        return Err(format!("{context} completion timestamp must be valid UTC"));
    }
    let checks = value["checks"]
        .as_object()
        .ok_or_else(|| format!("{context} checks are missing"))?;
    let required_checks = [
        "exact-binary-match",
        "complete-notice-bundle",
        "version-and-doctor",
        "documented-quickstart",
        "agent-v4-host-smoke",
        "agent-v4-mcp-lifecycle-smoke",
        "isolated-install",
        "uninstall",
        "workspace-retained",
        "no-publication",
    ];
    if checks.len() != required_checks.len()
        || required_checks
            .iter()
            .any(|check| checks.get(*check) != Some(&Value::Bool(true)))
    {
        return Err(format!("{context} does not pass every required check"));
    }
    let expected = json!({
        "schema": DOCUMENTATION_UNINSTALL_SCHEMA,
        "record": expected_record,
        "target": expected_target,
        "environment": expected_environment,
        "tag": tag,
        "archive_sha256": archive_sha,
        "github_run_id": run_id,
        "observed_version": tag.trim_start_matches('v'),
        "checks": checks,
        "completed_at": completed_at
    });
    if *value != expected {
        return Err(format!(
            "{context} contains unknown, noncanonical, or mismatched fields"
        ));
    }
    Ok((run_id, archive_sha))
}

fn verify_package_candidate_pair(
    from_tag: &str,
    from_assets: &Path,
    to_tag: &str,
    to_assets: &Path,
) -> Result<(), String> {
    let (from_version, from_stage) = parse_release_tag(from_tag)?;
    let (to_version, to_stage) = parse_release_tag(to_tag)?;
    if from_stage != ReleaseStage::Beta || to_stage != ReleaseStage::ReleaseCandidate {
        return Err("package-manager candidates require a Beta-to-RC tag pair".to_owned());
    }
    if (from_version.major, from_version.minor, from_version.patch)
        != (to_version.major, to_version.minor, to_version.patch)
    {
        return Err("package-manager candidate tags must use the same release line".to_owned());
    }
    verify_release(from_tag, from_assets)?;
    verify_release(to_tag, to_assets)?;

    let root = repository_root().join("packaging/candidates");
    let from_source = check_channel_candidate_directory(&root.join(from_tag))?;
    let to_source = check_channel_candidate_directory(&root.join(to_tag))?;
    validate_package_candidate_source_against_assets(
        &from_source,
        from_tag,
        from_stage,
        from_assets,
    )?;
    validate_package_candidate_source_against_assets(&to_source, to_tag, to_stage, to_assets)?;
    if from_source.source_commit == to_source.source_commit
        || from_source.manifest_sha256 == to_source.manifest_sha256
    {
        return Err("Beta and RC candidates must bind distinct release sources".to_owned());
    }
    println!("package-manager candidates: ok ({from_tag} -> {to_tag})");
    Ok(())
}

fn validate_package_candidate_source_against_assets(
    source: &ChannelCandidateSource,
    expected_tag: &str,
    expected_stage: ReleaseStage,
    assets: &Path,
) -> Result<(), String> {
    if source.tag != expected_tag || source.stage != expected_stage {
        return Err(format!(
            "package-manager candidate `{expected_tag}` has the wrong release identity"
        ));
    }
    let manifest_path = assets.join(&source.manifest_file);
    if sha256_file(&manifest_path)? != source.manifest_sha256 {
        return Err(format!(
            "package-manager candidate `{expected_tag}` does not bind the verified public manifest"
        ));
    }
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| {
        format!(
            "could not read package-manager source manifest {}: {error}",
            manifest_path.display()
        )
    })?)
    .map_err(|error| format!("package-manager source manifest is invalid JSON: {error}"))?;
    if manifest["tag"] != expected_tag
        || manifest["version"] != source.version
        || manifest["stage"] != expected_stage.as_str()
        || manifest["source"]["commit"] != source.source_commit
    {
        return Err(format!(
            "package-manager candidate `{expected_tag}` differs from its verified release manifest"
        ));
    }
    let manifest_artifacts = manifest["artifacts"]
        .as_array()
        .ok_or_else(|| "verified release manifest artifacts are missing".to_owned())?;
    for (target, source_artifact) in &source.artifacts {
        let manifest_artifact = manifest_artifacts
            .iter()
            .find(|artifact| artifact["target"] == target.as_str())
            .ok_or_else(|| format!("verified release manifest has no `{target}` artifact"))?;
        if manifest_artifact["archive"] != source_artifact.archive
            || manifest_artifact["sha256"] != source_artifact.sha256
            || manifest_artifact["size"] != source_artifact.size
        {
            return Err(format!(
                "package-manager candidate artifact `{target}` differs from the verified release"
            ));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_package_manager_evidence_record(
    value: &Value,
    expected_record: &str,
    expected_channel: &str,
    expected_target: &str,
    expected_environment: &str,
    from_tag: &str,
    to_tag: &str,
) -> Result<(u64, String, String), String> {
    let context = format!("package-manager evidence `{expected_record}`");
    let run_id = value["github_run_id"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| format!("{context} has no positive GitHub run ID"))?;
    let from_digest = required_string(value, "from_candidate_source_sha256", &context)?.to_owned();
    let to_digest = required_string(value, "to_candidate_source_sha256", &context)?.to_owned();
    validate_lower_hex(
        &format!("{context} Beta candidate digest"),
        &from_digest,
        64,
    )?;
    validate_lower_hex(&format!("{context} RC candidate digest"), &to_digest, 64)?;
    let tool_version = required_string(value, "tool_version", &context)?;
    let completed_at = required_string(value, "completed_at", &context)?;
    if !completed_at.ends_with('Z') {
        return Err(format!("{context} completion timestamp must be UTC"));
    }
    let checks = value["checks"]
        .as_object()
        .ok_or_else(|| format!("{context} checks are missing"))?;
    let required_checks = [
        "candidate-sources-verified",
        "official-validation",
        "install",
        "from-version",
        "from-doctor",
        "workspace-created",
        "upgrade",
        "to-version",
        "to-doctor",
        "uninstall",
        "workspace-retained",
        "no-publication",
    ];
    if checks.len() != required_checks.len()
        || required_checks
            .iter()
            .any(|check| checks.get(*check) != Some(&Value::Bool(true)))
    {
        return Err(format!("{context} does not pass every required check"));
    }
    let expected = json!({
        "schema": PACKAGE_MANAGER_QUALIFICATION_SCHEMA,
        "record": expected_record,
        "channel": expected_channel,
        "target": expected_target,
        "environment": expected_environment,
        "from_tag": from_tag,
        "to_tag": to_tag,
        "from_candidate_source_sha256": from_digest,
        "to_candidate_source_sha256": to_digest,
        "github_run_id": run_id,
        "tool_version": tool_version,
        "observed_versions": {
            "from": from_tag.trim_start_matches('v'),
            "to": to_tag.trim_start_matches('v')
        },
        "checks": checks,
        "completed_at": completed_at
    });
    if *value != expected {
        return Err(format!(
            "{context} contains unknown, noncanonical, or mismatched fields"
        ));
    }
    Ok((run_id, from_digest, to_digest))
}

fn parse_release_tag(tag: &str) -> Result<(Version, ReleaseStage), String> {
    let version = Version::parse(
        tag.strip_prefix('v')
            .ok_or_else(|| format!("release tag `{tag}` must start with `v`"))?,
    )
    .map_err(|error| format!("release tag `{tag}` is invalid SemVer: {error}"))?;
    let stage = ReleaseStage::from_version(&version)?;
    Ok((version, stage))
}

fn check_channel_candidate_directory(path: &Path) -> Result<ChannelCandidateSource, String> {
    let source_path = path.join("candidate-source.json");
    reject_symlink(&source_path)?;
    let source_value: Value = serde_json::from_slice(&fs::read(&source_path).map_err(|error| {
        format!(
            "candidate source is missing at {}: {error}",
            source_path.display()
        )
    })?)
    .map_err(|error| format!("candidate source is invalid JSON: {error}"))?;
    let source = channel_candidate_source_from_value(&source_value)?;
    let expected = render_channel_candidates(&source)?;
    let mut actual_paths = BTreeSet::new();
    collect_relative_files(path, path, &mut actual_paths)?;
    actual_paths.remove("candidate-source.json");
    if actual_paths != expected.keys().cloned().collect() {
        return Err(format!(
            "channel candidate file set differs at {}: expected {:?}, found {actual_paths:?}",
            path.display(),
            expected.keys().collect::<Vec<_>>()
        ));
    }
    for (relative, expected_body) in expected {
        let actual = fs::read_to_string(path.join(&relative))
            .map_err(|error| format!("could not read channel candidate `{relative}`: {error}"))?;
        if actual != expected_body {
            return Err(format!(
                "channel candidate `{relative}` drifted from its verified release source"
            ));
        }
    }
    Ok(source)
}

fn collect_relative_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeSet<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("could not inspect {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("could not inspect candidate file: {error}"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "channel candidate tree contains a symlink: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            collect_relative_files(root, &path, files)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("could not relativize candidate path: {error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            if !files.insert(relative.clone()) {
                return Err(format!("duplicate channel candidate file `{relative}`"));
            }
        } else {
            return Err(format!(
                "channel candidate is not a regular file: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn required_string<'a>(value: &'a Value, name: &str, context: &str) -> Result<&'a str, String> {
    value[name]
        .as_str()
        .filter(|field| !field.is_empty())
        .ok_or_else(|| format!("{context} field `{name}` is missing"))
}

fn validate_lower_hex(context: &str, value: &str, length: usize) -> Result<(), String> {
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{context} must be exactly {length} lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

fn bind_signing_evidence(
    tag: &str,
    target_name: &str,
    evidence_path: &Path,
    binary_path: &Path,
    archive_path: &Path,
) -> Result<(), String> {
    validate_release_tag(tag)?;
    let version = env!("CARGO_PKG_VERSION");
    let target = release_targets()?
        .into_iter()
        .find(|target| target.triple == target_name)
        .ok_or_else(|| format!("unknown release signing target `{target_name}`"))?;
    if target.signing == "none" {
        return Err(format!(
            "release target `{target_name}` does not use platform code signing"
        ));
    }
    reject_symlink(binary_path)?;
    if binary_path.file_name().and_then(|name| name.to_str()) != Some(&target.executable) {
        return Err(format!(
            "signing evidence binary must be named `{}`",
            target.executable
        ));
    }
    reject_symlink(evidence_path)?;
    reject_symlink(archive_path)?;
    let expected_archive = format!("canisend-{version}-{target_name}.{}", target.archive);
    if archive_path.file_name().and_then(|name| name.to_str()) != Some(&expected_archive) {
        return Err(format!(
            "signing evidence archive must be named `{expected_archive}`"
        ));
    }
    let actual: Value = serde_json::from_slice(&fs::read(evidence_path).map_err(|error| {
        format!(
            "signing evidence is missing at {}: {error}",
            evidence_path.display()
        )
    })?)
    .map_err(|error| format!("signing evidence is invalid JSON: {error}"))?;
    let mut canonical = canonical_signing_evidence(&actual, &target, version, None)?;
    if canonical != actual {
        return Err("unbound signing evidence contains unknown or non-canonical fields".to_owned());
    }
    if canonical["binary"]["sha256"] != sha256_file(binary_path)?
        || canonical["binary"]["size"] != file_size(binary_path)?
    {
        return Err(format!(
            "signing evidence does not match signed binary `{}`",
            binary_path.display()
        ));
    }
    canonical["archive"] = json!({
        "file": expected_archive,
        "sha256": sha256_file(archive_path)?,
        "size": file_size(archive_path)?,
    });
    let canonical = canonical_signing_evidence(&canonical, &target, version, Some(archive_path))?;
    write_pretty_json(evidence_path, &canonical)?;
    println!(
        "signing evidence: bound {target_name} to {}",
        archive_path.display()
    );
    Ok(())
}

fn read_bound_signing_evidence(
    path: &Path,
    target: &ReleaseTarget,
    version: &str,
    archive: &Path,
) -> Result<Value, String> {
    reject_symlink(path)?;
    let actual: Value =
        serde_json::from_slice(&fs::read(path).map_err(|error| {
            format!("signing evidence is missing at {}: {error}", path.display())
        })?)
        .map_err(|error| format!("signing evidence is invalid JSON: {error}"))?;
    let canonical = canonical_signing_evidence(&actual, target, version, Some(archive))?;
    if actual != canonical {
        return Err(format!(
            "signing evidence contains unknown or non-canonical fields: {}",
            path.display()
        ));
    }
    Ok(canonical)
}

fn canonical_signing_evidence(
    value: &Value,
    target: &ReleaseTarget,
    version: &str,
    archive: Option<&Path>,
) -> Result<Value, String> {
    if value["schema"] != CODE_SIGNING_EVIDENCE_SCHEMA
        || value["version"] != version
        || value["target"] != target.triple
        || value["status"] != "verified"
    {
        return Err(format!(
            "code-signing evidence identity is invalid for `{}`",
            target.triple
        ));
    }
    let expected_kind = match target.signing.as_str() {
        "apple-adhoc" => "apple-adhoc",
        "authenticode-self-signed" => "windows-authenticode-self-signed",
        other => {
            return Err(format!(
                "target `{}` has unsupported signing kind `{other}`",
                target.triple
            ));
        }
    };
    if value["kind"] != expected_kind {
        return Err(format!(
            "code-signing evidence kind is invalid for `{}`",
            target.triple
        ));
    }
    let binary = &value["binary"];
    if binary["file"] != target.executable {
        return Err(format!(
            "signed binary file is invalid for `{}`",
            target.triple
        ));
    }
    let binary_sha = required_string(binary, "sha256", "signed binary")?;
    validate_lower_hex("signed binary SHA-256", binary_sha, 64)?;
    let binary_size = binary["size"]
        .as_u64()
        .filter(|size| *size > 0)
        .ok_or_else(|| "signed binary has no positive size".to_owned())?;

    let archive_value = if let Some(archive_path) = archive {
        reject_symlink(archive_path)?;
        let expected_name = format!("canisend-{version}-{}.{}", target.triple, target.archive);
        if archive_path.file_name().and_then(|name| name.to_str()) != Some(&expected_name) {
            return Err(format!("signed archive must be named `{expected_name}`"));
        }
        let archive_value = &value["archive"];
        let archive_sha = required_string(archive_value, "sha256", "signed archive")?;
        validate_lower_hex("signed archive SHA-256", archive_sha, 64)?;
        let archive_size = archive_value["size"]
            .as_u64()
            .filter(|size| *size > 0)
            .ok_or_else(|| "signed archive has no positive size".to_owned())?;
        if archive_value["file"] != expected_name
            || archive_sha != sha256_file(archive_path)?
            || archive_size != file_size(archive_path)?
        {
            return Err(format!(
                "code-signing evidence is not bound to `{expected_name}`"
            ));
        }
        json!({
            "file": expected_name,
            "sha256": archive_sha,
            "size": archive_size,
        })
    } else {
        if !value["archive"].is_null() {
            return Err("unbound signing evidence must use a null archive".to_owned());
        }
        Value::Null
    };

    let signer = &value["signer"];
    let identity = bounded_evidence_string(signer, "identity", "signer", 256)?;
    let (canonical_signer, canonical_verification) = match target.signing.as_str() {
        "apple-adhoc" => {
            if identity != "adhoc" {
                return Err("macOS ad-hoc signer identity is invalid".to_owned());
            }
            let code_identifier =
                bounded_evidence_string(signer, "code_identifier", "macOS ad-hoc signer", 128)?;
            if code_identifier != "io.github.jxpeng98.canisend" {
                return Err("macOS ad-hoc code-signing identifier is invalid".to_owned());
            }
            let verification = &value["verification"];
            if verification["codesign_valid"] != true
                || verification["adhoc"] != true
                || verification["developer_id"] != false
                || verification["hardened_runtime"] != true
                || verification["secure_timestamp"] != false
                || verification["notarized"] != false
                || verification["gatekeeper_trusted_publisher"] != false
                || verification["get_task_allow"] != false
            {
                return Err("macOS ad-hoc signing evidence is incomplete".to_owned());
            }
            (
                json!({
                    "identity": identity,
                    "code_identifier": code_identifier,
                }),
                json!({
                    "codesign_valid": true,
                    "adhoc": true,
                    "developer_id": false,
                    "hardened_runtime": true,
                    "secure_timestamp": false,
                    "notarized": false,
                    "gatekeeper_trusted_publisher": false,
                    "get_task_allow": false,
                }),
            )
        }
        "authenticode-self-signed" => {
            if identity != "CN=CanISend Community Build" {
                return Err("Windows self-signed identity is invalid".to_owned());
            }
            let thumbprint = required_string(signer, "thumbprint", "Windows signer")?;
            validate_lower_hex("Windows signer thumbprint", thumbprint, 40)?;
            let verification = &value["verification"];
            let authenticode_status = required_string(
                verification,
                "authenticode_status",
                "Windows self-signed signature",
            )?;
            if !matches!(authenticode_status, "NotTrusted" | "UnknownError")
                || verification["signature_present"] != true
                || verification["self_signed"] != true
                || verification["certificate_trusted"] != false
                || verification["file_digest"] != "SHA256"
                || verification["timestamp_present"] != false
                || verification["service"] != "powershell-self-signed-authenticode"
            {
                return Err("Windows self-signed Authenticode evidence is incomplete".to_owned());
            }
            (
                json!({
                    "identity": identity,
                    "thumbprint": thumbprint,
                }),
                json!({
                    "authenticode_status": authenticode_status,
                    "signature_present": true,
                    "self_signed": true,
                    "certificate_trusted": false,
                    "file_digest": "SHA256",
                    "timestamp_present": false,
                    "service": "powershell-self-signed-authenticode",
                }),
            )
        }
        _ => unreachable!("signing kind was checked above"),
    };

    Ok(json!({
        "schema": CODE_SIGNING_EVIDENCE_SCHEMA,
        "version": version,
        "target": target.triple,
        "kind": expected_kind,
        "status": "verified",
        "binary": {
            "file": target.executable,
            "sha256": binary_sha,
            "size": binary_size,
        },
        "archive": archive_value,
        "signer": canonical_signer,
        "verification": canonical_verification,
    }))
}

fn bounded_evidence_string<'a>(
    value: &'a Value,
    name: &str,
    context: &str,
    maximum: usize,
) -> Result<&'a str, String> {
    let field = required_string(value, name, context)?;
    if field.len() > maximum || field.chars().any(char::is_control) {
        return Err(format!(
            "{context} field `{name}` exceeds its bound or contains control characters"
        ));
    }
    Ok(field)
}

fn validate_release_tag(tag: &str) -> Result<ReleaseStage, String> {
    let expected = format!("v{}", env!("CARGO_PKG_VERSION"));
    if tag != expected {
        return Err(format!(
            "release tag `{tag}` does not match workspace version `{expected}`"
        ));
    }
    let version = Version::parse(tag.trim_start_matches('v'))
        .map_err(|error| format!("release tag is not valid SemVer: {error}"))?;
    let stage = ReleaseStage::from_version(&version)?;
    println!("release tag: ok ({tag}, stage {})", stage.as_str());
    Ok(stage)
}

fn write_release_sbom(output: &Path) -> Result<(), String> {
    let root = repository_root();
    let metadata_output = Command::new("cargo")
        .current_dir(&root)
        .args(["metadata", "--format-version", "1", "--locked"])
        .output()
        .map_err(|error| format!("failed to run cargo metadata: {error}"))?;
    if !metadata_output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&metadata_output.stderr).trim()
        ));
    }
    let metadata: Value = serde_json::from_slice(&metadata_output.stdout)
        .map_err(|error| format!("cargo metadata returned invalid JSON: {error}"))?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| "cargo metadata has no packages array".to_owned())?;
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or_else(|| "cargo metadata has no resolve nodes".to_owned())?;
    let package_by_id = packages
        .iter()
        .filter_map(|package| package["id"].as_str().map(|id| (id.to_owned(), package)))
        .collect::<BTreeMap<_, _>>();
    let node_by_id = nodes
        .iter()
        .filter_map(|node| node["id"].as_str().map(|id| (id.to_owned(), node)))
        .collect::<BTreeMap<_, _>>();
    let root_ids = ["canisend-cli", "canisend-gui"]
        .into_iter()
        .map(|name| {
            packages
                .iter()
                .find(|package| package["name"] == name)
                .and_then(|package| package["id"].as_str())
                .map(str::to_owned)
                .ok_or_else(|| format!("cargo metadata does not contain {name}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut included = BTreeSet::new();
    let mut queue = VecDeque::from(root_ids.clone());
    while let Some(id) = queue.pop_front() {
        if !included.insert(id.clone()) {
            continue;
        }
        let node = node_by_id
            .get(&id)
            .ok_or_else(|| format!("cargo metadata resolve node is missing for `{id}`"))?;
        let dependencies = node["dependencies"]
            .as_array()
            .ok_or_else(|| format!("cargo metadata dependencies are missing for `{id}`"))?;
        for dependency in dependencies {
            let dependency = dependency
                .as_str()
                .ok_or_else(|| format!("cargo metadata dependency is not a string for `{id}`"))?;
            queue.push_back(dependency.to_owned());
        }
    }
    let mut components = included
        .iter()
        .map(|id| {
            package_by_id
                .get(id)
                .ok_or_else(|| format!("cargo package metadata is missing for `{id}`"))
                .and_then(|package| {
                    let component_type = if root_ids.contains(id) {
                        "application"
                    } else {
                        "library"
                    };
                    cargo_component(package, component_type)
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    components.sort_by(|left, right| left["bom-ref"].as_str().cmp(&right["bom-ref"].as_str()));
    let mut dependencies = Vec::with_capacity(included.len());
    for id in &included {
        let package = package_by_id
            .get(id)
            .ok_or_else(|| format!("cargo package metadata is missing for `{id}`"))?;
        let node = node_by_id
            .get(id)
            .ok_or_else(|| format!("cargo resolve node is missing for `{id}`"))?;
        let mut depends_on = node["dependencies"]
            .as_array()
            .ok_or_else(|| format!("cargo dependencies are missing for `{id}`"))?
            .iter()
            .filter_map(Value::as_str)
            .filter(|dependency| included.contains(*dependency))
            .map(|dependency| {
                package_by_id
                    .get(dependency)
                    .ok_or_else(|| format!("cargo package metadata is missing for `{dependency}`"))
                    .and_then(|package| cargo_bom_ref(package))
            })
            .collect::<Result<Vec<_>, _>>()?;
        depends_on.sort();
        dependencies.push(json!({
            "ref": cargo_bom_ref(package)?,
            "dependsOn": depends_on,
        }));
    }
    dependencies.sort_by(|left, right| left["ref"].as_str().cmp(&right["ref"].as_str()));
    let mut root_refs = root_ids
        .iter()
        .map(|id| {
            package_by_id
                .get(id)
                .ok_or_else(|| format!("cargo root package metadata is missing for `{id}`"))
                .and_then(|package| cargo_bom_ref(package))
        })
        .collect::<Result<Vec<_>, _>>()?;
    root_refs.sort();
    let product_ref = format!(
        "urn:canisend:product:{}",
        sha256(format!("CanISend@{}", env!("CARGO_PKG_VERSION")).as_bytes())
    );
    dependencies.push(json!({
        "ref": product_ref,
        "dependsOn": root_refs,
    }));
    let sbom = json!({
        "$schema": "https://cyclonedx.org/schema/bom-1.6.schema.json",
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "bom-ref": product_ref,
                "name": "CanISend",
                "version": env!("CARGO_PKG_VERSION"),
                "licenses": [{"license": {"name": env!("CARGO_PKG_LICENSE")}}],
                "externalReferences": [{
                    "type": "vcs",
                    "url": env!("CARGO_PKG_REPOSITORY")
                }]
            },
            "tools": {
                "components": [{
                    "type": "application",
                    "name": "canisend-xtask",
                    "version": env!("CARGO_PKG_VERSION")
                }]
            },
            "properties": [
                {"name": "canisend:agent_protocol", "value": AGENT_PROTOCOL},
                {"name": "canisend:workspace_format", "value": WORKSPACE_FORMAT},
                {"name": "canisend:schema_version", "value": PUBLIC_SCHEMA_VERSION},
                {"name": "canisend:release_surfaces", "value": "standalone-cli,macos-gui"}
            ]
        },
        "components": components,
        "dependencies": dependencies,
        "compositions": [{
            "aggregate": "complete",
            "assemblies": root_refs
        }]
    });
    write_pretty_json(output, &sbom)?;
    println!(
        "release SBOM: wrote {} components to {}",
        included.len(),
        output.display()
    );
    Ok(())
}

fn cargo_component(package: &Value, component_type: &str) -> Result<Value, String> {
    let name = required_json_string(package, "name")?;
    let version = required_json_string(package, "version")?;
    let mut component = Map::from_iter([
        ("type".to_owned(), Value::String(component_type.to_owned())),
        ("bom-ref".to_owned(), Value::String(cargo_bom_ref(package)?)),
        ("name".to_owned(), Value::String(name.to_owned())),
        ("version".to_owned(), Value::String(version.to_owned())),
        (
            "purl".to_owned(),
            Value::String(format!("pkg:cargo/{name}@{version}")),
        ),
    ]);
    if let Some(license) = package["license"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        component.insert(
            "licenses".to_owned(),
            json!([{"license": {"name": license}}]),
        );
    }
    if let Some(repository) = package["repository"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        component.insert(
            "externalReferences".to_owned(),
            json!([{"type": "vcs", "url": repository}]),
        );
    }
    if let Some(checksum) = package["checksum"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        component.insert(
            "hashes".to_owned(),
            json!([{"alg": "SHA-256", "content": checksum}]),
        );
    }
    Ok(Value::Object(component))
}

fn cargo_bom_ref(package: &Value) -> Result<String, String> {
    let id = required_json_string(package, "id")?;
    Ok(format!(
        "urn:canisend:cargo:sha256:{}",
        sha256(id.as_bytes())
    ))
}

fn required_json_string<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value[name]
        .as_str()
        .filter(|field| !field.is_empty())
        .ok_or_else(|| format!("cargo metadata package field `{name}` is missing"))
}

fn macos_gui_archive_name(version: &str) -> String {
    format!("CanISend-{version}-aarch64-apple-darwin.zip")
}

fn macos_gui_dmg_name(version: &str) -> String {
    format!("CanISend-{version}-aarch64-apple-darwin.dmg")
}

fn macos_gui_qualification_name(version: &str) -> String {
    format!("CanISend-{version}-aarch64-apple-darwin-qualification.json")
}

fn macos_gui_dmg_qualification_name(version: &str) -> String {
    format!("CanISend-{version}-aarch64-apple-darwin-dmg-qualification.json")
}

fn macos_gui_intel_compilation_name(version: &str) -> String {
    format!("CanISend-{version}-x86_64-apple-darwin-gui-compilation.json")
}

fn read_macos_gui_qualification(
    path: &Path,
    tag: &str,
    version: &str,
    archive: &Path,
) -> Result<Value, String> {
    reject_symlink(path)?;
    let body = fs::read(path)
        .map_err(|error| format!("could not read macOS GUI qualification evidence: {error}"))?;
    if body.len() > 65_536 {
        return Err("macOS GUI qualification evidence exceeds 65536 bytes".to_owned());
    }
    let value: Value = serde_json::from_slice(&body)
        .map_err(|error| format!("macOS GUI qualification evidence is invalid JSON: {error}"))?;
    let archive_name = macos_gui_archive_name(version);
    let archive_sha256 = sha256_file(archive)?;
    let archive_size = file_size(archive)?;
    let expected_profile = parse_release_tag(tag)?.1.cargo_profile();
    let run_id = value["github_run_id"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| "macOS GUI qualification evidence has no positive run ID".to_owned())?;
    let completed_at = required_string(&value, "completed_at", "macOS GUI qualification")?;
    OffsetDateTime::parse(completed_at, &Rfc3339)
        .map_err(|error| format!("macOS GUI qualification completion time is invalid: {error}"))?;
    let canonical = json!({
        "schema": "canisend.macos-gui-qualification/v1",
        "record": "desktop-macos-aarch64",
        "target": "aarch64-apple-darwin",
        "environment": "macos-15",
        "profile": expected_profile,
        "tag": tag,
        "version": version,
        "archive": {
            "file": archive_name,
            "sha256": archive_sha256,
            "size": archive_size
        },
        "github_run_id": run_id,
        "checks": {
            "bounded_archive": true,
            "exact_top_level": true,
            "no_symlinks": true,
            "companion_integrity": true,
            "nested_adhoc_signatures": true,
            "outer_adhoc_signature": true,
            "version_match": true,
            "packaged_cli_doctor": true,
            "packaged_dual_pack_quickstart": true,
            "packaged_agent_v4_host_resources": true,
            "packaged_agent_v4_mcp_lifecycle": true,
            "packaged_gui_launch": true,
            "no_publication": true
        },
        "completed_at": completed_at
    });
    if value != canonical {
        return Err(
            "macOS GUI qualification evidence is not canonical or does not bind the archive"
                .to_owned(),
        );
    }
    Ok(canonical)
}

fn read_macos_gui_dmg_qualification(
    path: &Path,
    tag: &str,
    version: &str,
    dmg: &Path,
) -> Result<Value, String> {
    reject_symlink(path)?;
    let body = fs::read(path)
        .map_err(|error| format!("could not read macOS GUI DMG qualification evidence: {error}"))?;
    if body.len() > 65_536 {
        return Err("macOS GUI DMG qualification evidence exceeds 65536 bytes".to_owned());
    }
    let value: Value = serde_json::from_slice(&body).map_err(|error| {
        format!("macOS GUI DMG qualification evidence is invalid JSON: {error}")
    })?;
    let dmg_name = macos_gui_dmg_name(version);
    let dmg_sha256 = sha256_file(dmg)?;
    let dmg_size = file_size(dmg)?;
    let expected_profile = parse_release_tag(tag)?.1.cargo_profile();
    let run_id = value["github_run_id"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| "macOS GUI DMG qualification evidence has no positive run ID".to_owned())?;
    let completed_at = required_string(&value, "completed_at", "macOS GUI DMG qualification")?;
    OffsetDateTime::parse(completed_at, &Rfc3339).map_err(|error| {
        format!("macOS GUI DMG qualification completion time is invalid: {error}")
    })?;
    let canonical = json!({
        "schema": "canisend.macos-gui-dmg-qualification/v1",
        "record": "desktop-macos-aarch64-dmg",
        "target": "aarch64-apple-darwin",
        "environment": "macos-15",
        "profile": expected_profile,
        "tag": tag,
        "version": version,
        "image": {
            "file": dmg_name,
            "sha256": dmg_sha256,
            "size": dmg_size
        },
        "github_run_id": run_id,
        "checks": {
            "bounded_image": true,
            "hdiutil_verify": true,
            "readonly_mount": true,
            "exact_top_level": true,
            "applications_link": true,
            "companion_integrity": true,
            "nested_adhoc_signatures": true,
            "outer_adhoc_signature": true,
            "version_match": true,
            "no_publication": true
        },
        "completed_at": completed_at
    });
    if value != canonical {
        return Err(
            "macOS GUI DMG qualification evidence is not canonical or does not bind the image"
                .to_owned(),
        );
    }
    Ok(canonical)
}

fn read_macos_gui_intel_compilation(
    path: &Path,
    tag: &str,
    version: &str,
    commit: &str,
) -> Result<Value, String> {
    reject_symlink(path)?;
    let body = fs::read(path)
        .map_err(|error| format!("could not read macOS Intel GUI compilation evidence: {error}"))?;
    if body.len() > 65_536 {
        return Err("macOS Intel GUI compilation evidence exceeds 65536 bytes".to_owned());
    }
    let value: Value = serde_json::from_slice(&body).map_err(|error| {
        format!("macOS Intel GUI compilation evidence is invalid JSON: {error}")
    })?;
    let run_id = value["github_run_id"]
        .as_u64()
        .filter(|run| *run > 0)
        .ok_or_else(|| "macOS Intel GUI compilation evidence has no positive run ID".to_owned())?;
    let completed_at = required_string(
        &value,
        "completed_at",
        "macOS Intel GUI compilation evidence",
    )?;
    OffsetDateTime::parse(completed_at, &Rfc3339).map_err(|error| {
        format!("macOS Intel GUI compilation completion time is invalid: {error}")
    })?;
    let binary_sha256 = required_string(
        &value["binary"],
        "sha256",
        "macOS Intel GUI compilation binary",
    )?;
    validate_lower_hex(
        "macOS Intel GUI compilation binary SHA-256",
        binary_sha256,
        64,
    )?;
    let binary_size = value["binary"]["size"]
        .as_u64()
        .filter(|size| *size > 0)
        .ok_or_else(|| {
            "macOS Intel GUI compilation evidence has no positive binary size".to_owned()
        })?;
    let canonical = json!({
        "schema": "canisend.macos-gui-compilation/v1",
        "record": "desktop-macos-intel-compile-only",
        "target": "x86_64-apple-darwin",
        "environment": "macos-15-intel",
        "tag": tag,
        "version": version,
        "source_commit": commit,
        "binary": {
            "file": "canisend-gui",
            "architecture": "x86_64",
            "profile": "release",
            "sha256": binary_sha256,
            "size": binary_size
        },
        "github_run_id": run_id,
        "checks": {
            "locked_build": true,
            "release_profile": true,
            "target_architecture": true,
            "archive_published": false,
            "native_runtime_qualified": false,
            "support_claim": false,
            "no_publication": true
        },
        "completed_at": completed_at
    });
    if value != canonical {
        return Err(
            "macOS Intel GUI compilation evidence is not canonical or does not bind the release"
                .to_owned(),
        );
    }
    Ok(canonical)
}

fn assemble_release(
    tag: &str,
    commit: &str,
    artifacts_root: &Path,
    output: &Path,
) -> Result<(), String> {
    let stage = validate_release_tag(tag)?;
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("release commit must be a full 40-character hexadecimal Git commit".to_owned());
    }
    if output.exists() {
        return Err(format!(
            "release output must not already exist: {}",
            output.display()
        ));
    }
    fs::create_dir_all(output).map_err(|error| {
        format!(
            "could not create release output {}: {error}",
            output.display()
        )
    })?;
    let version = env!("CARGO_PKG_VERSION");
    let targets = release_targets()?;
    let mut archive_entries = Vec::with_capacity(targets.len());
    let mut signing_evidence_paths = Vec::new();
    for target in &targets {
        let file_name = format!("canisend-{version}-{}.{}", target.triple, target.archive);
        let source = find_unique_file(artifacts_root, &file_name)?;
        reject_symlink(&source)?;
        let destination = output.join(&file_name);
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "could not copy release archive {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
        let signing_evidence = if !matches!(stage, ReleaseStage::Alpha) && target.signing != "none"
        {
            let evidence_name = format!("canisend-{version}-{}-signing.json", target.triple);
            let evidence_source = find_unique_file(artifacts_root, &evidence_name)?;
            read_bound_signing_evidence(&evidence_source, target, version, &destination)?;
            let evidence_destination = output.join(&evidence_name);
            fs::copy(&evidence_source, &evidence_destination).map_err(|error| {
                format!(
                    "could not copy signing evidence {} to {}: {error}",
                    evidence_source.display(),
                    evidence_destination.display()
                )
            })?;
            signing_evidence_paths.push(evidence_destination);
            Value::String(evidence_name)
        } else {
            Value::Null
        };
        archive_entries.push(json!({
            "archive": file_name,
            "archive_format": target.archive,
            "executable": target.executable,
            "profile": stage.cargo_profile(),
            "runner": target.runner,
            "sha256": sha256_file(&destination)?,
            "signing_kind": target.signing,
            "size": file_size(&destination)?,
            "signing_evidence": signing_evidence,
            "target": target.triple,
        }));
    }
    let desktop_archive_name = macos_gui_archive_name(version);
    let desktop_archive_source = find_unique_file(artifacts_root, &desktop_archive_name)?;
    reject_symlink(&desktop_archive_source)?;
    let desktop_archive = output.join(&desktop_archive_name);
    fs::copy(&desktop_archive_source, &desktop_archive).map_err(|error| {
        format!(
            "could not copy macOS GUI archive {} to {}: {error}",
            desktop_archive_source.display(),
            desktop_archive.display()
        )
    })?;
    let desktop_qualification_name = macos_gui_qualification_name(version);
    let desktop_qualification_source =
        find_unique_file(artifacts_root, &desktop_qualification_name)?;
    read_macos_gui_qualification(
        &desktop_qualification_source,
        tag,
        version,
        &desktop_archive,
    )?;
    let desktop_qualification = output.join(&desktop_qualification_name);
    fs::copy(&desktop_qualification_source, &desktop_qualification).map_err(|error| {
        format!(
            "could not copy macOS GUI qualification evidence {} to {}: {error}",
            desktop_qualification_source.display(),
            desktop_qualification.display()
        )
    })?;
    let desktop_dmg_name = macos_gui_dmg_name(version);
    let desktop_dmg_source = find_unique_file(artifacts_root, &desktop_dmg_name)?;
    reject_symlink(&desktop_dmg_source)?;
    let desktop_dmg = output.join(&desktop_dmg_name);
    fs::copy(&desktop_dmg_source, &desktop_dmg).map_err(|error| {
        format!(
            "could not copy macOS GUI DMG {} to {}: {error}",
            desktop_dmg_source.display(),
            desktop_dmg.display()
        )
    })?;
    let desktop_dmg_qualification_name = macos_gui_dmg_qualification_name(version);
    let desktop_dmg_qualification_source =
        find_unique_file(artifacts_root, &desktop_dmg_qualification_name)?;
    read_macos_gui_dmg_qualification(
        &desktop_dmg_qualification_source,
        tag,
        version,
        &desktop_dmg,
    )?;
    let desktop_dmg_qualification = output.join(&desktop_dmg_qualification_name);
    fs::copy(
        &desktop_dmg_qualification_source,
        &desktop_dmg_qualification,
    )
    .map_err(|error| {
        format!(
            "could not copy macOS GUI DMG qualification evidence {} to {}: {error}",
            desktop_dmg_qualification_source.display(),
            desktop_dmg_qualification.display()
        )
    })?;
    let desktop_entries = vec![
        json!({
            "archive": desktop_archive_name,
            "archive_format": "zip",
            "bundle": "CanISend.app",
            "companion_manifest": "CanISend.app.manifest.json",
            "developer_id": false,
            "distribution": "portable",
            "notarized": false,
            "profile": stage.cargo_profile(),
            "qualification_evidence": desktop_qualification_name,
            "runner": "macos-15",
            "sha256": sha256_file(&desktop_archive)?,
            "signing_kind": "apple-adhoc",
            "size": file_size(&desktop_archive)?,
            "surface": "desktop-gui",
            "target": "aarch64-apple-darwin",
        }),
        json!({
            "applications_link": "/Applications",
            "archive": desktop_dmg_name,
            "archive_format": "dmg",
            "bundle": "CanISend.app",
            "companion_manifest": "CanISend.app.manifest.json",
            "developer_id": false,
            "distribution": "installer",
            "notarized": false,
            "profile": stage.cargo_profile(),
            "qualification_evidence": desktop_dmg_qualification_name,
            "runner": "macos-15",
            "sha256": sha256_file(&desktop_dmg)?,
            "signing_kind": "apple-adhoc",
            "size": file_size(&desktop_dmg)?,
            "surface": "desktop-gui",
            "target": "aarch64-apple-darwin",
        }),
    ];
    let mut desktop_compilation_entries = Vec::new();
    let desktop_intel_compilation = if stage.requires_intel_gui_release_evidence() {
        let evidence_name = macos_gui_intel_compilation_name(version);
        let evidence_source = find_unique_file(artifacts_root, &evidence_name)?;
        read_macos_gui_intel_compilation(
            &evidence_source,
            tag,
            version,
            &commit.to_ascii_lowercase(),
        )?;
        let evidence_destination = output.join(&evidence_name);
        fs::copy(&evidence_source, &evidence_destination).map_err(|error| {
            format!(
                "could not copy macOS Intel GUI compilation evidence {} to {}: {error}",
                evidence_source.display(),
                evidence_destination.display()
            )
        })?;
        desktop_compilation_entries.push(json!({
            "archive": null,
            "evidence": evidence_name,
            "native_runtime_qualified": false,
            "runner": "macos-15-intel",
            "status": "compile-only",
            "surface": "desktop-gui",
            "target": "x86_64-apple-darwin",
        }));
        Some(evidence_destination)
    } else {
        None
    };
    let sbom_name = format!("canisend-{version}-sbom.cdx.json");
    let sbom_path = output.join(&sbom_name);
    write_release_sbom(&sbom_path)?;
    let supplemental_sources = [
        ("KNOWN_LIMITATIONS.md", "release/KNOWN_LIMITATIONS.md"),
        ("ISSUE_COLLECTION.md", "release/ISSUE_COLLECTION.md"),
        ("RELEASE_NOTES.md", "release/RELEASE_NOTES.md"),
        ("THIRD_PARTY_NOTICES.md", "THIRD_PARTY_NOTICES.md"),
    ];
    let mut supplemental_entries = vec![
        release_file_entry(&sbom_path)?,
        release_file_entry(&desktop_qualification)?,
        release_file_entry(&desktop_dmg_qualification)?,
    ];
    if let Some(evidence) = &desktop_intel_compilation {
        supplemental_entries.push(release_file_entry(evidence)?);
    }
    for evidence in &signing_evidence_paths {
        supplemental_entries.push(release_file_entry(evidence)?);
    }
    if matches!(stage, ReleaseStage::Stable) {
        let ledger_path = repository_root().join("release/qualification-ledger.json");
        let ledger: Value = serde_json::from_slice(
            &fs::read(&ledger_path)
                .map_err(|error| format!("could not read Stable qualification ledger: {error}"))?,
        )
        .map_err(|error| format!("Stable qualification ledger is invalid JSON: {error}"))?;
        for (name, body) in render_stable_channel_publication(
            tag,
            &commit.to_ascii_lowercase(),
            &archive_entries,
            &ledger,
        )? {
            let destination = output.join(name);
            fs::write(&destination, body).map_err(|error| {
                format!(
                    "could not write stable channel asset {}: {error}",
                    destination.display()
                )
            })?;
            supplemental_entries.push(release_file_entry(&destination)?);
        }
    }
    for (name, source) in supplemental_sources {
        let source = repository_root().join(source);
        let destination = output.join(name);
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "could not copy supplemental release file {}: {error}",
                source.display()
            )
        })?;
        supplemental_entries.push(release_file_entry(&destination)?);
    }
    supplemental_entries.sort_by(|left, right| left["file"].as_str().cmp(&right["file"].as_str()));
    let manifest_name = format!("canisend-{version}-manifest.json");
    let manifest_path = output.join(&manifest_name);
    let manifest = json!({
        "schema": RELEASE_MANIFEST_SCHEMA,
        "product": "canisend",
        "version": version,
        "tag": tag,
        "stage": stage.as_str(),
        "source": {
            "commit": commit.to_ascii_lowercase(),
            "locked_dependencies": true,
            "repository": env!("CARGO_PKG_REPOSITORY")
        },
        "contracts": {
            "agent_protocol": AGENT_PROTOCOL,
            "public_schema_version": PUBLIC_SCHEMA_VERSION,
            "resource_format": canisend_resources::RESOURCE_VERSION,
            "workspace_format": WORKSPACE_FORMAT
        },
        "artifacts": archive_entries,
        "desktop_artifacts": desktop_entries,
        "desktop_compilation": desktop_compilation_entries,
        "supplemental_files": supplemental_entries,
        "trust": {
            "archive_code_signing_required": !matches!(stage, ReleaseStage::Alpha),
            "desktop_bundle_code_signing_required": true,
            "default_telemetry": false,
            "manifest_attestation": "GitHub OIDC artifact attestation",
            "verification_command": format!(
                "gh attestation verify {manifest_name} --repo {}",
                env!("CARGO_PKG_REPOSITORY").trim_start_matches("https://github.com/")
            )
        }
    });
    write_pretty_json(&manifest_path, &manifest)?;
    write_checksums(output)?;
    verify_release(tag, output)?;
    println!("release assets: assembled {}", output.display());
    Ok(())
}

fn verify_release(tag: &str, directory: &Path) -> Result<(), String> {
    let (parsed_version, stage) = parse_release_tag(tag)?;
    let version = parsed_version.to_string();
    println!("release tag: ok ({tag}, stage {})", stage.as_str());
    let manifest_path = directory.join(format!("canisend-{version}-manifest.json"));
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| {
        format!(
            "release manifest is missing at {}: {error}",
            manifest_path.display()
        )
    })?)
    .map_err(|error| format!("release manifest is invalid JSON: {error}"))?;
    if manifest["schema"] != RELEASE_MANIFEST_SCHEMA
        || manifest["version"] != version
        || manifest["tag"] != tag
        || manifest["stage"] != stage.as_str()
    {
        return Err("release manifest identity does not match this build".to_owned());
    }
    verify_release_manifest_contents(stage, &version, directory, &manifest)?;
    let checksums_path = directory.join("SHA256SUMS");
    let checksums = fs::read_to_string(&checksums_path).map_err(|error| {
        format!(
            "SHA256SUMS is missing at {}: {error}",
            checksums_path.display()
        )
    })?;
    let mut verified = BTreeSet::new();
    for (line_number, line) in checksums.lines().enumerate() {
        let (expected, file_name) = line
            .split_once("  ")
            .ok_or_else(|| format!("invalid SHA256SUMS line {}: `{line}`", line_number + 1))?;
        if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "invalid SHA-256 at SHA256SUMS line {}",
                line_number + 1
            ));
        }
        if file_name.is_empty()
            || file_name.contains('/')
            || file_name.contains('\\')
            || file_name == "SHA256SUMS"
        {
            return Err(format!("unsafe checksum file name `{file_name}`"));
        }
        if !verified.insert(file_name.to_owned()) {
            return Err(format!("duplicate checksum entry `{file_name}`"));
        }
        let actual = sha256_file(&directory.join(file_name))?;
        if actual != expected.to_ascii_lowercase() {
            return Err(format!("checksum mismatch for `{file_name}`"));
        }
    }
    let actual_files = fs::read_dir(directory)
        .map_err(|error| format!("could not inspect release directory: {error}"))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name() != "SHA256SUMS")
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>();
    if verified != actual_files {
        return Err(format!(
            "checksum coverage differs: verified {verified:?}, files {actual_files:?}"
        ));
    }
    println!("release assets: verified {} files", verified.len());
    Ok(())
}

fn verify_release_candidate(
    tag: &str,
    expected_commit: &str,
    directory: &Path,
) -> Result<(), String> {
    validate_lower_hex(
        "expected release candidate source commit",
        expected_commit,
        40,
    )?;
    verify_release(tag, directory)?;
    let version = parse_release_tag(tag)?.0.to_string();
    let manifest_path = directory.join(format!("canisend-{version}-manifest.json"));
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).map_err(|error| {
        format!(
            "release manifest is missing at {}: {error}",
            manifest_path.display()
        )
    })?)
    .map_err(|error| format!("release manifest is invalid JSON: {error}"))?;
    verify_release_candidate_source(&manifest, expected_commit)?;
    println!("release candidate: verified {tag} at {expected_commit}");
    Ok(())
}

fn verify_release_candidate_source(manifest: &Value, expected_commit: &str) -> Result<(), String> {
    validate_lower_hex(
        "expected release candidate source commit",
        expected_commit,
        40,
    )?;
    let actual_commit = required_string(&manifest["source"], "commit", "release source")?;
    validate_lower_hex("release source commit", actual_commit, 40)?;
    if actual_commit != expected_commit {
        return Err(format!(
            "release candidate source commit `{actual_commit}` does not match tagged commit `{expected_commit}`"
        ));
    }
    Ok(())
}

fn verify_release_manifest_contents(
    stage: ReleaseStage,
    version: &str,
    directory: &Path,
    manifest: &Value,
) -> Result<(), String> {
    if manifest["product"] != "canisend"
        || manifest["source"]["locked_dependencies"] != true
        || manifest["source"]["repository"] != env!("CARGO_PKG_REPOSITORY")
        || manifest["contracts"]["agent_protocol"] != AGENT_PROTOCOL
        || manifest["contracts"]["public_schema_version"] != PUBLIC_SCHEMA_VERSION
        || manifest["contracts"]["resource_format"] != canisend_resources::RESOURCE_VERSION
        || manifest["contracts"]["workspace_format"] != WORKSPACE_FORMAT
        || manifest["trust"]["default_telemetry"] != false
        || manifest["trust"]["archive_code_signing_required"]
            != !matches!(stage, ReleaseStage::Alpha)
        || manifest["trust"]["desktop_bundle_code_signing_required"] != true
    {
        return Err("release manifest policy or contract metadata is invalid".to_owned());
    }
    let commit = required_string(&manifest["source"], "commit", "release source")?;
    validate_lower_hex("release source commit", commit, 40)?;

    let targets = release_targets()?;
    let entries = manifest["artifacts"]
        .as_array()
        .ok_or_else(|| "release manifest artifacts are missing".to_owned())?;
    if entries.len() != targets.len() {
        return Err(format!(
            "release manifest must contain exactly {} artifacts",
            targets.len()
        ));
    }
    let mut by_target = BTreeMap::new();
    for entry in entries {
        let target = required_string(entry, "target", "release artifact")?;
        if by_target.insert(target, entry).is_some() {
            return Err(format!("duplicate release manifest target `{target}`"));
        }
    }
    for target in targets {
        let entry = by_target
            .get(target.triple.as_str())
            .ok_or_else(|| format!("release manifest target `{}` is missing", target.triple))?;
        let file_name = format!("canisend-{version}-{}.{}", target.triple, target.archive);
        if entry["archive"] != file_name
            || entry["archive_format"] != target.archive
            || entry["executable"] != target.executable
            || entry["profile"] != stage.cargo_profile()
            || entry["runner"] != target.runner
            || entry["signing_kind"] != target.signing
        {
            return Err(format!(
                "release manifest metadata is invalid for target `{}`",
                target.triple
            ));
        }
        let signing_evidence_name =
            if !matches!(stage, ReleaseStage::Alpha) && target.signing != "none" {
                Some(format!("canisend-{version}-{}-signing.json", target.triple))
            } else {
                None
            };
        match signing_evidence_name {
            Some(ref evidence_name) if entry["signing_evidence"] == *evidence_name => {
                read_bound_signing_evidence(
                    &directory.join(evidence_name),
                    &target,
                    version,
                    &directory.join(&file_name),
                )?;
            }
            None if entry["signing_evidence"].is_null() => {}
            _ => {
                return Err(format!(
                    "release signing evidence reference is invalid for `{}`",
                    target.triple
                ));
            }
        }
        let declared_sha = required_string(entry, "sha256", "release artifact")?;
        validate_lower_hex(
            &format!("release artifact `{}` SHA-256", target.triple),
            declared_sha,
            64,
        )?;
        let declared_size = entry["size"]
            .as_u64()
            .filter(|size| *size > 0)
            .ok_or_else(|| format!("release artifact `{}` has no positive size", target.triple))?;
        let path = directory.join(&file_name);
        reject_symlink(&path)?;
        if sha256_file(&path)? != declared_sha || file_size(&path)? != declared_size {
            return Err(format!(
                "release manifest digest or size does not match `{file_name}`"
            ));
        }
    }

    let desktop_entries = manifest["desktop_artifacts"]
        .as_array()
        .ok_or_else(|| "release manifest desktop artifacts are missing".to_owned())?;
    if desktop_entries.len() != 2 {
        return Err("release manifest must contain exactly two macOS desktop artifacts".to_owned());
    }
    let mut desktop_by_format = BTreeMap::new();
    for entry in desktop_entries {
        let format = required_string(entry, "archive_format", "desktop artifact")?;
        if desktop_by_format.insert(format, entry).is_some() {
            return Err(format!("duplicate desktop artifact format `{format}`"));
        }
    }
    let desktop_entry = desktop_by_format
        .get("zip")
        .ok_or_else(|| "release manifest portable ZIP desktop artifact is missing".to_owned())?;
    let desktop_archive_name = macos_gui_archive_name(version);
    let desktop_qualification_name = macos_gui_qualification_name(version);
    if desktop_entry["archive"] != desktop_archive_name
        || desktop_entry["archive_format"] != "zip"
        || desktop_entry["bundle"] != "CanISend.app"
        || desktop_entry["companion_manifest"] != "CanISend.app.manifest.json"
        || desktop_entry["developer_id"] != false
        || desktop_entry["distribution"] != "portable"
        || desktop_entry["notarized"] != false
        || desktop_entry["profile"] != stage.cargo_profile()
        || desktop_entry["qualification_evidence"] != desktop_qualification_name
        || desktop_entry["runner"] != "macos-15"
        || desktop_entry["signing_kind"] != "apple-adhoc"
        || desktop_entry["surface"] != "desktop-gui"
        || desktop_entry["target"] != "aarch64-apple-darwin"
    {
        return Err("release manifest macOS GUI artifact metadata is invalid".to_owned());
    }
    let desktop_sha = required_string(
        desktop_entry,
        "sha256",
        "release macOS GUI desktop artifact",
    )?;
    validate_lower_hex("release macOS GUI archive SHA-256", desktop_sha, 64)?;
    let desktop_size = desktop_entry["size"]
        .as_u64()
        .filter(|size| *size > 0)
        .ok_or_else(|| "release macOS GUI desktop artifact has no positive size".to_owned())?;
    let desktop_archive = directory.join(&desktop_archive_name);
    reject_symlink(&desktop_archive)?;
    if sha256_file(&desktop_archive)? != desktop_sha || file_size(&desktop_archive)? != desktop_size
    {
        return Err(format!(
            "release manifest digest or size does not match `{desktop_archive_name}`"
        ));
    }
    read_macos_gui_qualification(
        &directory.join(&desktop_qualification_name),
        &format!("v{version}"),
        version,
        &desktop_archive,
    )?;

    let desktop_dmg_entry = desktop_by_format
        .get("dmg")
        .ok_or_else(|| "release manifest installer DMG desktop artifact is missing".to_owned())?;
    let desktop_dmg_name = macos_gui_dmg_name(version);
    let desktop_dmg_qualification_name = macos_gui_dmg_qualification_name(version);
    if desktop_dmg_entry["applications_link"] != "/Applications"
        || desktop_dmg_entry["archive"] != desktop_dmg_name
        || desktop_dmg_entry["archive_format"] != "dmg"
        || desktop_dmg_entry["bundle"] != "CanISend.app"
        || desktop_dmg_entry["companion_manifest"] != "CanISend.app.manifest.json"
        || desktop_dmg_entry["developer_id"] != false
        || desktop_dmg_entry["distribution"] != "installer"
        || desktop_dmg_entry["notarized"] != false
        || desktop_dmg_entry["profile"] != stage.cargo_profile()
        || desktop_dmg_entry["qualification_evidence"] != desktop_dmg_qualification_name
        || desktop_dmg_entry["runner"] != "macos-15"
        || desktop_dmg_entry["signing_kind"] != "apple-adhoc"
        || desktop_dmg_entry["surface"] != "desktop-gui"
        || desktop_dmg_entry["target"] != "aarch64-apple-darwin"
    {
        return Err("release manifest macOS GUI DMG metadata is invalid".to_owned());
    }
    let desktop_dmg_sha = required_string(
        desktop_dmg_entry,
        "sha256",
        "release macOS GUI DMG artifact",
    )?;
    validate_lower_hex("release macOS GUI DMG SHA-256", desktop_dmg_sha, 64)?;
    let desktop_dmg_size = desktop_dmg_entry["size"]
        .as_u64()
        .filter(|size| *size > 0)
        .ok_or_else(|| "release macOS GUI DMG artifact has no positive size".to_owned())?;
    let desktop_dmg = directory.join(&desktop_dmg_name);
    reject_symlink(&desktop_dmg)?;
    if sha256_file(&desktop_dmg)? != desktop_dmg_sha || file_size(&desktop_dmg)? != desktop_dmg_size
    {
        return Err(format!(
            "release manifest digest or size does not match `{desktop_dmg_name}`"
        ));
    }
    read_macos_gui_dmg_qualification(
        &directory.join(&desktop_dmg_qualification_name),
        &format!("v{version}"),
        version,
        &desktop_dmg,
    )?;

    let desktop_compilation_entries = manifest["desktop_compilation"]
        .as_array()
        .ok_or_else(|| "release manifest desktop compilation records are missing".to_owned())?;
    let desktop_intel_compilation_name = macos_gui_intel_compilation_name(version);
    if validate_desktop_compilation_entries(stage, version, desktop_compilation_entries)?.is_some()
    {
        read_macos_gui_intel_compilation(
            &directory.join(&desktop_intel_compilation_name),
            &format!("v{version}"),
            version,
            commit,
        )?;
    }

    let mut expected_supplemental = BTreeSet::from([
        "ISSUE_COLLECTION.md".to_owned(),
        "KNOWN_LIMITATIONS.md".to_owned(),
        "RELEASE_NOTES.md".to_owned(),
        "THIRD_PARTY_NOTICES.md".to_owned(),
        desktop_qualification_name,
        desktop_dmg_qualification_name,
        format!("canisend-{version}-sbom.cdx.json"),
    ]);
    if stage.requires_intel_gui_release_evidence() {
        expected_supplemental.insert(desktop_intel_compilation_name);
    }
    if !matches!(stage, ReleaseStage::Alpha) {
        for target in release_targets()?
            .into_iter()
            .filter(|target| target.signing != "none")
        {
            expected_supplemental
                .insert(format!("canisend-{version}-{}-signing.json", target.triple));
        }
    }
    if matches!(stage, ReleaseStage::Stable) {
        expected_supplemental.extend(stable_channel_asset_names(version));
    }
    let supplemental = manifest["supplemental_files"]
        .as_array()
        .ok_or_else(|| "release manifest supplemental files are missing".to_owned())?;
    if supplemental.len() != expected_supplemental.len() {
        return Err("release manifest supplemental file count is invalid".to_owned());
    }
    let mut actual_supplemental = BTreeSet::new();
    for entry in supplemental {
        let file = required_string(entry, "file", "supplemental release file")?;
        if file.contains('/') || file.contains('\\') || !actual_supplemental.insert(file.to_owned())
        {
            return Err(format!(
                "unsafe or duplicate supplemental release file `{file}`"
            ));
        }
        let declared_sha = required_string(entry, "sha256", "supplemental release file")?;
        validate_lower_hex(
            &format!("supplemental release file `{file}` SHA-256"),
            declared_sha,
            64,
        )?;
        let declared_size = entry["size"]
            .as_u64()
            .filter(|size| *size > 0)
            .ok_or_else(|| format!("supplemental release file `{file}` has no positive size"))?;
        let path = directory.join(file);
        reject_symlink(&path)?;
        if sha256_file(&path)? != declared_sha || file_size(&path)? != declared_size {
            return Err(format!(
                "release manifest digest or size does not match supplemental file `{file}`"
            ));
        }
    }
    if actual_supplemental != expected_supplemental {
        return Err("release manifest supplemental file set is invalid".to_owned());
    }
    if matches!(stage, ReleaseStage::Stable) {
        verify_stable_channel_publication(directory, &format!("v{version}"), manifest)?;
    }
    Ok(())
}

fn validate_desktop_compilation_entries<'a>(
    stage: ReleaseStage,
    version: &str,
    entries: &'a [Value],
) -> Result<Option<&'a Value>, String> {
    if !stage.requires_intel_gui_release_evidence() {
        if entries.is_empty() {
            return Ok(None);
        }
        return Err(
            "Alpha release manifest must not contain Intel GUI compilation evidence".to_owned(),
        );
    }

    let [entry] = entries else {
        return Err(
            "Beta, RC, and Stable manifests must contain exactly one Intel GUI compilation record"
                .to_owned(),
        );
    };
    if !entry["archive"].is_null()
        || entry["evidence"] != macos_gui_intel_compilation_name(version)
        || entry["native_runtime_qualified"] != false
        || entry["runner"] != "macos-15-intel"
        || entry["status"] != "compile-only"
        || entry["surface"] != "desktop-gui"
        || entry["target"] != "x86_64-apple-darwin"
    {
        return Err("release manifest macOS Intel compile-only record is invalid".to_owned());
    }
    Ok(Some(entry))
}

fn write_checksums(directory: &Path) -> Result<(), String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("could not inspect release output: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("could not inspect release output entry: {error}"))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    let mut body = String::new();
    for entry in entries {
        let path = entry.path();
        if !path.is_file() || entry.file_name() == "SHA256SUMS" {
            continue;
        }
        reject_symlink(&path)?;
        body.push_str(&format!(
            "{}  {}\n",
            sha256_file(&path)?,
            entry.file_name().to_string_lossy()
        ));
    }
    fs::write(directory.join("SHA256SUMS"), body)
        .map_err(|error| format!("could not write SHA256SUMS: {error}"))
}

fn release_file_entry(path: &Path) -> Result<Value, String> {
    Ok(json!({
        "file": path
            .file_name()
            .ok_or_else(|| format!("release file has no name: {}", path.display()))?
            .to_string_lossy(),
        "sha256": sha256_file(path)?,
        "size": file_size(path)?,
    }))
}

fn find_unique_file(root: &Path, file_name: &str) -> Result<PathBuf, String> {
    let mut matches = Vec::new();
    collect_named_files(root, file_name, &mut matches)?;
    match matches.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err(format!(
            "release archive `{file_name}` was not found under {}",
            root.display()
        )),
        _ => Err(format!(
            "release archive `{file_name}` appears more than once under {}",
            root.display()
        )),
    }
}

fn collect_named_files(
    root: &Path,
    file_name: &str,
    matches: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for entry in fs::read_dir(root)
        .map_err(|error| format!("could not inspect {}: {error}", root.display()))?
    {
        let entry =
            entry.map_err(|error| format!("could not inspect release artifact: {error}"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "release artifact tree contains a symlink: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            collect_named_files(&path, file_name, matches)?;
        } else if entry.file_name() == file_name {
            matches.push(path);
        }
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "release input is not a regular file: {}",
            path.display()
        ));
    }
    Ok(())
}

fn file_size(path: &Path) -> Result<u64, String> {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|error| format!("could not inspect {}: {error}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("could not read {} for hashing: {error}", path.display()))?;
    Ok(sha256(&bytes))
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn write_pretty_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let mut file = fs::File::create(path)
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, value)
        .map_err(|error| format!("could not serialize {}: {error}", path.display()))?;
    file.write_all(b"\n")
        .map_err(|error| format!("could not finish {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_coupling_inventory_is_current_and_fails_closed_on_drift() {
        let root = repository_root();
        let inventory = build_domain_coupling_inventory(&root).expect("domain coupling inventory");
        let contract_path = root.join("docs/contracts/domain-coupling-inventory-v1.json");
        let mut contract: Value = serde_json::from_slice(
            &fs::read(contract_path).expect("domain coupling inventory contract"),
        )
        .expect("domain coupling inventory contract JSON");
        validate_domain_coupling_contract(&contract, &inventory).expect("current inventory");

        contract["expected"]["inventory_sha256"] = json!(&"0".repeat(64));
        assert!(validate_domain_coupling_contract(&contract, &inventory).is_err());
    }

    #[test]
    fn domain_coupling_classifier_has_explicit_ownership_buckets() {
        let family = |value: &str| BTreeSet::from([value.to_owned()]);
        assert_eq!(
            classify_domain_coupling(
                "crates/canisend-resources/resources/workflow-packs/org.canisend.academic-job/manifest.json",
                &family("academic-vocabulary"),
            ),
            Ok("academic-pack")
        );
        assert_eq!(
            classify_domain_coupling(
                "crates/canisend-io/src/discovery/adapters.rs",
                &family("academic-vocabulary"),
            ),
            Ok("optional-adapter")
        );
        assert_eq!(
            classify_domain_coupling(
                "crates/canisend-store/src/job.rs",
                &family("legacy-job-surface"),
            ),
            Ok("compatibility-surface")
        );
        assert_eq!(
            classify_domain_coupling(
                "crates/canisend-app/src/agent_v3.rs",
                &family("academic-vocabulary"),
            ),
            Ok("kernel")
        );
        assert!(
            classify_domain_coupling(
                "crates/new-domain/src/lib.rs",
                &family("unrecognized-coupling")
            )
            .is_err()
        );
    }

    #[test]
    fn workspace_dependency_policy_rejects_unapproved_reclassified_and_overdue_edges() {
        let root = repository_root();
        let policy: Value = serde_json::from_slice(
            &fs::read(
                root.join("docs/architecture/rust-native/workspace-dependency-policy-v1.json"),
            )
            .expect("dependency policy"),
        )
        .expect("dependency policy JSON");
        let (packages, edges) =
            current_workspace_dependency_facts(&root).expect("workspace dependency facts");
        let today = Date::from_calendar_date(2026, Month::August, 8).expect("fixture date");
        let summary = validate_workspace_dependency_policy(&policy, &packages, &edges, today)
            .expect("current policy");
        assert_eq!(summary.actual_edges, 28);
        assert_eq!(summary.target_edges, 27);

        let mut reclassified = policy.clone();
        reclassified["actual_edges"][0]["optional"] = json!(true);
        assert!(
            validate_workspace_dependency_policy(&reclassified, &packages, &edges, today).is_err()
        );

        let mut unapproved_edges = edges.clone();
        unapproved_edges.push(json!({
            "from": "canisend-core",
            "to": "canisend-resources",
            "kind": "normal",
            "target": null,
            "optional": false,
            "default_features": true,
            "features": [],
            "rename": null
        }));
        assert!(
            validate_workspace_dependency_policy(&policy, &packages, &unapproved_edges, today)
                .is_err()
        );

        let mut overdue = policy;
        overdue["temporary_exceptions"][0]["review_by"] = json!("2026-08-02");
        assert!(validate_workspace_dependency_policy(&overdue, &packages, &edges, today).is_err());
    }

    #[test]
    fn dependency_edge_schema_preserves_build_target_optional_feature_and_rename_dimensions() {
        let normalized = normalize_dependency_edge(&json!({
            "from": "canisend-gui",
            "to": "canisend-app",
            "kind": "build",
            "target": "cfg(target_os = \"windows\")",
            "optional": true,
            "default_features": false,
            "features": ["zeta", "alpha"],
            "rename": "application_facade"
        }))
        .expect("fully classified edge");
        assert_eq!(normalized["kind"], "build");
        assert_eq!(normalized["target"], "cfg(target_os = \"windows\")");
        assert_eq!(normalized["optional"], true);
        assert_eq!(normalized["default_features"], false);
        assert_eq!(normalized["features"], json!(["alpha", "zeta"]));
        assert_eq!(normalized["rename"], "application_facade");
    }

    #[test]
    fn dependency_assurance_binds_current_exceptions_and_rejects_stale_windows() {
        check_dependency_assurance().expect("current dependency assurance");
        let reviewed = Date::from_calendar_date(2026, Month::August, 3).expect("review date");
        let review_by = Date::from_calendar_date(2026, Month::August, 10).expect("review-by date");
        let expires = Date::from_calendar_date(2026, Month::August, 17).expect("expiry date");
        validate_dependency_exception_dates(
            "RUSTSEC-2026-0194",
            reviewed,
            review_by,
            expires,
            reviewed,
        )
        .expect("current exception dates");
        assert!(
            validate_dependency_exception_dates(
                "RUSTSEC-2026-0194",
                reviewed,
                review_by,
                expires,
                Date::from_calendar_date(2026, Month::August, 11).expect("overdue date"),
            )
            .is_err()
        );
        assert!(
            validate_dependency_exception_dates(
                "RUSTSEC-2026-0194",
                reviewed,
                Date::from_calendar_date(2026, Month::August, 18).expect("broad review date"),
                Date::from_calendar_date(2026, Month::August, 24).expect("broad expiry date"),
                reviewed,
            )
            .is_err()
        );
    }

    #[test]
    fn third_party_lock_fingerprint_ignores_workspace_version_only_changes() {
        let root = std::env::temp_dir().join(format!(
            "canisend-third-party-lock-fingerprint-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale lock fixture");
        }
        fs::create_dir_all(&root).expect("create lock fixture");
        let lock = |workspace_version: &str, external_version: &str, checksum: &str| {
            format!(
                "version = 4\n\n[[package]]\nname = \"canisend-core\"\nversion = \"{workspace_version}\"\n\n[[package]]\nname = \"external\"\nversion = \"{external_version}\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"{checksum}\"\n"
            )
        };
        fs::write(
            root.join("Cargo.lock"),
            lock("1.0.0-alpha.5", "1.0.0", &"a".repeat(64)),
        )
        .expect("write initial lock");
        let initial = third_party_lock_fingerprint(&root).expect("initial fingerprint");
        fs::write(
            root.join("Cargo.lock"),
            lock("1.0.0-alpha.6", "1.0.0", &"a".repeat(64)),
        )
        .expect("write workspace-only transition");
        assert_eq!(
            third_party_lock_fingerprint(&root).expect("workspace-only fingerprint"),
            initial
        );
        fs::write(
            root.join("Cargo.lock"),
            lock("1.0.0-alpha.6", "1.0.1", &"b".repeat(64)),
        )
        .expect("write external transition");
        assert_ne!(
            third_party_lock_fingerprint(&root).expect("external fingerprint"),
            initial
        );
        fs::remove_dir_all(root).expect("remove lock fixture");
    }

    fn sample_release_status_sources() -> ReleaseStatusSources {
        let version = Version::parse("1.0.0-alpha.5").expect("sample source version");
        let targets = [
            "aarch64-apple-darwin",
            "x86_64-apple-darwin",
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
            "x86_64-unknown-linux-musl",
        ];
        ReleaseStatusSources {
            workspace_version: version.clone(),
            workspace_license: "GPL-3.0-only".to_owned(),
            qualification: json!({
                "schema": RELEASE_QUALIFICATION_SCHEMA,
                "workspace_stage": "alpha",
                "status": "pre-beta",
                "stable_authorized": false,
                "feature_freeze": {"status": "planned"},
                "beta": {"status": "pending"},
                "release_candidates": []
            }),
            support: json!({
                "schema": SUPPORT_POLICY_SCHEMA,
                "release_line": "1.0",
                "publication_status": "pre-stable-draft",
                "platforms": {"target_count": targets.len()},
                "contracts": {
                    "agent_protocol": "canisend.agent/v2",
                    "public_schema_version": "2.0.0"
                },
                "workspace": {
                    "format": "canisend.workspace/v2",
                    "current_database_schema_version": 13
                }
            }),
            targets: json!({
                "schema": RELEASE_TARGET_SCHEMA,
                "targets": targets.map(|triple| json!({"triple": triple}))
            }),
            alpha_package: json!({
                "schema": "canisend.alpha-package-contract/v2",
                "version": version.to_string(),
                "tag": format!("v{version}"),
                "standalone_cli": {
                    "assets": targets.map(|target| json!({"target": target}))
                },
                "desktop_macos": {"target": "aarch64-apple-darwin"},
                "desktop_macos_intel": {
                    "target": "x86_64-apple-darwin",
                    "status": "not-published"
                }
            }),
            beta_readiness: json!({
                "schema": BETA_READINESS_SCHEMA,
                "status": "recorded",
                "alpha_release": {"tag": "v1.0.0-alpha.5"}
            }),
            beta_freeze: json!({
                "schema": BETA_CONTRACT_FREEZE_SCHEMA,
                "status": "recorded",
                "baseline": {"release": "v1.0.0-alpha.5"}
            }),
            feedback: json!({
                "schema": FEEDBACK_SNAPSHOT_SCHEMA,
                "status": "recorded",
                "release_line": "1.0",
                "expected_release": {"tag": "v1.0.0-alpha.5"},
                "next_roadmap": {"path": "docs/roadmap.md"}
            }),
            cli_gui_parity: json!({
                "format": "canisend.cli-gui-parity/v1",
                "entries": [
                    {"operation": "product.version"},
                    {"operation": "workspace.init"}
                ]
            }),
            svelte_parity: json!({
                "format": SVELTE_PARITY_SCHEMA,
                "cutover_ready": true,
                "entries": [
                    {"operation": "product.version"},
                    {"operation": "workspace.init"}
                ]
            }),
            signing: json!({
                "schema": SIGNING_POLICY_SCHEMA,
                "trust_tier": "community-build"
            }),
            git: ReleaseStatusGitFacts {
                head_commit: "1".repeat(40),
                worktree_dirty: false,
                public_tag: "v1.0.0-alpha.5".to_owned(),
                public_version: version,
                public_commit: "1".repeat(40),
                source_commits_ahead: 0,
            },
        }
    }

    #[test]
    fn release_status_derives_a_canonical_consistent_view() {
        let status = build_release_status_document(&sample_release_status_sources())
            .expect("canonical release status");
        assert_eq!(status["schema"], RELEASE_STATUS_SCHEMA);
        assert_eq!(status["authoritative"], false);
        assert_eq!(status["hard_consistent"], true);
        assert_eq!(status["support"]["cli_target_count"], 5);
        assert_eq!(status["contracts"]["operation_family_count"], 2);
        assert_eq!(status["drift"]["count"], 0);
        assert_eq!(status["drift"]["blocks_stage_transition"], false);
    }

    #[test]
    fn release_status_reports_pending_source_and_stale_stage_evidence() {
        let mut sources = sample_release_status_sources();
        sources.git.head_commit = "2".repeat(40);
        sources.git.source_commits_ahead = 14;
        sources.beta_readiness["alpha_release"]["tag"] = json!("v1.0.0-alpha.4");
        sources.beta_freeze["baseline"]["release"] = json!("v1.0.0-alpha.4");
        sources.feedback["expected_release"]["tag"] = json!("v1.0.0-alpha.1");

        let status = build_release_status_document(&sources).expect("derived drift status");
        let codes = status["drift"]["items"]
            .as_array()
            .expect("drift items")
            .iter()
            .map(|item| item["code"].as_str().expect("drift code"))
            .collect::<Vec<_>>();
        assert_eq!(
            codes,
            [
                "source-commit-ahead-of-public-checkpoint",
                "beta-readiness-not-current-public",
                "beta-freeze-not-current-public",
                "feedback-not-current-public"
            ]
        );
        assert_eq!(status["drift"]["blocking_count"], 3);
        assert_eq!(status["drift"]["blocks_stage_transition"], true);
    }

    #[test]
    fn release_status_rejects_public_checkpoint_newer_than_source() {
        let mut sources = sample_release_status_sources();
        sources.git.public_version = Version::parse("1.0.0-alpha.6").expect("newer public version");
        sources.git.public_tag = "v1.0.0-alpha.6".to_owned();
        assert!(
            build_release_status_document(&sources)
                .expect_err("future public checkpoint must fail")
                .contains("newer than source")
        );
    }

    #[test]
    fn release_status_rejects_ledger_stage_disagreement() {
        let mut sources = sample_release_status_sources();
        sources.qualification["workspace_stage"] = json!("beta");
        sources.qualification["status"] = json!("beta-qualifying");
        assert!(
            build_release_status_document(&sources)
                .expect_err("stage disagreement must fail")
                .contains("stage disagreement")
        );
    }

    #[test]
    fn release_status_rejects_platform_count_disagreement() {
        let mut sources = sample_release_status_sources();
        sources.support["platforms"]["target_count"] = json!(4);
        assert!(
            build_release_status_document(&sources)
                .expect_err("platform disagreement must fail")
                .contains("platform disagreement")
        );
    }

    fn sample_channel_source() -> ChannelCandidateSource {
        channel_candidate_source_from_value(&json!({
            "schema": CHANNEL_CANDIDATE_SOURCE_SCHEMA,
            "candidate_only": true,
            "publication_authorized": false,
            "release": {
                "tag": NATIVE_ALPHA_TAG,
                "version": "0.7.0-alpha.1",
                "stage": "alpha",
                "source_commit": NATIVE_ALPHA_SOURCE,
                "repository": env!("CARGO_PKG_REPOSITORY"),
                "manifest_file": "canisend-0.7.0-alpha.1-manifest.json",
                "manifest_sha256": "1111111111111111111111111111111111111111111111111111111111111111"
            },
            "artifacts": [
                {
                    "target": "aarch64-apple-darwin",
                    "archive": "canisend-0.7.0-alpha.1-aarch64-apple-darwin.tar.gz",
                    "sha256": "2222222222222222222222222222222222222222222222222222222222222222",
                    "size": 1
                },
                {
                    "target": "x86_64-apple-darwin",
                    "archive": "canisend-0.7.0-alpha.1-x86_64-apple-darwin.tar.gz",
                    "sha256": "3333333333333333333333333333333333333333333333333333333333333333",
                    "size": 2
                },
                {
                    "target": "x86_64-pc-windows-msvc",
                    "archive": "canisend-0.7.0-alpha.1-x86_64-pc-windows-msvc.zip",
                    "sha256": "4444444444444444444444444444444444444444444444444444444444444444",
                    "size": 3
                }
            ]
        }))
        .expect("sample channel source")
    }

    fn sample_stable_publication_ledger() -> Value {
        let first_run = 29_641_000_001_u64;
        let final_run = 29_641_000_002_u64;
        json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "stable",
            "status": "qualified",
            "stable_authorized": true,
            "beta": {
                "status": "qualified",
                "tag": "v0.7.0-beta.1",
                "source_commit": "6".repeat(40),
                "signed_matrix_run": 29_640_000_001_u64,
                "signing_evidence_targets": [
                    "aarch64-apple-darwin",
                    "x86_64-apple-darwin",
                    "x86_64-pc-windows-msvc"
                ]
            },
            "feature_freeze": {"status": "frozen", "baseline_commit": "7".repeat(40)},
            "release_candidates": [
                {
                    "tag": "v0.7.0-rc.1",
                    "status": "success",
                    "source_commit": "8".repeat(40),
                    "signed_matrix_run": first_run
                },
                {
                    "tag": "v0.7.0-rc.2",
                    "status": "success",
                    "source_commit": "9".repeat(40),
                    "signed_matrix_run": final_run
                }
            ],
            "upgrade_matrix": {"status": "passed", "evidence": ["qualified"]},
            "documentation_uninstall": {
                "status": "passed",
                "native_matrix_run": first_run,
                "evidence": ["qualified"]
            },
            "package_managers": {
                "channels": ["homebrew-cask", "scoop", "winget"],
                "evidence": [
                    format!(
                        "package-manager qualification run {first_run} passed Homebrew arm64/Intel, Scoop, and WinGet records"
                    ),
                    "v0.7.0-beta.1 to v0.7.0-rc.1 install, version, doctor, workspace, upgrade, uninstall, and retention passed"
                ],
                "qualification": {
                    "beta_tag": "v0.7.0-beta.1",
                    "rc_tag": "v0.7.0-rc.1",
                    "records": 4,
                    "run_id": first_run
                },
                "status": "passed"
            },
            "release_notes": {
                "status": "stable-final",
                "review": {
                    "evidence": [
                        "v0.7.0-rc.2 release notes and rollback guidance reviewed by reviewer",
                        format!(
                            "signed RC matrix run {final_run} manifest, public issues, assets, limitations, and package-channel state reviewed"
                        )
                    ],
                    "release_manifest_sha256": "a".repeat(64),
                    "release_notes_body_sha256": "b".repeat(64),
                    "reviewer": "reviewer",
                    "rollback_sha256": "c".repeat(64),
                    "signed_matrix_run": final_run,
                    "source_commit": "9".repeat(40),
                    "status": "reviewed",
                    "tag": "v0.7.0-rc.2"
                }
            }
        })
    }

    fn sample_package_manager_evidence(
        record: &str,
        channel: &str,
        target: &str,
        environment: &str,
    ) -> Value {
        json!({
            "schema": PACKAGE_MANAGER_QUALIFICATION_SCHEMA,
            "record": record,
            "channel": channel,
            "target": target,
            "environment": environment,
            "from_tag": "v0.7.0-beta.1",
            "to_tag": "v0.7.0-rc.1",
            "from_candidate_source_sha256": "a".repeat(64),
            "to_candidate_source_sha256": "b".repeat(64),
            "github_run_id": 29_640_000_001_u64,
            "tool_version": "native package tool 1.0.0",
            "observed_versions": {
                "from": "0.7.0-beta.1",
                "to": "0.7.0-rc.1"
            },
            "checks": {
                "candidate-sources-verified": true,
                "official-validation": true,
                "install": true,
                "from-version": true,
                "from-doctor": true,
                "workspace-created": true,
                "upgrade": true,
                "to-version": true,
                "to-doctor": true,
                "uninstall": true,
                "workspace-retained": true,
                "no-publication": true
            },
            "completed_at": "2026-07-18T10:00:00Z"
        })
    }

    fn sample_upgrade_evidence(record: &str, target: &str, environment: &str) -> Value {
        json!({
            "schema": UPGRADE_QUALIFICATION_SCHEMA,
            "record": record,
            "target": target,
            "environment": environment,
            "from_tag": "v0.7.0-beta.1",
            "to_tag": "v0.7.0-rc.1",
            "manifests": {
                "from_sha256": "a".repeat(64),
                "to_sha256": "b".repeat(64)
            },
            "archives": {
                "from_sha256": sha256(format!("beta-{target}").as_bytes()),
                "to_sha256": sha256(format!("rc-{target}").as_bytes())
            },
            "github_run_id": 29_650_000_001_u64,
            "observed_versions": {
                "from": "0.7.0-beta.1",
                "to": "0.7.0-rc.1"
            },
            "database_schemas": {"before": 13, "after": 13},
            "old_binary_behavior": "same-schema-accepted",
            "checks": {
                "verified-release-pair": true,
                "from-version-and-doctor": true,
                "workspace-created-and-checked": true,
                "verified-pre-upgrade-backup": true,
                "to-version-and-doctor": true,
                "workspace-upgraded-and-checked": true,
                "old-binary-behavior-verified": true,
                "backup-restored-to-new-path": true,
                "restored-workspace-checked-by-old-binary": true,
                "host-pack-regenerated": true,
                "installed-binary-and-notices-uninstalled": true,
                "workspace-backup-and-restore-retained": true,
                "no-publication": true
            },
            "completed_at": "2026-07-18T12:00:00Z"
        })
    }

    fn sample_documentation_uninstall_evidence(
        record: &str,
        target: &str,
        environment: &str,
        archive_sha256: &str,
    ) -> Value {
        json!({
            "schema": DOCUMENTATION_UNINSTALL_SCHEMA,
            "record": record,
            "target": target,
            "environment": environment,
            "tag": "v0.7.0-rc.1",
            "archive_sha256": archive_sha256,
            "github_run_id": 29_660_000_001_u64,
            "observed_version": "0.7.0-rc.1",
            "checks": {
                "exact-binary-match": true,
                "complete-notice-bundle": true,
                "version-and-doctor": true,
                "documented-quickstart": true,
                "agent-v4-host-smoke": true,
                "agent-v4-mcp-lifecycle-smoke": true,
                "isolated-install": true,
                "uninstall": true,
                "workspace-retained": true,
                "no-publication": true
            },
            "completed_at": "2026-07-18T13:00:00Z"
        })
    }

    fn sample_apple_signing_evidence() -> Value {
        json!({
            "schema": CODE_SIGNING_EVIDENCE_SCHEMA,
            "version": env!("CARGO_PKG_VERSION"),
            "target": "aarch64-apple-darwin",
            "kind": "apple-adhoc",
            "status": "verified",
            "binary": {
                "file": "canisend",
                "sha256": "5555555555555555555555555555555555555555555555555555555555555555",
                "size": 42
            },
            "archive": null,
            "signer": {
                "identity": "adhoc",
                "code_identifier": "io.github.jxpeng98.canisend"
            },
            "verification": {
                "codesign_valid": true,
                "adhoc": true,
                "developer_id": false,
                "hardened_runtime": true,
                "secure_timestamp": false,
                "notarized": false,
                "gatekeeper_trusted_publisher": false,
                "get_task_allow": false
            }
        })
    }

    fn sample_windows_signing_evidence() -> Value {
        json!({
            "schema": CODE_SIGNING_EVIDENCE_SCHEMA,
            "version": env!("CARGO_PKG_VERSION"),
            "target": "x86_64-pc-windows-msvc",
            "kind": "windows-authenticode-self-signed",
            "status": "verified",
            "binary": {
                "file": "canisend.exe",
                "sha256": "7777777777777777777777777777777777777777777777777777777777777777",
                "size": 84
            },
            "archive": null,
            "signer": {
                "identity": "CN=CanISend Community Build",
                "thumbprint": "8888888888888888888888888888888888888888"
            },
            "verification": {
                "authenticode_status": "NotTrusted",
                "signature_present": true,
                "self_signed": true,
                "certificate_trusted": false,
                "file_digest": "SHA256",
                "timestamp_present": false,
                "service": "powershell-self-signed-authenticode"
            }
        })
    }

    #[test]
    fn workspace_version_maps_to_exact_current_tag() {
        let version = Version::parse(env!("CARGO_PKG_VERSION")).expect("workspace version");
        let stage = ReleaseStage::from_version(&version).expect("workspace release stage");
        let tag = format!("v{version}");
        assert_eq!(validate_release_tag(&tag), Ok(stage));
        assert!(validate_release_tag(version.to_string().as_str()).is_err());
        assert!(validate_release_tag("v0.7.0-alpha.999").is_err());
        assert_eq!(
            parse_release_tag("v0.7.0-beta.1")
                .expect("historical Beta tag")
                .1,
            ReleaseStage::Beta
        );
        assert_eq!(
            parse_release_tag("v0.7.0-rc.1")
                .expect("historical RC tag")
                .1,
            ReleaseStage::ReleaseCandidate
        );
    }

    #[test]
    fn windows_msi_versions_preserve_release_order() {
        for (version, expected) in [
            ("1.0.0-alpha.5", "1.0.5"),
            ("1.0.0-beta.1", "1.0.65"),
            ("1.0.0-rc.1", "1.0.129"),
            ("1.0.0", "1.0.255"),
            ("1.0.1-alpha.1", "1.0.257"),
        ] {
            assert_eq!(
                windows_msi_product_version(&Version::parse(version).expect("valid SemVer")),
                Ok(expected.to_owned())
            );
        }
        assert!(
            windows_msi_product_version(&Version::parse("1.0.0-alpha.64").expect("valid SemVer"))
                .is_err()
        );
        assert!(
            windows_msi_product_version(&Version::parse("256.0.0").expect("valid SemVer")).is_err()
        );
    }

    #[test]
    fn desktop_version_updates_track_release_transitions() {
        let root = std::env::temp_dir().join(format!(
            "canisend-desktop-version-update-{}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("apps/canisend-desktop")).expect("create app fixture");
        fs::create_dir_all(root.join("crates/canisend-desktop")).expect("create Tauri fixture");
        fs::write(
            root.join("apps/canisend-desktop/package.json"),
            "{\n  \"version\": \"1.0.0-alpha.5\"\n}\n",
        )
        .expect("write package fixture");
        fs::write(
            root.join("crates/canisend-desktop/tauri.conf.json"),
            "{\n  \"version\": \"1.0.0-alpha.5\"\n}\n",
        )
        .expect("write Tauri fixture");
        fs::write(
            root.join("crates/canisend-desktop/tauri.windows.conf.json"),
            "{\n  \"bundle\": {\n    \"windows\": {\n      \"wix\": {\n        \"version\": \"1.0.5\"\n      }\n    }\n  }\n}\n",
        )
        .expect("write Windows fixture");

        let mut files = BTreeMap::new();
        insert_desktop_version_updates(
            &root,
            &mut files,
            &Version::parse("1.0.0-alpha.5").expect("from version"),
            &Version::parse("1.0.0-beta.1").expect("to version"),
        )
        .expect("render desktop version updates");
        assert_eq!(files.len(), 3);
        assert!(
            String::from_utf8(files["apps/canisend-desktop/package.json"].clone())
                .expect("package UTF-8")
                .contains("1.0.0-beta.1")
        );
        assert!(
            String::from_utf8(files["crates/canisend-desktop/tauri.windows.conf.json"].clone())
                .expect("Windows config UTF-8")
                .contains("\"version\": \"1.0.65\"")
        );

        fs::remove_dir_all(&root).expect("remove desktop version fixture");
    }

    #[test]
    fn intel_gui_release_evidence_starts_at_beta() {
        assert!(!ReleaseStage::Alpha.requires_intel_gui_release_evidence());
        assert!(ReleaseStage::Beta.requires_intel_gui_release_evidence());
        assert!(ReleaseStage::ReleaseCandidate.requires_intel_gui_release_evidence());
        assert!(ReleaseStage::Stable.requires_intel_gui_release_evidence());
    }

    #[test]
    fn build_profile_is_selected_from_validated_release_stage() {
        assert_eq!(ReleaseStage::Alpha.cargo_profile(), "release-alpha");
        assert_eq!(ReleaseStage::Beta.cargo_profile(), "release");
        assert_eq!(ReleaseStage::ReleaseCandidate.cargo_profile(), "release");
        assert_eq!(ReleaseStage::Stable.cargo_profile(), "release");
    }

    #[test]
    fn desktop_package_classes_separate_runtime_inclusive_artifacts() {
        assert_eq!(desktop_package_class("deb"), ("standard", false));
        assert_eq!(desktop_package_class("nsis"), ("standard", false));
        assert_eq!(desktop_package_class("nsis-offline"), ("offline", true));
        assert_eq!(desktop_package_class("appimage"), ("portable", true));
    }

    #[cfg(unix)]
    #[test]
    fn portable_payload_accepts_only_in_root_links_and_named_host() {
        use std::os::unix::fs::symlink;

        let parent =
            std::env::temp_dir().join(format!("canisend-portable-payload-{}", std::process::id()));
        let root = parent.join("squashfs-root");
        let binary = root.join("usr/bin/canisend-gui");
        let library = root.join("usr/lib/libwebkit.so");
        fs::create_dir_all(binary.parent().expect("binary parent")).expect("create binary parent");
        fs::create_dir_all(library.parent().expect("library parent"))
            .expect("create library parent");
        fs::write(&binary, b"\x7fELFcanisend").expect("write host fixture");
        fs::write(&library, b"\x7fELFruntime").expect("write runtime fixture");
        symlink("usr/bin/canisend-gui", root.join("AppRun")).expect("create in-root link");

        let root = root.canonicalize().expect("canonical payload root");
        let (_, hosts) = inspect_portable_desktop_payload(&root).expect("inspect portable payload");
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].0, "usr/bin/canisend-gui");

        let outside = parent.join("outside");
        fs::write(&outside, b"outside").expect("write outside fixture");
        symlink("../outside", root.join("escape")).expect("create escaping link");
        assert!(inspect_portable_desktop_payload(&root).is_err());

        fs::remove_dir_all(&parent).expect("remove portable payload fixture");
    }

    #[test]
    fn typst_template_contract_matches_embedded_latest_templates() {
        check_typst_template_contract().expect("latest Typst template contract");
    }

    #[test]
    fn desktop_profile_matrix_has_four_isolated_candidates() {
        assert_eq!(
            desktop_profile_configuration("release"),
            Some(("3", "thin"))
        );
        assert_eq!(
            desktop_profile_configuration("size-s-thin"),
            Some(("s", "thin"))
        );
        assert_eq!(
            desktop_profile_configuration("size-z-thin"),
            Some(("z", "thin"))
        );
        assert_eq!(
            desktop_profile_configuration("size-z-fat"),
            Some(("z", "fat"))
        );
        assert_eq!(desktop_profile_configuration("unknown"), None);
    }

    #[test]
    fn desktop_compilation_manifest_policy_is_stage_aware() {
        let version = "1.0.0-beta.1";
        let canonical = json!({
            "archive": null,
            "evidence": macos_gui_intel_compilation_name(version),
            "native_runtime_qualified": false,
            "runner": "macos-15-intel",
            "status": "compile-only",
            "surface": "desktop-gui",
            "target": "x86_64-apple-darwin"
        });

        assert!(
            validate_desktop_compilation_entries(ReleaseStage::Alpha, version, &[])
                .expect("accept empty Alpha desktop compilation records")
                .is_none()
        );
        assert!(
            validate_desktop_compilation_entries(
                ReleaseStage::Alpha,
                version,
                std::slice::from_ref(&canonical)
            )
            .is_err()
        );
        assert!(validate_desktop_compilation_entries(ReleaseStage::Beta, version, &[]).is_err());
        assert!(
            validate_desktop_compilation_entries(
                ReleaseStage::Beta,
                version,
                std::slice::from_ref(&canonical)
            )
            .expect("accept canonical Beta Intel compilation record")
            .is_some()
        );

        let mut overclaim = canonical;
        overclaim["native_runtime_qualified"] = Value::Bool(true);
        assert!(
            validate_desktop_compilation_entries(
                ReleaseStage::Stable,
                version,
                std::slice::from_ref(&overclaim)
            )
            .is_err()
        );
    }

    #[test]
    fn release_candidate_source_must_match_tagged_commit() {
        let commit = "0123456789abcdef0123456789abcdef01234567";
        let manifest = json!({
            "source": {
                "commit": commit
            }
        });
        verify_release_candidate_source(&manifest, commit).expect("matching candidate source");

        let different_commit = "89abcdef0123456789abcdef0123456789abcdef";
        let error = verify_release_candidate_source(&manifest, different_commit)
            .expect_err("mismatched candidate source");
        assert!(error.contains("does not match tagged commit"));
        assert!(verify_release_candidate_source(&manifest, "not-a-commit").is_err());
    }

    #[test]
    fn release_contract_has_five_unique_targets() {
        let targets = release_targets().expect("release targets");
        assert_eq!(targets.len(), 5);
        assert_eq!(
            targets
                .iter()
                .map(|target| target.triple.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            5
        );
    }

    #[test]
    fn native_test_ownership_runs_the_source_suite_once() {
        check_native_test_ownership().expect("native test ownership policy");
    }

    #[test]
    fn rust_toolchain_claims_match_every_active_owner() {
        check_rust_toolchain_alignment().expect("Rust toolchain alignment");
    }

    #[test]
    fn release_notes_are_stage_neutral_and_heading_only_transitions() {
        check_release_notes_policy().expect("release notes policy");
        let root = repository_root();
        let notes =
            fs::read_to_string(root.join("release/RELEASE_NOTES.md")).expect("read release notes");
        let current_heading = format!("# CanISend {}", env!("CARGO_PKG_VERSION"));
        let transitioned = replace_exact_count(
            &notes,
            &current_heading,
            "# CanISend 9.9.9-rc.999",
            1,
            "test release-note heading",
        )
        .expect("transition release-note heading");
        assert_eq!(
            notes.split_once('\n').expect("current notes body").1,
            transitioned
                .split_once('\n')
                .expect("transitioned notes body")
                .1
        );

        let body = notes
            .split_once('\n')
            .expect("release notes contain a heading")
            .1;
        let stale = format!("{current_heading}\nThe alpha release is stage-specific.\n{body}");
        let sections = [
            "Highlights",
            "Compatibility",
            "Install and verify",
            "Upgrade and rollback",
            "Security and privacy",
            "Known limitations",
            "Feedback and support",
        ];
        let guidance = [
            "does not require Python",
            "canisend.workspace/v4",
            "canisend.agent/v4",
            "never submits an application",
            "SHA256SUMS",
            "GitHub build provenance",
            "back up every important workspace",
            "restore the pre-upgrade backup into a new directory",
            "no in-place database downgrade",
            "no telemetry",
            "KNOWN_LIMITATIONS.md",
            "Never attach a workspace",
        ];
        let guides = [
            "docs/guides/release-verification.md",
            "docs/guides/quick-start.md",
            "docs/guides/upgrade-and-rollback.md",
        ];
        assert!(
            validate_release_notes(
                &root,
                &Version::parse(env!("CARGO_PKG_VERSION")).expect("workspace version"),
                &stale,
                &sections,
                &guidance,
                &guides,
            )
            .is_err()
        );
    }

    #[test]
    fn frozen_contract_digest_is_checkout_line_ending_independent() {
        let lf = canonicalize_frozen_contract_text(b"{\n  \"schema\": 2\n}\n")
            .expect("LF contract text");
        let crlf = canonicalize_frozen_contract_text(b"{\r\n  \"schema\": 2\r\n}\r\n")
            .expect("CRLF contract text");
        assert_eq!(lf, crlf);
        assert_eq!(
            digest_named_bytes(&[("schema.json".to_owned(), lf)]),
            digest_named_bytes(&[("schema.json".to_owned(), crlf)])
        );
        assert!(canonicalize_frozen_contract_text(b"invalid\rtext").is_err());
    }

    #[test]
    fn internal_path_dependencies_are_exactly_versioned() {
        check_internal_dependency_versions().expect("internal dependency versions");
    }

    #[test]
    fn beta_readiness_has_no_unresolved_alpha_blockers() {
        check_beta_readiness().expect("Beta readiness ledger");
    }

    #[test]
    fn beta_transition_accepts_only_exact_dual_pack_alpha_seven() {
        let root = std::env::temp_dir().join(format!(
            "canisend-alpha7-beta-authority-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale Alpha.7 authority fixture");
        }
        fs::create_dir_all(root.join("release")).expect("create release fixture");
        for (id, digest) in [
            ("org.canisend.academic-job", "a".repeat(64)),
            ("org.canisend.generic-application", "b".repeat(64)),
        ] {
            write_pretty_json(
                &root.join(format!(
                    "crates/canisend-resources/resources/workflow-packs/{id}/manifest.json"
                )),
                &json!({"id": id, "version": "1.0.0", "content_digest": digest}),
            )
            .expect("write embedded Pack fixture");
        }
        let source = "7".repeat(40);
        write_pretty_json(
            &root.join("release/beta-readiness.json"),
            &json!({
                "schema": BETA_READINESS_SCHEMA,
                "status": "qualified",
                "alpha_release": {
                    "tag": "v1.0.0-alpha.7",
                    "source_commit": source,
                    "release_run": 77_u64,
                    "release_url": "https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.7"
                },
                "contracts": beta_readiness_contracts(&root).expect("Beta contracts")
            }),
        )
        .expect("write readiness fixture");
        write_pretty_json(
            &root.join("release/beta-contract-freeze.json"),
            &json!({
                "baseline": {
                    "release": "v1.0.0-alpha.7",
                    "source_commit": source
                }
            }),
        )
        .expect("write freeze fixture");
        let alpha_seven = Version::parse("1.0.0-alpha.7").expect("Alpha.7 version");
        check_beta_transition_authorities(&root, &alpha_seven).expect("exact Alpha.7 authority");
        let alpha_six = Version::parse("1.0.0-alpha.6").expect("Alpha.6 version");
        assert!(check_beta_transition_authorities(&root, &alpha_six).is_err());

        let mut stale: Value = serde_json::from_slice(
            &fs::read(root.join("release/beta-readiness.json")).expect("read readiness fixture"),
        )
        .expect("parse readiness fixture");
        stale["contracts"]["workflow_packs"][1]["content_digest"] = json!(&"c".repeat(64));
        write_pretty_json(&root.join("release/beta-readiness.json"), &stale)
            .expect("write stale Pack digest");
        assert!(check_beta_transition_authorities(&root, &alpha_seven).is_err());
        fs::remove_dir_all(root).expect("remove Alpha.7 authority fixture");
    }

    #[test]
    fn beta_agent_and_workspace_contracts_match_freeze() {
        check_beta_contract_freeze().expect("Beta contract freeze");
    }

    #[test]
    fn signing_policy_matches_fail_closed_workflow_contract() {
        check_signing_policy().expect("signing policy");
    }

    #[test]
    fn active_release_runbooks_track_the_current_release_line() {
        assert_eq!(
            check_active_release_runbooks(&repository_root()).expect("active release runbooks"),
            7
        );
    }

    #[test]
    fn active_release_truth_rejects_stale_current_surfaces_and_ignores_history() {
        let root = std::env::temp_dir().join(format!(
            "canisend-active-release-truth-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale active-truth fixture");
        }
        for relative in [
            "docs/superpowers/plans",
            "docs/contracts",
            ".github/ISSUE_TEMPLATE",
            "release/history/0.7",
        ] {
            fs::create_dir_all(root.join(relative)).expect("create active-truth fixture path");
        }
        fs::write(
            root.join("docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md"),
            "# CanISend generic framework 1.0 delivery roadmap\n\n\
             **Status:** Active — authoritative\n\n\
             **Current public checkpoint:** [`v1.0.0-alpha.5`](https://example.invalid)\n\n\
             **Current machine stage:** Alpha / `pre-beta`\n\n\
             **Next intended checkpoint:** `v1.0.0-alpha.7` is the breaking v4 checkpoint.\n",
        )
        .expect("write active roadmap fixture");
        write_pretty_json(
            &root.join("docs/contracts/cli-gui-parity-v1.json"),
            &json!({"entries": [{"status": "implemented"}]}),
        )
        .expect("write parity fixture");
        let readme = "## Current status\nThe checked-in source version is `1.0.0-alpha.5`. \
            The latest publicly qualified checkpoint is `v1.0.0-alpha.5`. \
            A domain-neutral Rust kernel provides org.canisend.generic-application and \
            org.canisend.academic-job.\n";
        fs::write(root.join("README.md"), readme).expect("write README fixture");
        fs::write(
            root.join("RELEASE.md"),
            "Checked-in source: `1.0.0-alpha.5`\n\
             Latest public checkpoint: [`v1.0.0-alpha.5`]\n\
             GPL-3.0-only Community signing is not a publicly trusted publisher identity.\n\
             Verify GitHub build provenance.\n",
        )
        .expect("write release fixture");
        fs::write(
            root.join(".github/ISSUE_TEMPLATE/bug.yml"),
            "placeholder: 1.0.0-alpha.5\n",
        )
        .expect("write Issue fixture");
        fs::write(
            root.join("release/KNOWN_LIMITATIONS.md"),
            "The GUI covers all 1 declared operation families. \
             Community signatures do not establish an operating-system-trusted publisher. \
             Never disable an operating-system security control globally.\n",
        )
        .expect("write limitations fixture");
        fs::write(
            root.join("release/history/0.7/README.md"),
            "Historical 0.7.0-alpha.1 and 35 declared operation families.\n",
        )
        .expect("write historical fixture");
        let version = Version::parse("1.0.0-alpha.5").expect("fixture version");
        check_active_release_truth_for_version(&root, &version)
            .expect("historical version text must not affect active truth");

        fs::write(
            root.join("README.md"),
            readme.replace("1.0.0-alpha.5", "1.0.0-alpha.4"),
        )
        .expect("seed stale README fixture");
        assert!(check_active_release_truth_for_version(&root, &version).is_err());
        fs::remove_dir_all(root).expect("remove active-truth fixture");
    }

    #[test]
    fn feedback_roadmap_path_is_snapshot_declared_and_bounded() {
        for path in [
            "docs/superpowers/plans/2026-07-18-post-0.7-roadmap.md",
            "docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md",
        ] {
            assert_eq!(
                feedback_roadmap_relative(&json!({"next_roadmap": {"path": path}}))
                    .expect("bounded roadmap path"),
                path
            );
        }
        for path in [
            "../roadmap.md",
            "/tmp/roadmap.md",
            "docs/superpowers/plans/../../secrets.md",
            "docs/superpowers/plans/roadmap.json",
        ] {
            assert!(feedback_roadmap_relative(&json!({"next_roadmap": {"path": path}})).is_err());
        }
    }

    #[test]
    fn final_feedback_binds_latest_recorded_rc() {
        let root = std::env::temp_dir().join(format!(
            "canisend-latest-rc-feedback-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale RC feedback fixture");
        }
        fs::create_dir_all(root.join("release")).expect("create RC feedback fixture");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\n[workspace.package]\nversion = \"1.0.0-rc.2\"\n",
        )
        .expect("write RC workspace fixture");
        write_pretty_json(
            &root.join("release/qualification-ledger.json"),
            &json!({
                "release_candidates": [
                    {
                        "tag": "v1.0.0-rc.1",
                        "source_commit": "7".repeat(40),
                        "signed_matrix_run": 31_u64
                    },
                    {
                        "tag": "v1.0.0-rc.2",
                        "source_commit": "8".repeat(40),
                        "signed_matrix_run": 32_u64
                    }
                ],
                "release_notes": {"review": null}
            }),
        )
        .expect("write RC qualification fixture");
        let stale = json!({"snapshot_stage": "rc", "release": {"tag": "v1.0.0-rc.1"}});
        assert!(check_final_rc_feedback_binding(&root, &stale).is_err());
        let current = json!({"snapshot_stage": "rc", "release": {"tag": "v1.0.0-rc.2"}});
        check_final_rc_feedback_binding(&root, &current).expect("latest RC feedback");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\n[workspace.package]\nversion = \"1.0.0-rc.3\"\n",
        )
        .expect("prepare later RC workspace fixture");
        assert!(check_final_rc_feedback_binding(&root, &current).is_err());
        fs::remove_dir_all(root).expect("remove RC feedback fixture");
    }

    #[test]
    fn documentation_keeps_dual_pack_and_migration_user_paths_visible() {
        check_documentation().expect("dual-Pack user documentation");
    }

    #[test]
    fn support_policy_matches_current_contracts_and_release_line() {
        check_support_policy().expect("support policy");
    }

    #[test]
    fn support_policy_cannot_remain_draft_for_stable_version() {
        let prerelease = Version::parse("1.0.0-rc.1").expect("RC version");
        let stable = Version::parse("1.0.0").expect("Stable version");
        assert_eq!(
            support_policy_publication_status(&prerelease),
            "pre-stable-draft"
        );
        assert_eq!(support_policy_publication_status(&stable), "published");
    }

    #[test]
    fn stable_requires_rc_feedback_and_published_next_roadmap() {
        let alpha = Version::parse("0.7.0-alpha.1").expect("Alpha version");
        let prerelease = Version::parse("0.7.0-rc.1").expect("RC version");
        let stable = Version::parse("0.7.0").expect("Stable version");
        assert_eq!(
            feedback_publication_requirements(&alpha, "alpha-baseline"),
            (None, "draft")
        );
        assert_eq!(
            feedback_publication_requirements(&prerelease, "alpha-baseline"),
            (None, "draft")
        );
        assert_eq!(
            feedback_publication_requirements(&prerelease, "rc"),
            (Some("rc"), "reviewed")
        );
        assert_eq!(
            feedback_publication_requirements(&stable, "rc"),
            (Some("rc"), "published")
        );
    }

    #[test]
    fn release_stage_requires_progressive_qualification_status() {
        assert_eq!(
            qualification_status_for_stage(ReleaseStage::Alpha),
            "pre-beta"
        );
        assert_eq!(
            qualification_status_for_stage(ReleaseStage::Beta),
            "beta-qualifying"
        );
        assert_eq!(
            qualification_status_for_stage(ReleaseStage::ReleaseCandidate),
            "rc-qualifying"
        );
        assert_eq!(
            qualification_status_for_stage(ReleaseStage::Stable),
            "qualified"
        );
    }

    #[test]
    fn beta_qualification_promotion_requires_canonical_pending_state() {
        let pending = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "beta",
            "status": "beta-qualifying",
            "stable_authorized": false,
            "feature_freeze": {"status": "planned", "baseline_commit": null},
            "beta": {
                "signed_matrix_run": null,
                "signing_evidence_targets": [],
                "source_commit": null,
                "status": "pending",
                "tag": null
            }
        });
        let source = "7".repeat(40);
        let qualified = beta_qualified_ledger(&pending, "v0.7.0-beta.1", 29_640_000_001, &source)
            .expect("qualify canonical Beta ledger");
        assert_eq!(qualified["beta"]["status"], "qualified");
        assert_eq!(qualified["beta"]["tag"], "v0.7.0-beta.1");
        assert_eq!(qualified["beta"]["source_commit"], source);
        assert_eq!(
            qualified["beta"]["signing_evidence_targets"],
            json!([
                "aarch64-apple-darwin",
                "x86_64-apple-darwin",
                "x86_64-pc-windows-msvc"
            ])
        );
        assert!(
            beta_qualified_ledger(&qualified, "v0.7.0-beta.1", 29_640_000_001, &source).is_err()
        );
        assert!(beta_qualified_ledger(&pending, "v0.7.0-alpha.1", 1, &source).is_err());
        assert!(beta_qualified_ledger(&pending, "v0.7.0-beta.1", 0, &source).is_err());
    }

    #[test]
    fn rc_qualification_records_two_distinct_clean_tag_matrices() {
        let ledger = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "rc",
            "status": "rc-qualifying",
            "stable_authorized": false,
            "beta": {"status": "qualified"},
            "feature_freeze": {"status": "frozen", "baseline_commit": "6".repeat(40)},
            "release_candidates": []
        });
        let first_source = "7".repeat(40);
        let second_source = "8".repeat(40);
        let first = rc_qualified_ledger(&ledger, "v0.7.0-rc.1", 29_641_000_001, &first_source)
            .expect("record first RC matrix");
        let second = rc_qualified_ledger(&first, "v0.7.0-rc.2", 29_641_000_002, &second_source)
            .expect("record second RC matrix");
        assert_eq!(
            second["release_candidates"].as_array().map(Vec::len),
            Some(2)
        );
        assert_eq!(
            second["release_candidates"][1],
            json!({
                "signed_matrix_run": 29_641_000_002_u64,
                "source_commit": second_source,
                "status": "success",
                "tag": "v0.7.0-rc.2"
            })
        );
        assert!(
            rc_qualified_ledger(&first, "v0.7.0-rc.1", 29_641_000_002, &second_source).is_err()
        );
        assert!(
            rc_qualified_ledger(&first, "v0.7.0-rc.2", 29_641_000_001, &second_source).is_err()
        );
        assert!(rc_qualified_ledger(&first, "v0.7.0-rc.2", 29_641_000_002, &first_source).is_err());
        assert!(
            rc_qualified_ledger(&ledger, "v0.7.0-beta.1", 29_641_000_001, &first_source).is_err()
        );
    }

    #[test]
    fn release_notes_review_binds_latest_recorded_rc_and_canonical_evidence() {
        let latest_source = "8".repeat(40);
        let ledger = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "rc",
            "status": "rc-qualifying",
            "stable_authorized": false,
            "feature_freeze": {"status": "frozen", "baseline_commit": "6".repeat(40)},
            "release_notes": {"status": "rc-final", "review": null},
            "release_candidates": [
                {
                    "signed_matrix_run": 29_641_000_001_u64,
                    "source_commit": "7".repeat(40),
                    "status": "success",
                    "tag": "v0.7.0-rc.1"
                },
                {
                    "signed_matrix_run": 29_641_000_002_u64,
                    "source_commit": latest_source,
                    "status": "success",
                    "tag": "v0.7.0-rc.2"
                }
            ]
        });
        let qualified = release_notes_qualified_ledger(
            &ledger,
            "v0.7.0-rc.2",
            &latest_source,
            "reviewer",
            &"a".repeat(64),
            &"b".repeat(64),
            &"c".repeat(64),
        )
        .expect("record final RC release-notes review");
        let review = &qualified["release_notes"]["review"];
        assert_eq!(review["signed_matrix_run"], 29_641_000_002_u64);
        validate_release_notes_review_record(
            review,
            ledger["release_candidates"]
                .as_array()
                .expect("RC candidates"),
        )
        .expect("validate canonical final review");

        assert!(
            release_notes_qualified_ledger(
                &ledger,
                "v0.7.0-rc.1",
                &"7".repeat(40),
                "reviewer",
                &"a".repeat(64),
                &"b".repeat(64),
                &"c".repeat(64),
            )
            .is_err()
        );
        assert!(
            release_notes_qualified_ledger(
                &ledger,
                "v0.7.0-rc.2",
                &latest_source,
                "invalid--reviewer",
                &"a".repeat(64),
                &"b".repeat(64),
                &"c".repeat(64),
            )
            .is_err()
        );
        let mut noncanonical = review.clone();
        noncanonical["approved"] = Value::Bool(true);
        assert!(
            validate_release_notes_review_record(
                &noncanonical,
                ledger["release_candidates"]
                    .as_array()
                    .expect("RC candidates"),
            )
            .is_err()
        );
    }

    #[test]
    fn stage_transition_policy_is_forward_only_and_dry_run_by_default() {
        check_stage_transition_policy().expect("stage-transition policy");
        let alpha = Version::parse("0.7.0-alpha.1").expect("Alpha version");
        let alpha_two = Version::parse("0.7.0-alpha.2").expect("second Alpha version");
        let alpha_three = Version::parse("0.7.0-alpha.3").expect("third Alpha version");
        let beta = Version::parse("0.7.0-beta.1").expect("Beta version");
        let beta_two = Version::parse("0.7.0-beta.2").expect("second Beta version");
        let rc = Version::parse("0.7.0-rc.1").expect("RC version");
        let rc_two = Version::parse("0.7.0-rc.2").expect("second RC version");
        let rc_three = Version::parse("0.7.0-rc.3").expect("third RC version");
        validate_stage_transition(&alpha, ReleaseStage::Alpha, &beta, ReleaseStage::Beta)
            .expect("Alpha to first Beta");
        validate_stage_transition(&alpha, ReleaseStage::Alpha, &alpha_two, ReleaseStage::Alpha)
            .expect("sequential Alpha iteration");
        assert!(
            validate_stage_transition(
                &alpha,
                ReleaseStage::Alpha,
                &alpha_three,
                ReleaseStage::Alpha
            )
            .is_err()
        );
        assert!(
            validate_stage_transition(&alpha, ReleaseStage::Alpha, &beta_two, ReleaseStage::Beta)
                .is_err()
        );
        assert!(
            validate_stage_transition(
                &alpha,
                ReleaseStage::Alpha,
                &rc,
                ReleaseStage::ReleaseCandidate
            )
            .is_err()
        );
        validate_stage_transition(
            &rc,
            ReleaseStage::ReleaseCandidate,
            &rc_two,
            ReleaseStage::ReleaseCandidate,
        )
        .expect("sequential RC iteration");
        assert!(
            validate_stage_transition(
                &rc,
                ReleaseStage::ReleaseCandidate,
                &rc_three,
                ReleaseStage::ReleaseCandidate
            )
            .is_err()
        );
        assert!(
            validate_stage_transition(&beta, ReleaseStage::Beta, &beta_two, ReleaseStage::Beta)
                .is_err()
        );

        let current_rc = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "rc",
            "status": "rc-qualifying",
            "stable_authorized": false,
            "beta": {"status": "qualified"},
            "feature_freeze": {"status": "frozen"},
            "release_candidates": [{
                "tag": "v0.7.0-rc.1",
                "source_commit": "7".repeat(40),
                "signed_matrix_run": 1_u64
            }]
        });
        assert!(
            validate_transition_ledger_preconditions(
                &current_rc,
                &rc_two,
                ReleaseStage::ReleaseCandidate,
                ReleaseStage::ReleaseCandidate,
            )
            .is_err(),
            "RC.2 cannot be skipped before its exact matrix is recorded"
        );
    }

    #[test]
    fn alpha_package_contract_v3_binds_dual_pack_and_migration_authorities() {
        let root = repository_root();
        let version = Version::parse(env!("CARGO_PKG_VERSION")).expect("source version");
        let contract: Value = serde_json::from_slice(
            &fs::read(root.join("release/alpha-package-contract.json"))
                .expect("read Alpha package contract"),
        )
        .expect("parse Alpha package contract");
        check_alpha_package_contract_identity_and_bindings(&root, &version, &contract)
            .expect("current Alpha package bindings");

        for (name, mutated) in [
            ("schema", {
                let mut value = contract.clone();
                value["schema"] = Value::String(ALPHA_PACKAGE_CONTRACT_V2_SCHEMA.to_owned());
                value
            }),
            ("generic Pack digest", {
                let mut value = contract.clone();
                value["contracts"]["workflow_packs"][1]["content_digest"] =
                    Value::String("0".repeat(64));
                value
            }),
            ("resource manifest digest", {
                let mut value = contract.clone();
                value["contracts"]["resource_manifest"]["sha256"] = Value::String("0".repeat(64));
                value
            }),
            ("operation registry digest", {
                let mut value = contract.clone();
                value["contracts"]["operation_registry"]["sha256"] = Value::String("0".repeat(64));
                value
            }),
            ("migration tree digest", {
                let mut value = contract.clone();
                value["contracts"]["migration_inventory"]["tree_sha256"] =
                    Value::String("0".repeat(64));
                value
            }),
        ] {
            assert!(
                check_alpha_package_contract_identity_and_bindings(&root, &version, &mutated)
                    .is_err(),
                "mutated {name} must fail closed"
            );
        }
    }

    #[test]
    fn alpha_package_contract_schema_changes_at_alpha6() {
        let alpha5 = Version::parse("1.0.0-alpha.5").expect("Alpha.5");
        let alpha6 = Version::parse("1.0.0-alpha.6").expect("Alpha.6");
        let alpha7 = Version::parse("1.0.0-alpha.7").expect("Alpha.7");
        assert_eq!(
            alpha_package_contract_schema(&alpha5).expect("Alpha.5 schema"),
            ALPHA_PACKAGE_CONTRACT_V2_SCHEMA
        );
        assert_eq!(
            alpha_package_contract_schema(&alpha6).expect("Alpha.6 schema"),
            ALPHA_PACKAGE_CONTRACT_V3_SCHEMA
        );
        assert_eq!(
            alpha_package_contract_schema(&alpha7).expect("Alpha.7 schema"),
            ALPHA_PACKAGE_CONTRACT_V3_SCHEMA
        );
    }

    #[test]
    fn sequential_alpha_iteration_updates_candidate_authorities_and_invalidates_baselines() {
        let root =
            std::env::temp_dir().join(format!("canisend-sequential-alpha-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale sequential-Alpha fixture");
        }
        for relative in [
            "tools/native-preview",
            "apps/canisend-desktop/src",
            "docs/contracts",
            "docs/guides",
            "docs/performance",
            ".github/workflows",
            ".github/ISSUE_TEMPLATE",
            "release",
        ] {
            fs::create_dir_all(root.join(relative)).expect("create sequential-Alpha fixture path");
        }
        fs::write(
            root.join("tools/native-preview/package.json"),
            "{\n  \"version\": \"1.0.0-alpha.5\"\n}\n",
        )
        .expect("write native-preview package fixture");
        fs::write(
            root.join("apps/canisend-desktop/src/App.svelte"),
            "<span>{product?.version ?? \"1.0.0-alpha.5\"}</span>\n",
        )
        .expect("write desktop fallback fixture");
        fs::write(
            root.join("docs/contracts/cli-gui-parity-v1.json"),
            "{\n  \"version\": \"1.0.0-alpha.5\"\n}\n",
        )
        .expect("write parity fixture");
        fs::write(
            root.join("docs/performance/macos-gui-alpha-baseline.json"),
            "{\n  \"version\": \"1.0.0-alpha.5\"\n}\n",
        )
        .expect("write performance baseline fixture");
        fs::write(
            root.join(".github/workflows/release.yml"),
            "default: \"v1.0.0-alpha.6\"\n",
        )
        .expect("write workflow fixture");
        fs::write(
            root.join(".github/ISSUE_TEMPLATE/bug.yml"),
            "placeholder: 1.0.0-alpha.5\n",
        )
        .expect("write Issue template fixture");
        fs::write(
            root.join("README.md"),
            "The checked-in source version is `1.0.0-alpha.5`; fixture.\n",
        )
        .expect("write README fixture");
        fs::write(
            root.join("RELEASE.md"),
            "Checked-in source: `1.0.0-alpha.5`; fixture.\n",
        )
        .expect("write root release guide fixture");
        fs::write(
            root.join("docs/guides/known-limitations.md"),
            "It applies to the `1.0.0-alpha.5` development line. The source version still says `1.0.0-alpha.5`.\n",
        )
        .expect("write known-limitations fixture");
        write_pretty_json(
            &root.join("release/alpha-package-contract.json"),
            &json!({
                "schema": "canisend.alpha-package-contract/v2",
                "version": "1.0.0-alpha.5",
                "tag": "v1.0.0-alpha.5",
                "standalone_cli": {
                    "assets": [{"file": "canisend-1.0.0-alpha.5-target.tar.gz"}]
                },
                "desktop_macos": {
                    "archive": "CanISend-1.0.0-alpha.5-target.zip"
                }
            }),
        )
        .expect("write Alpha package contract fixture");

        let from = Version::parse("1.0.0-alpha.5").expect("source Alpha");
        let to = Version::parse("1.0.0-alpha.6").expect("target Alpha");
        let mut files = BTreeMap::new();
        insert_sequential_alpha_updates(&root, &mut files, &from, &to)
            .expect("render sequential Alpha updates");
        assert_eq!(files.len(), 12);
        for relative in [
            "tools/native-preview/package.json",
            "apps/canisend-desktop/src/App.svelte",
            "docs/contracts/cli-gui-parity-v1.json",
            "docs/performance/macos-gui-alpha-baseline.json",
            ".github/ISSUE_TEMPLATE/bug.yml",
            "README.md",
            "RELEASE.md",
            "docs/guides/known-limitations.md",
            "release/alpha-package-contract.json",
        ] {
            let body = std::str::from_utf8(&files[relative]).expect("UTF-8 Alpha update");
            assert!(
                body.contains("1.0.0-alpha.6"),
                "missing target in {relative}"
            );
            assert!(
                !body.contains("1.0.0-alpha.5"),
                "retained source in {relative}"
            );
        }
        let readiness: Value = serde_json::from_slice(&files["release/beta-readiness.json"])
            .expect("parse pending readiness");
        let freeze: Value = serde_json::from_slice(&files["release/beta-contract-freeze.json"])
            .expect("parse pending freeze");
        let feedback: Value = serde_json::from_slice(&files["release/feedback-snapshot.json"])
            .expect("parse pending feedback");
        assert_eq!(readiness["status"], "pending-alpha-publication");
        assert_eq!(readiness["alpha_release"]["tag"], "v1.0.0-alpha.6");
        assert_eq!(freeze["baseline"]["release"], "v1.0.0-alpha.6");
        assert_eq!(feedback["expected_release"]["tag"], "v1.0.0-alpha.6");
        let initial =
            pending_release_feedback(&Version::parse("1.0.0-alpha.1").expect("initial Alpha"))
                .expect("initial feedback");
        assert!(
            pending_release_feedback_is_canonical(&initial, &from)
                .expect("legacy pre-sequential feedback boundary")
        );
        assert!(
            !pending_release_feedback_is_canonical(&initial, &to)
                .expect("sequential feedback boundary")
        );
        assert!(
            pending_release_feedback_is_canonical(&feedback, &to)
                .expect("target feedback boundary")
        );
        fs::remove_dir_all(root).expect("remove sequential-Alpha fixture");
    }

    #[test]
    fn release_line_target_requires_later_first_alpha() {
        let from = Version::parse("0.7.0-rc.2").expect("source version");
        let target = Version::parse("1.0.0-alpha.1").expect("target version");
        validate_release_line_target(&from, &target, ReleaseStage::Alpha)
            .expect("first Alpha on later line");
        for invalid in [
            "0.7.0-alpha.1",
            "0.8.1-alpha.1",
            "1.0.0-alpha.2",
            "1.0.0-beta.1",
            "1.0.0-alpha.1+local",
        ] {
            let version = Version::parse(invalid).expect("invalid-case SemVer");
            let stage = ReleaseStage::from_version(&version).expect("recognized stage");
            assert!(
                validate_release_line_target(&from, &version, stage).is_err(),
                "accepted invalid release-line target {invalid}"
            );
        }
    }

    #[test]
    fn controlled_write_rolls_back_replaced_and_new_files() {
        let root =
            std::env::temp_dir().join(format!("canisend-controlled-write-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale controlled-write fixture");
        }
        fs::create_dir_all(&root).expect("create controlled-write fixture");
        fs::write(root.join("a.txt"), b"original\n").expect("write original fixture");
        let files = BTreeMap::from([
            ("a.txt".to_owned(), b"replacement\n".to_vec()),
            ("nested/b.txt".to_owned(), b"new\n".to_vec()),
        ]);
        let error = write_controlled_files_transactionally(&root, &files, Some(1))
            .expect_err("inject replacement failure");
        assert!(error.contains("rolled back"));
        assert_eq!(
            fs::read(root.join("a.txt")).expect("read rolled-back original"),
            b"original\n"
        );
        assert!(!root.join("nested/b.txt").exists());
        assert!(
            fs::read_dir(&root)
                .expect("inspect controlled-write fixture")
                .filter_map(Result::ok)
                .all(|entry| !entry
                    .file_name()
                    .to_string_lossy()
                    .contains("canisend-transaction"))
        );
        fs::remove_dir_all(root).expect("remove controlled-write fixture");
    }

    #[test]
    fn clean_worktree_gate_rejects_untracked_activation_input() {
        let root = std::env::temp_dir().join(format!("canisend-clean-gate-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale clean-gate fixture");
        }
        fs::create_dir_all(&root).expect("create clean-gate fixture");
        let status = Command::new("git")
            .current_dir(&root)
            .args(["init", "--quiet"])
            .status()
            .expect("run git init");
        assert!(status.success());
        require_clean_worktree(&root, "fixture activation").expect("new repository is clean");
        fs::write(root.join("untracked.txt"), b"dirty\n").expect("write untracked fixture");
        assert!(require_clean_worktree(&root, "fixture activation").is_err());
        fs::remove_dir_all(root).expect("remove clean-gate fixture");
    }

    #[test]
    fn release_line_render_archives_hashes_and_resets_active_state() {
        let root =
            std::env::temp_dir().join(format!("canisend-line-render-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale line-render fixture");
        }
        for directory in [
            "crates/app",
            "crates/contracts",
            "release",
            "packaging/candidates/v0.7.0-alpha.1",
        ] {
            fs::create_dir_all(root.join(directory)).expect("create line-render fixture directory");
        }
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\", \"crates/contracts\"]\n\
             [workspace.package]\nversion = \"0.7.0-rc.2\"\n",
        )
        .expect("write line-render workspace");
        fs::write(
            root.join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion.workspace = true\n\
             [dependencies]\ncontracts = { path = \"../contracts\", version = \"=0.7.0-rc.2\" }\n",
        )
        .expect("write line-render app");
        fs::write(
            root.join("crates/contracts/Cargo.toml"),
            "[package]\nname = \"contracts\"\nversion.workspace = true\n",
        )
        .expect("write line-render contracts");
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.7.0-rc.2\"\n\
             \n[[package]]\nname = \"contracts\"\nversion = \"0.7.0-rc.2\"\n",
        )
        .expect("write line-render lockfile");
        let historical_sources = [
            "release/RELEASE_NOTES.md",
            "release/beta-contract-freeze.json",
            "release/beta-readiness.json",
            "release/feature-freeze-exceptions.json",
            "release/feedback-snapshot.json",
            "release/qualification-ledger.json",
            "release/support-policy.json",
        ];
        for (index, relative) in historical_sources.iter().enumerate() {
            fs::write(root.join(relative), format!("historical-{index}\n"))
                .expect("write historical line-render source");
        }
        fs::write(
            root.join("packaging/candidates/v0.7.0-alpha.1/candidate-source.json"),
            b"retained candidate\n",
        )
        .expect("write retained candidate fixture");
        let source_commit = "a".repeat(40);
        let activation = render_release_line_activation(&root, "v1.0.0-alpha.1", &source_commit)
            .expect("render release-line activation");
        assert_eq!(activation.from_version.to_string(), "0.7.0-rc.2");
        assert_eq!(activation.to_version.to_string(), "1.0.0-alpha.1");
        assert!(
            String::from_utf8(
                activation
                    .files
                    .get("Cargo.toml")
                    .expect("rendered workspace")
                    .clone()
            )
            .expect("rendered workspace UTF-8")
            .contains("version = \"1.0.0-alpha.1\"")
        );
        let manifest: Value = serde_json::from_slice(
            activation
                .files
                .get("release/history/0.7/manifest.json")
                .expect("history manifest"),
        )
        .expect("parse history manifest");
        assert_eq!(
            manifest["files"].as_array().expect("history files").len(),
            7
        );
        for entry in manifest["files"].as_array().expect("history files") {
            let archive = entry["archive_path"].as_str().expect("archive path");
            assert_eq!(
                entry["sha256"],
                sha256(activation.files.get(archive).expect("archived bytes"))
            );
        }
        let active: Value = serde_json::from_slice(
            activation
                .files
                .get("release/qualification-ledger.json")
                .expect("active qualification"),
        )
        .expect("parse active qualification");
        assert_eq!(active["workspace_stage"], "alpha");
        assert_eq!(active["beta"]["status"], "pending");
        assert_eq!(active["release_candidates"], json!([]));
        fs::remove_dir_all(root).expect("remove line-render fixture");
    }

    #[test]
    fn beta_transition_requires_a_recent_nonfuture_readiness_audit() {
        let root = std::env::temp_dir().join(format!(
            "canisend-beta-readiness-age-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale readiness-age fixture");
        }
        fs::create_dir_all(root.join("release")).expect("create readiness-age fixture");
        let now = OffsetDateTime::parse("2026-07-18T12:00:00Z", &Rfc3339).expect("fixture time");
        for (audited_at, accepted) in [
            ("2026-07-17T13:00:00Z", true),
            ("2026-07-17T11:00:00Z", false),
            ("2026-07-18T12:06:00Z", false),
        ] {
            write_pretty_json(
                &root.join("release/beta-readiness.json"),
                &json!({"audited_at": audited_at}),
            )
            .expect("write readiness-age fixture");
            assert_eq!(
                check_beta_readiness_freshness(&root, now).is_ok(),
                accepted,
                "unexpected freshness result for {audited_at}"
            );
        }
        fs::remove_dir_all(root).expect("remove readiness-age fixture");
    }

    #[test]
    fn alpha_baseline_accepts_current_or_previous_alpha_and_survives_later_stages() {
        let alpha = Version::parse("1.0.0-alpha.4").expect("Alpha version");
        let beta = Version::parse("1.0.0-beta.1").expect("Beta version");
        assert_eq!(
            validate_alpha_baseline_tag(&alpha, "v1.0.0-alpha.4").expect("current Alpha baseline"),
            alpha
        );
        assert_eq!(
            validate_alpha_baseline_tag(&alpha, "v1.0.0-alpha.3")
                .expect("previous public Alpha during sequential candidate work")
                .to_string(),
            "1.0.0-alpha.3"
        );
        assert!(
            validate_alpha_baseline_tag(&alpha, "v1.0.0-alpha.2").is_err(),
            "a stale Alpha must not replace the latest public baseline"
        );
        assert!(
            validate_alpha_baseline_tag(&alpha, "v1.0.0-alpha.5").is_err(),
            "a future Alpha must not qualify the current source"
        );
        assert!(
            validate_alpha_baseline_tag(&alpha, "v1.1.0-alpha.4").is_err(),
            "another release line must not qualify this source"
        );
        assert!(
            validate_alpha_baseline_tag(&alpha, "v1.0.0-beta.1").is_err(),
            "the baseline itself must be an Alpha"
        );
        assert_eq!(
            validate_alpha_baseline_tag(&beta, "v1.0.0-alpha.4")
                .expect("preserved Alpha baseline after transition")
                .to_string(),
            "1.0.0-alpha.4"
        );
    }

    #[test]
    fn stage_transition_changes_only_controlled_current_state() {
        let root =
            std::env::temp_dir().join(format!("canisend-stage-transition-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale transition fixture");
        }
        fs::create_dir_all(root.join("crates/app")).expect("create app fixture");
        fs::create_dir_all(root.join("crates/contracts")).expect("create contracts fixture");
        fs::create_dir_all(root.join("release")).expect("create release fixture");
        fs::create_dir_all(root.join("packaging/candidates/alpha"))
            .expect("create historical candidate fixture");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\", \"crates/contracts\"]\n\
             [workspace.package]\nversion = \"0.7.0-beta.1\"\n",
        )
        .expect("write workspace fixture");
        fs::write(
            root.join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion.workspace = true\n\
             [dependencies]\ncontracts = { package = \"contracts\", path = \"../contracts\", version = \"=0.7.0-beta.1\" }\n",
        )
        .expect("write app fixture");
        fs::write(
            root.join("crates/contracts/Cargo.toml"),
            "[package]\nname = \"contracts\"\nversion.workspace = true\n",
        )
        .expect("write contracts fixture");
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.7.0-beta.1\"\n\
             dependencies = [\"contracts\"]\n\n[[package]]\nname = \"contracts\"\nversion = \"0.7.0-beta.1\"\n",
        )
        .expect("write lock fixture");
        write_pretty_json(
            &root.join("release/qualification-ledger.json"),
            &json!({
                "schema": RELEASE_QUALIFICATION_SCHEMA,
                "workspace_stage": "beta",
                "status": "beta-qualifying",
                "stable_authorized": false,
                "beta": {"status": "qualified"},
                "feature_freeze": {"status": "frozen", "baseline_commit": "7".repeat(40)},
                "release_notes": {"status": "beta-current"}
            }),
        )
        .expect("write qualification fixture");
        fs::write(
            root.join("release/RELEASE_NOTES.md"),
            "# CanISend 0.7.0-beta.1\n\nFixture notes.\n",
        )
        .expect("write notes fixture");
        for relative in [
            "release/beta-readiness.json",
            "release/beta-contract-freeze.json",
            "release/feedback-snapshot.json",
            "packaging/candidates/alpha/candidate-source.json",
        ] {
            fs::write(root.join(relative), b"historical 0.7.0-alpha.1\n")
                .expect("write historical fixture");
        }

        let workspace_before = fs::read(root.join("Cargo.toml")).expect("read workspace before");
        let transition =
            render_stage_transition(&root, "v0.7.0-rc.1").expect("render Beta to RC transition");
        assert_eq!(
            stage_transition_report(&root, &transition, false).expect("dry-run report")["writes_performed"],
            false
        );
        assert_eq!(
            fs::read(root.join("Cargo.toml")).expect("read workspace after dry run"),
            workspace_before
        );
        assert_eq!(transition.files.len(), 5);
        for (relative, body) in &transition.files {
            fs::write(root.join(relative), body).expect("apply rendered transition fixture");
        }
        assert!(
            fs::read_to_string(root.join("Cargo.toml"))
                .expect("read transitioned workspace")
                .contains("version = \"0.7.0-rc.1\"")
        );
        assert!(
            fs::read_to_string(root.join("crates/app/Cargo.toml"))
                .expect("read transitioned app")
                .contains("version = \"=0.7.0-rc.1\"")
        );
        let ledger: Value = serde_json::from_slice(
            &fs::read(root.join("release/qualification-ledger.json"))
                .expect("read transitioned ledger"),
        )
        .expect("parse transitioned ledger");
        assert_eq!(ledger["workspace_stage"], "rc");
        assert_eq!(ledger["status"], "rc-qualifying");
        assert_eq!(ledger["release_notes"]["status"], "rc-final");
        for relative in [
            "release/beta-readiness.json",
            "release/beta-contract-freeze.json",
            "release/feedback-snapshot.json",
            "packaging/candidates/alpha/candidate-source.json",
        ] {
            assert_eq!(
                fs::read_to_string(root.join(relative)).expect("read historical fixture"),
                "historical 0.7.0-alpha.1\n"
            );
        }
        fs::remove_dir_all(root).expect("remove transition fixture");
    }

    #[test]
    fn rc_iteration_preserves_existing_qualification_evidence() {
        let root =
            std::env::temp_dir().join(format!("canisend-rc-iteration-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale RC fixture");
        }
        fs::create_dir_all(root.join("crates/app")).expect("create RC app fixture");
        fs::create_dir_all(root.join("crates/contracts")).expect("create RC contracts fixture");
        fs::create_dir_all(root.join("fuzz")).expect("create RC fuzz fixture");
        fs::create_dir_all(root.join("release")).expect("create RC release fixture");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\", \"crates/contracts\"]\n\
             [workspace.package]\nversion = \"0.7.0-rc.1\"\n",
        )
        .expect("write RC workspace fixture");
        fs::write(
            root.join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion.workspace = true\n\
             [dependencies]\ncontracts = { package = \"contracts\", path = \"../contracts\", version = \"=0.7.0-rc.1\" }\n",
        )
        .expect("write RC app fixture");
        fs::write(
            root.join("crates/contracts/Cargo.toml"),
            "[package]\nname = \"contracts\"\nversion.workspace = true\n",
        )
        .expect("write RC contracts fixture");
        fs::write(
            root.join("fuzz/Cargo.toml"),
            "[package]\nname = \"canisend-fuzz\"\nversion = \"0.0.0\"\n\
             [dependencies]\napp = { path = \"../crates/app\", version = \"=0.7.0-rc.1\" }\n\
             contracts = { path = \"../crates/contracts\", version = \"=0.7.0-rc.1\" }\n",
        )
        .expect("write RC fuzz fixture");
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.7.0-rc.1\"\n\
             dependencies = [\"contracts\"]\n\n[[package]]\nname = \"contracts\"\nversion = \"0.7.0-rc.1\"\n",
        )
        .expect("write RC lock fixture");
        fs::write(
            root.join("fuzz/Cargo.lock"),
            "version = 4\n\n[[package]]\nname = \"canisend-app\"\nversion = \"0.7.0-rc.1\"\n\
             \n[[package]]\nname = \"canisend-fuzz\"\nversion = \"0.0.0\"\n\
             \n[[package]]\nname = \"canisend-contracts\"\nversion = \"0.7.0-rc.1\"\n",
        )
        .expect("write RC fuzz lock fixture");
        let ledger = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "rc",
            "status": "rc-qualifying",
            "stable_authorized": false,
            "beta": {"status": "qualified", "tag": "v0.7.0-beta.1"},
            "feature_freeze": {"status": "frozen", "baseline_commit": "7".repeat(40)},
            "release_notes": {
                "status": "rc-final",
                "review": {
                    "status": "reviewed",
                    "tag": "v0.7.0-rc.1"
                }
            },
            "release_candidates": [{
                "tag": "v0.7.0-rc.1",
                "status": "success",
                "source_commit": "8".repeat(40),
                "signed_matrix_run": 29_641_000_001_u64
            }]
        });
        write_pretty_json(&root.join("release/qualification-ledger.json"), &ledger)
            .expect("write RC ledger fixture");
        fs::write(
            root.join("release/RELEASE_NOTES.md"),
            "# CanISend 0.7.0-rc.1\n\nFixture notes.\n",
        )
        .expect("write RC notes fixture");

        let mut expected_ledger = ledger.clone();
        expected_ledger["release_notes"]["review"] = Value::Null;
        let transition =
            render_stage_transition(&root, "v0.7.0-rc.2").expect("render sequential RC iteration");
        assert_eq!(transition.files.len(), 7);
        for (relative, body) in &transition.files {
            fs::write(root.join(relative), body).expect("apply RC iteration fixture");
        }
        assert_eq!(
            serde_json::from_slice::<Value>(
                &fs::read(root.join("release/qualification-ledger.json"))
                    .expect("read RC ledger after iteration")
            )
            .expect("parse RC ledger after iteration"),
            expected_ledger
        );
        assert!(
            fs::read_to_string(root.join("Cargo.toml"))
                .expect("read iterated RC workspace")
                .contains("version = \"0.7.0-rc.2\"")
        );
        assert_eq!(
            fs::read_to_string(root.join("fuzz/Cargo.toml"))
                .expect("read iterated RC fuzz manifest")
                .matches("version = \"=0.7.0-rc.2\"")
                .count(),
            2
        );
        assert_eq!(
            fs::read_to_string(root.join("fuzz/Cargo.lock"))
                .expect("read iterated RC fuzz lock")
                .matches("version = \"0.7.0-rc.2\"")
                .count(),
            2
        );
        fs::remove_dir_all(root).expect("remove RC fixture");
    }

    #[test]
    fn stable_transition_publishes_reviewed_feedback_without_remeasuring_it() {
        let root = std::env::temp_dir().join(format!(
            "canisend-stable-feedback-transition-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale Stable fixture");
        }
        fs::create_dir_all(root.join("crates/app")).expect("create Stable app fixture");
        fs::create_dir_all(root.join("crates/contracts")).expect("create Stable contracts fixture");
        fs::create_dir_all(root.join("release")).expect("create Stable release fixture");
        fs::create_dir_all(root.join("docs/superpowers/plans"))
            .expect("create Stable roadmap fixture");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\", \"crates/contracts\"]\n\
             [workspace.package]\nversion = \"0.7.0-rc.2\"\n",
        )
        .expect("write Stable workspace fixture");
        fs::write(
            root.join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion.workspace = true\n\
             [dependencies]\ncontracts = { package = \"contracts\", path = \"../contracts\", version = \"=0.7.0-rc.2\" }\n",
        )
        .expect("write Stable app fixture");
        fs::write(
            root.join("crates/contracts/Cargo.toml"),
            "[package]\nname = \"contracts\"\nversion.workspace = true\n",
        )
        .expect("write Stable contracts fixture");
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.7.0-rc.2\"\n\
             dependencies = [\"contracts\"]\n\n[[package]]\nname = \"contracts\"\nversion = \"0.7.0-rc.2\"\n",
        )
        .expect("write Stable lock fixture");
        let first_rc_run = 29_641_000_001_u64;
        write_pretty_json(
            &root.join("release/qualification-ledger.json"),
            &json!({
                "schema": RELEASE_QUALIFICATION_SCHEMA,
                "workspace_stage": "rc",
                "status": "rc-qualifying",
                "stable_authorized": false,
                "beta": {
                    "status": "qualified",
                    "tag": "v0.7.0-beta.1",
                    "source_commit": "6".repeat(40),
                    "signed_matrix_run": 29_640_000_001_u64,
                    "signing_evidence_targets": [
                        "aarch64-apple-darwin",
                        "x86_64-apple-darwin",
                        "x86_64-pc-windows-msvc"
                    ]
                },
                "feature_freeze": {"status": "frozen", "baseline_commit": "7".repeat(40)},
                "release_candidates": [
                    {
                        "tag": "v0.7.0-rc.1",
                        "status": "success",
                        "source_commit": "8".repeat(40),
                        "signed_matrix_run": first_rc_run
                    },
                    {
                        "tag": "v0.7.0-rc.2",
                        "status": "success",
                        "source_commit": "9".repeat(40),
                        "signed_matrix_run": 29_641_000_002_u64
                    }
                ],
                "upgrade_matrix": {"status": "passed", "evidence": ["qualified"]},
                "documentation_uninstall": {
                    "status": "passed",
                    "native_matrix_run": first_rc_run,
                    "evidence": ["qualified"]
                },
                "package_managers": {
                    "channels": ["homebrew-cask", "scoop", "winget"],
                    "evidence": [
                        format!(
                            "package-manager qualification run {} passed Homebrew arm64/Intel, Scoop, and WinGet records",
                            first_rc_run
                        ),
                        "v0.7.0-beta.1 to v0.7.0-rc.1 install, version, doctor, workspace, upgrade, uninstall, and retention passed"
                    ],
                    "qualification": {
                        "beta_tag": "v0.7.0-beta.1",
                        "rc_tag": "v0.7.0-rc.1",
                        "records": 4,
                        "run_id": first_rc_run
                    },
                    "status": "passed"
                },
                "release_notes": {
                    "status": "rc-final",
                    "review": {
                        "evidence": [
                            "v0.7.0-rc.2 release notes and rollback guidance reviewed by reviewer",
                            format!(
                                "signed RC matrix run {} manifest, public issues, assets, limitations, and package-channel state reviewed",
                                29_641_000_002_u64
                            )
                        ],
                        "release_manifest_sha256": "a".repeat(64),
                        "release_notes_body_sha256": "b".repeat(64),
                        "reviewer": "reviewer",
                        "rollback_sha256": "c".repeat(64),
                        "signed_matrix_run": 29_641_000_002_u64,
                        "source_commit": "9".repeat(40),
                        "status": "reviewed",
                        "tag": "v0.7.0-rc.2"
                    }
                }
            }),
        )
        .expect("write Stable qualification fixture");
        fs::write(
            root.join("release/RELEASE_NOTES.md"),
            "# CanISend 0.7.0-rc.2\n\nFixture notes.\n",
        )
        .expect("write Stable notes fixture");
        let feedback = json!({
            "schema": FEEDBACK_SNAPSHOT_SCHEMA,
            "snapshot_stage": "rc",
            "release": {"tag": "v0.7.0-rc.2"},
            "public_issues": {"open": 2, "closed": 3, "total": 5},
            "release_downloads": {"total": 17},
            "next_roadmap": {
                "path": "docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md",
                "status": "reviewed"
            }
        });
        write_pretty_json(&root.join("release/feedback-snapshot.json"), &feedback)
            .expect("write Stable feedback fixture");
        let feedback_before = fs::read_to_string(root.join("release/feedback-snapshot.json"))
            .expect("read Stable feedback fixture");
        fs::write(
            root.join("docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md"),
            "# Next roadmap\n\n**Status:** Reviewed\n",
        )
        .expect("write Stable roadmap fixture");
        write_pretty_json(
            &root.join("release/support-policy.json"),
            &json!({"publication_status": "pre-stable-draft"}),
        )
        .expect("write Stable support fixture");

        let transition =
            render_stage_transition(&root, "v0.7.0").expect("render Stable transition");
        let published_feedback: Value = serde_json::from_slice(
            transition
                .files
                .get("release/feedback-snapshot.json")
                .expect("feedback transition output"),
        )
        .expect("parse published feedback");
        assert_eq!(published_feedback["next_roadmap"]["status"], "published");
        assert_eq!(
            published_feedback["public_issues"],
            feedback["public_issues"]
        );
        assert_eq!(
            published_feedback["release_downloads"],
            feedback["release_downloads"]
        );
        assert_eq!(
            transition
                .files
                .get("release/feedback-snapshot.json")
                .expect("feedback transition bytes"),
            &feedback_before
                .replacen("\"status\": \"reviewed\"", "\"status\": \"published\"", 1)
                .into_bytes()
        );
        assert!(
            String::from_utf8(
                transition
                    .files
                    .get("docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md")
                    .expect("roadmap transition output")
                    .clone()
            )
            .expect("UTF-8 roadmap")
            .contains("**Status:** Published")
        );
        fs::remove_dir_all(root).expect("remove Stable fixture");
    }

    #[test]
    fn native_documentation_preparation_requires_exact_run_evidence() {
        let missing_run = json!({
            "status": "prepared-native",
            "native_matrix_run": null,
            "evidence": ["five-target lifecycle smoke passed"]
        });
        assert!(validate_documentation_uninstall_progress(&missing_run).is_err());

        let qualified = json!({
            "status": "prepared-native",
            "native_matrix_run": 29_637_471_699_u64,
            "evidence": ["five-target lifecycle smoke passed"]
        });
        validate_documentation_uninstall_progress(&qualified).expect("native preparation evidence");
    }

    #[test]
    fn planned_feature_freeze_cannot_preapprove_source_changes() {
        let freeze = json!({"status": "planned", "baseline_commit": null});
        let planned = json!({
            "schema": FEATURE_FREEZE_EXCEPTIONS_SCHEMA,
            "status": "planned",
            "baseline_commit": null,
            "exceptions": []
        });
        validate_feature_freeze_exception_record(&freeze, &planned)
            .expect("canonical planned feature freeze");

        let mut preauthorized = planned;
        preauthorized["exceptions"] = json!([{
            "commit": "0000000000000000000000000000000000000000",
            "class": "release-blocker",
            "reason": "Future changes cannot be authorized before the freeze.",
            "paths": ["crates/canisend-store/src/lib.rs"]
        }]);
        assert!(validate_feature_freeze_exception_record(&freeze, &preauthorized).is_err());

        let baseline = "0123456789abcdef0123456789abcdef01234567";
        let frozen = json!({"status": "frozen", "baseline_commit": baseline});
        let frozen_record = json!({
            "schema": FEATURE_FREEZE_EXCEPTIONS_SCHEMA,
            "status": "frozen",
            "baseline_commit": baseline,
            "exceptions": []
        });
        validate_feature_freeze_exception_record(&frozen, &frozen_record)
            .expect("canonical frozen feature-freeze record");
        assert!(is_automatic_feature_freeze_path(
            "docs/release/feature-freeze.md"
        ));
        assert!(is_automatic_feature_freeze_path(
            "release/qualification-ledger.json"
        ));
        assert!(!is_automatic_feature_freeze_path(
            "crates/canisend-store/src/lib.rs"
        ));
        assert!(!is_automatic_feature_freeze_path(
            ".github/workflows/release.yml"
        ));
    }

    #[test]
    fn feature_freeze_activation_is_head_bound_and_two_file_only() {
        let root = std::env::temp_dir().join(format!(
            "canisend-feature-freeze-activation-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale activation fixture");
        }
        fs::create_dir_all(root.join("release")).expect("create activation fixture");
        run_git(&root, &["init", "--initial-branch=main"])
            .expect("initialize activation repository");
        run_git(&root, &["config", "user.name", "CanISend qualification"])
            .expect("configure activation name");
        run_git(
            &root,
            &["config", "user.email", "qualification@canisend.invalid"],
        )
        .expect("configure activation email");
        fs::write(root.join("README.md"), "fixture\n").expect("write initial fixture");
        run_git(&root, &["add", "README.md"]).expect("stage initial fixture");
        run_git(&root, &["commit", "-m", "initialize fixture"]).expect("commit initial fixture");
        write_pretty_json(
            &root.join("release/qualification-ledger.json"),
            &json!({
                "schema": RELEASE_QUALIFICATION_SCHEMA,
                "workspace_stage": "beta",
                "status": "beta-qualifying",
                "stable_authorized": false,
                "beta": {"status": "qualified"},
                "feature_freeze": {"status": "planned", "baseline_commit": null}
            }),
        )
        .expect("write activation ledger");
        write_pretty_json(
            &root.join("release/feature-freeze-exceptions.json"),
            &json!({
                "schema": FEATURE_FREEZE_EXCEPTIONS_SCHEMA,
                "status": "planned",
                "baseline_commit": null,
                "exceptions": []
            }),
        )
        .expect("write activation exceptions");
        run_git(&root, &["add", "release"]).expect("stage Beta qualification fixture");
        run_git(&root, &["commit", "-m", "qualify beta"])
            .expect("commit Beta qualification fixture");
        let baseline = run_git_lines(&root, &["rev-parse", "HEAD"])
            .expect("read activation baseline")
            .pop()
            .expect("activation baseline");
        let parent = run_git_lines(&root, &["rev-parse", "HEAD^"])
            .expect("read activation parent")
            .pop()
            .expect("activation parent");
        assert!(render_feature_freeze_activation(&root, &parent).is_err());

        let before = fs::read(root.join("release/qualification-ledger.json"))
            .expect("read ledger before activation");
        let freeze = render_feature_freeze_activation(&root, &baseline)
            .expect("render feature-freeze activation");
        assert_eq!(freeze.files.len(), 2);
        assert_eq!(
            feature_freeze_report(&root, &freeze, false).expect("activation dry-run report")["writes_performed"],
            false
        );
        assert_eq!(
            fs::read(root.join("release/qualification-ledger.json"))
                .expect("read ledger after activation dry run"),
            before
        );
        for (relative, body) in &freeze.files {
            fs::write(root.join(relative), body).expect("apply activation fixture");
        }
        run_git(&root, &["add", "release"]).expect("stage activation fixture");
        run_git(&root, &["commit", "-m", "activate feature freeze"])
            .expect("commit activation fixture");
        validate_feature_freeze_history(&root, &baseline, &[])
            .expect("automatic activation history");
        fs::remove_dir_all(root).expect("remove activation fixture");
    }

    #[test]
    fn frozen_feature_history_requires_exact_commit_paths() {
        let root = std::env::temp_dir().join(format!(
            "canisend-feature-freeze-history-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale feature-freeze fixture");
        }
        fs::create_dir_all(root.join("crates")).expect("create source fixture directory");
        run_git(&root, &["init", "--initial-branch=main"]).expect("initialize fixture repository");
        run_git(&root, &["config", "user.name", "CanISend qualification"])
            .expect("configure fixture name");
        run_git(
            &root,
            &["config", "user.email", "qualification@canisend.invalid"],
        )
        .expect("configure fixture email");
        fs::write(root.join("crates/core.txt"), "baseline\n").expect("write baseline source");
        run_git(&root, &["add", "crates/core.txt"]).expect("stage baseline source");
        run_git(&root, &["commit", "-m", "baseline"]).expect("commit baseline source");
        let baseline = run_git_lines(&root, &["rev-parse", "HEAD"])
            .expect("read baseline commit")
            .pop()
            .expect("baseline commit");

        fs::create_dir_all(root.join("docs")).expect("create docs fixture directory");
        fs::write(root.join("docs/note.md"), "automatic documentation\n")
            .expect("write documentation fixture");
        run_git(&root, &["add", "docs/note.md"]).expect("stage documentation fixture");
        run_git(&root, &["commit", "-m", "document release"])
            .expect("commit documentation fixture");

        fs::write(root.join("crates/core.txt"), "release blocker fix\n")
            .expect("write blocker fixture");
        run_git(&root, &["add", "crates/core.txt"]).expect("stage blocker fixture");
        run_git(&root, &["commit", "-m", "fix release blocker"]).expect("commit blocker fixture");
        let blocker = run_git_lines(&root, &["rev-parse", "HEAD"])
            .expect("read blocker commit")
            .pop()
            .expect("blocker commit");

        let exceptions = vec![json!({
            "commit": blocker,
            "class": "release-blocker",
            "reason": "Correct the owned release implementation before RC qualification.",
            "paths": ["crates/core.txt"]
        })];
        validate_feature_freeze_history(&root, &baseline, &exceptions)
            .expect("exact feature-freeze history");

        let mut wrong = exceptions;
        wrong[0]["paths"] = json!(["crates/other.txt"]);
        assert!(validate_feature_freeze_history(&root, &baseline, &wrong).is_err());
        fs::remove_dir_all(root).expect("remove feature-freeze fixture");
    }

    #[test]
    fn scheduled_fuzz_policy_is_pinned_and_complete() {
        check_fuzz_policy().expect("scheduled fuzz policy");
    }

    #[test]
    fn generated_property_test_policy_is_distinct_and_pinned() {
        check_property_test_policy().expect("generated property-test policy");
    }

    #[test]
    fn channel_candidates_preserve_archives_and_nested_binary_paths() {
        let files = render_channel_candidates(&sample_channel_source()).expect("render candidates");
        let homebrew = &files["homebrew/Casks/canisend.rb"];
        assert!(homebrew.contains("arch arm: \"aarch64\", intel: \"x86_64\""));
        assert!(homebrew.contains("sha256 arm:"));
        assert!(homebrew.contains("binary \"canisend-#{version}-#{arch}-apple-darwin/canisend\""));
        let scoop: Value =
            serde_json::from_str(&files["scoop/bucket/canisend.json"]).expect("valid Scoop JSON");
        assert_eq!(
            scoop["architecture"]["64bit"]["hash"],
            "4444444444444444444444444444444444444444444444444444444444444444"
        );
        assert_eq!(
            scoop["extract_dir"],
            "canisend-0.7.0-alpha.1-x86_64-pc-windows-msvc"
        );
        assert_eq!(scoop["license"], "MIT");
        let locale = files
            .iter()
            .find(|(path, _)| path.ends_with(".locale.en-US.yaml"))
            .map(|(_, body)| body)
            .expect("WinGet locale candidate");
        assert!(locale.contains("License: MIT\n"));
        let installer = files
            .iter()
            .find(|(path, _)| path.ends_with(".installer.yaml"))
            .map(|(_, body)| body)
            .expect("WinGet installer candidate");
        assert!(installer.contains("  PortableCommandAlias: canisend\n"));
        assert!(installer.contains("  InstallerUrl: https://"));
        assert!(installer.contains("canisend-0.7.0-alpha.1-x86_64-pc-windows-msvc\\canisend.exe"));
    }

    #[test]
    fn channel_license_preserves_history_and_changes_at_alpha_six() {
        assert_eq!(channel_license("0.7.0").expect("historical license"), "MIT");
        assert_eq!(
            channel_license("1.0.0-alpha.5").expect("last MIT Alpha license"),
            "MIT"
        );
        assert_eq!(
            channel_license("1.0.0-alpha.6").expect("first GPL Alpha license"),
            GPL_LICENSE
        );
        assert_eq!(
            channel_license("1.0.0").expect("Stable GPL license"),
            GPL_LICENSE
        );
    }

    #[test]
    fn stable_release_embeds_canonical_scoped_channel_manifests() {
        let artifacts = vec![
            json!({
                "target": "aarch64-apple-darwin",
                "archive": "canisend-0.7.0-aarch64-apple-darwin.tar.gz",
                "sha256": "1".repeat(64),
                "size": 11
            }),
            json!({
                "target": "x86_64-apple-darwin",
                "archive": "canisend-0.7.0-x86_64-apple-darwin.tar.gz",
                "sha256": "2".repeat(64),
                "size": 12
            }),
            json!({
                "target": "x86_64-pc-windows-msvc",
                "archive": "canisend-0.7.0-x86_64-pc-windows-msvc.zip",
                "sha256": "3".repeat(64),
                "size": 13
            }),
        ];
        let commit = "d".repeat(40);
        let files = render_stable_channel_publication(
            "v0.7.0",
            &commit,
            &artifacts,
            &sample_stable_publication_ledger(),
        )
        .expect("render qualified Stable channel assets");
        assert_eq!(
            files.keys().cloned().collect::<BTreeSet<_>>(),
            stable_channel_asset_names("0.7.0")
        );
        let source: Value =
            serde_json::from_slice(&files["canisend-0.7.0-channel-publication.json"])
                .expect("parse Stable channel source");
        assert_eq!(source["publication"]["scope"], "github-release-assets");
        assert_eq!(source["publication"]["authorized"], true);
        assert_eq!(source["publication"]["external_index_submission"], false);
        assert_eq!(source["manifests"].as_array().map(Vec::len), Some(5));

        let root = std::env::temp_dir().join(format!(
            "canisend-stable-channel-publication-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale Stable channel fixture");
        }
        fs::create_dir_all(&root).expect("create Stable channel fixture");
        for (name, body) in &files {
            fs::write(root.join(name), body).expect("write Stable channel fixture");
        }
        let release_manifest = json!({
            "source": {"commit": commit},
            "artifacts": artifacts
        });
        verify_stable_channel_publication(&root, "v0.7.0", &release_manifest)
            .expect("verify Stable channel assets");
        fs::write(root.join("canisend-0.7.0-homebrew-cask.rb"), b"tampered\n")
            .expect("tamper Stable channel fixture");
        assert!(verify_stable_channel_publication(&root, "v0.7.0", &release_manifest).is_err());
        fs::remove_dir_all(root).expect("remove Stable channel fixture");
    }

    #[test]
    fn package_manager_qualification_policy_is_native_and_nonpublishing() {
        check_package_manager_qualification_policy().expect("package-manager qualification policy");
    }

    #[test]
    fn upgrade_qualification_policy_is_five_target_and_nonpublishing() {
        check_upgrade_qualification_policy().expect("upgrade qualification policy");
    }

    #[test]
    fn upgrade_evidence_requires_canonical_old_binary_behavior_and_checks() {
        let mut evidence = sample_upgrade_evidence(
            "upgrade-aarch64-apple-darwin",
            "aarch64-apple-darwin",
            "macos-15",
        );
        validate_upgrade_qualification_record(
            &evidence,
            "upgrade-aarch64-apple-darwin",
            "aarch64-apple-darwin",
            "macos-15",
            "v0.7.0-beta.1",
            "v0.7.0-rc.1",
        )
        .expect("canonical upgrade evidence");

        evidence["checks"]["no-publication"] = Value::Bool(false);
        assert!(
            validate_upgrade_qualification_record(
                &evidence,
                "upgrade-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
                "v0.7.0-beta.1",
                "v0.7.0-rc.1",
            )
            .is_err()
        );

        let mut impossible = sample_upgrade_evidence(
            "upgrade-aarch64-apple-darwin",
            "aarch64-apple-darwin",
            "macos-15",
        );
        impossible["old_binary_behavior"] =
            Value::String("future-schema-rejected-without-mutation".to_owned());
        assert!(
            validate_upgrade_qualification_record(
                &impossible,
                "upgrade-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
                "v0.7.0-beta.1",
                "v0.7.0-rc.1",
            )
            .is_err()
        );
    }

    #[test]
    fn upgrade_evidence_directory_binds_one_five_target_run() {
        let root =
            std::env::temp_dir().join(format!("canisend-upgrade-evidence-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale upgrade evidence fixture");
        }
        fs::create_dir_all(&root).expect("create upgrade evidence fixture");
        for (file, record, target, environment) in [
            (
                "upgrade-aarch64-apple-darwin.json",
                "upgrade-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
            ),
            (
                "upgrade-x86_64-apple-darwin.json",
                "upgrade-x86_64-apple-darwin",
                "x86_64-apple-darwin",
                "macos-15-intel",
            ),
            (
                "upgrade-x86_64-unknown-linux-gnu.json",
                "upgrade-x86_64-unknown-linux-gnu",
                "x86_64-unknown-linux-gnu",
                "ubuntu-24.04",
            ),
            (
                "upgrade-x86_64-unknown-linux-musl.json",
                "upgrade-x86_64-unknown-linux-musl",
                "x86_64-unknown-linux-musl",
                "ubuntu-24.04",
            ),
            (
                "upgrade-x86_64-pc-windows-msvc.json",
                "upgrade-x86_64-pc-windows-msvc",
                "x86_64-pc-windows-msvc",
                "windows-2025",
            ),
        ] {
            write_pretty_json(
                &root.join(file),
                &sample_upgrade_evidence(record, target, environment),
            )
            .expect("write upgrade evidence fixture");
        }
        let summary = verify_upgrade_qualification_evidence("v0.7.0-beta.1", "v0.7.0-rc.1", &root)
            .expect("verify five target upgrade evidence");
        assert_eq!(summary.records, 5);

        fs::write(root.join("extra.json"), b"{}\n").expect("write extra evidence fixture");
        assert!(
            verify_upgrade_qualification_evidence("v0.7.0-beta.1", "v0.7.0-rc.1", &root,).is_err()
        );
        fs::remove_dir_all(root).expect("remove upgrade evidence fixture");
    }

    #[test]
    fn upgrade_ledger_promotion_requires_recorded_exact_rc() {
        let ledger = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "rc",
            "status": "rc-qualifying",
            "beta": {"status": "qualified", "tag": "v0.7.0-beta.1"},
            "feature_freeze": {"status": "frozen"},
            "release_candidates": [
                {"status": "success", "tag": "v0.7.0-rc.1"}
            ],
            "stable_authorized": false,
            "upgrade_matrix": {
                "beta_tag": null,
                "evidence": [],
                "rc_tag": null,
                "status": "pending"
            }
        });
        let summary = UpgradeQualificationSummary {
            run_id: 29_650_000_001,
            from_manifest_sha256: "a".repeat(64),
            to_manifest_sha256: "b".repeat(64),
            records: 5,
        };
        let qualified = upgrade_qualified_ledger(&ledger, "v0.7.0-beta.1", "v0.7.0-rc.1", &summary)
            .expect("promote exact RC upgrade evidence");
        assert_eq!(qualified["upgrade_matrix"]["status"], "passed");

        assert!(
            upgrade_qualified_ledger(&ledger, "v0.7.0-beta.1", "v0.7.0-rc.2", &summary,).is_err()
        );
    }

    #[test]
    fn documentation_uninstall_policy_is_same_rc_run_and_five_target() {
        check_documentation_uninstall_policy().expect("documentation/uninstall policy");
    }

    #[test]
    fn documentation_uninstall_record_binds_manifest_archive_and_checks() {
        let digest = "d".repeat(64);
        let mut evidence = sample_documentation_uninstall_evidence(
            "documentation-uninstall-aarch64-apple-darwin",
            "aarch64-apple-darwin",
            "macos-15",
            &digest,
        );
        validate_documentation_uninstall_record(
            &evidence,
            "documentation-uninstall-aarch64-apple-darwin",
            "aarch64-apple-darwin",
            "macos-15",
            "v0.7.0-rc.1",
            &digest,
        )
        .expect("canonical documentation/uninstall record");

        evidence["checks"]["workspace-retained"] = Value::Bool(false);
        assert!(
            validate_documentation_uninstall_record(
                &evidence,
                "documentation-uninstall-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
                "v0.7.0-rc.1",
                &digest,
            )
            .is_err()
        );
        let wrong = sample_documentation_uninstall_evidence(
            "documentation-uninstall-aarch64-apple-darwin",
            "aarch64-apple-darwin",
            "macos-15",
            &digest,
        );
        assert!(
            validate_documentation_uninstall_record(
                &wrong,
                "documentation-uninstall-aarch64-apple-darwin",
                "aarch64-apple-darwin",
                "macos-15",
                "v0.7.0-rc.1",
                &"e".repeat(64),
            )
            .is_err()
        );
    }

    #[test]
    fn documentation_uninstall_promotion_requires_same_recorded_rc_run() {
        let ledger = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "rc",
            "status": "rc-qualifying",
            "beta": {"status": "qualified"},
            "feature_freeze": {"status": "frozen"},
            "release_candidates": [{
                "tag": "v0.7.0-rc.1",
                "source_commit": "c".repeat(40),
                "signed_matrix_run": 29_660_000_001_u64,
                "status": "success"
            }],
            "documentation_uninstall": {
                "evidence": ["five-target Alpha preparation"],
                "native_matrix_run": 29_637_471_699_u64,
                "status": "prepared-native"
            },
            "stable_authorized": false
        });
        let summary = DocumentationUninstallSummary {
            run_id: 29_660_000_001,
            records: 5,
        };
        let qualified = documentation_uninstall_qualified_ledger(&ledger, "v0.7.0-rc.1", summary)
            .expect("qualify same RC run documentation evidence");
        assert_eq!(qualified["documentation_uninstall"]["status"], "passed");
        assert_eq!(
            qualified["documentation_uninstall"]["native_matrix_run"],
            summary.run_id
        );

        let wrong_run = DocumentationUninstallSummary {
            run_id: summary.run_id + 1,
            records: 5,
        };
        assert!(
            documentation_uninstall_qualified_ledger(&ledger, "v0.7.0-rc.1", wrong_run,).is_err()
        );
    }

    #[test]
    fn package_manager_evidence_requires_all_true_canonical_checks() {
        let mut evidence = sample_package_manager_evidence(
            "homebrew-aarch64-apple-darwin",
            "homebrew-cask",
            "aarch64-apple-darwin",
            "macos-15",
        );
        validate_package_manager_evidence_record(
            &evidence,
            "homebrew-aarch64-apple-darwin",
            "homebrew-cask",
            "aarch64-apple-darwin",
            "macos-15",
            "v0.7.0-beta.1",
            "v0.7.0-rc.1",
        )
        .expect("canonical package-manager evidence");

        evidence["checks"]["upgrade"] = Value::Bool(false);
        assert!(
            validate_package_manager_evidence_record(
                &evidence,
                "homebrew-aarch64-apple-darwin",
                "homebrew-cask",
                "aarch64-apple-darwin",
                "macos-15",
                "v0.7.0-beta.1",
                "v0.7.0-rc.1",
            )
            .is_err()
        );
    }

    #[test]
    fn package_manager_evidence_directory_binds_one_native_run() {
        let root = std::env::temp_dir().join(format!(
            "canisend-package-manager-evidence-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale package evidence fixture");
        }
        fs::create_dir_all(&root).expect("create package evidence fixture");
        for (file, record, channel, target, environment) in [
            (
                "homebrew-aarch64-apple-darwin.json",
                "homebrew-aarch64-apple-darwin",
                "homebrew-cask",
                "aarch64-apple-darwin",
                "macos-15",
            ),
            (
                "homebrew-x86_64-apple-darwin.json",
                "homebrew-x86_64-apple-darwin",
                "homebrew-cask",
                "x86_64-apple-darwin",
                "macos-15-intel",
            ),
            (
                "scoop-x86_64-pc-windows-msvc.json",
                "scoop-x86_64-pc-windows-msvc",
                "scoop",
                "x86_64-pc-windows-msvc",
                "windows-2025",
            ),
            (
                "winget-x86_64-pc-windows-msvc.json",
                "winget-x86_64-pc-windows-msvc",
                "winget",
                "x86_64-pc-windows-msvc",
                "windows-sandbox",
            ),
        ] {
            write_pretty_json(
                &root.join(file),
                &sample_package_manager_evidence(record, channel, target, environment),
            )
            .expect("write package evidence fixture");
        }
        let summary = verify_package_manager_evidence("v0.7.0-beta.1", "v0.7.0-rc.1", &root)
            .expect("verify complete package evidence");
        assert_eq!(summary.records, 4);
        assert!(verify_package_manager_evidence("v0.7.0-alpha.1", "v0.7.0-rc.1", &root).is_err());
        fs::remove_dir_all(root).expect("remove package evidence fixture");
    }

    #[test]
    fn package_manager_promotion_requires_exact_beta_and_recorded_rc() {
        let ledger = json!({
            "schema": RELEASE_QUALIFICATION_SCHEMA,
            "workspace_stage": "rc",
            "status": "rc-qualifying",
            "beta": {"status": "qualified", "tag": "v0.7.0-beta.1"},
            "feature_freeze": {"status": "frozen"},
            "release_candidates": [
                {"status": "success", "tag": "v0.7.0-rc.1"}
            ],
            "package_managers": {
                "channels": ["homebrew-cask", "scoop", "winget"],
                "evidence": [],
                "status": "candidates-only"
            },
            "stable_authorized": false
        });
        let summary = PackageManagerQualificationSummary {
            run_id: 29_670_000_001,
            records: 4,
        };
        let qualified =
            package_manager_qualified_ledger(&ledger, "v0.7.0-beta.1", "v0.7.0-rc.1", summary)
                .expect("qualify exact package-manager pair");
        assert_eq!(qualified["package_managers"]["status"], "passed");
        validate_package_manager_qualification_record(
            &qualified["package_managers"],
            &ledger["beta"],
            ledger["release_candidates"]
                .as_array()
                .expect("package RC candidates"),
        )
        .expect("validate canonical package-manager ledger record");
        assert!(
            package_manager_qualified_ledger(&ledger, "v0.7.0-beta.1", "v0.7.0-rc.2", summary,)
                .is_err()
        );
    }

    #[test]
    fn package_candidate_source_binds_verified_manifest_bytes() {
        let root = std::env::temp_dir().join(format!(
            "canisend-package-candidate-source-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale package source fixture");
        }
        fs::create_dir_all(&root).expect("create package source fixture");
        let mut source = sample_channel_source();
        let artifacts = source
            .artifacts
            .values()
            .map(|artifact| {
                json!({
                    "target": artifact.target,
                    "archive": artifact.archive,
                    "sha256": artifact.sha256,
                    "size": artifact.size
                })
            })
            .collect::<Vec<_>>();
        let manifest = json!({
            "tag": source.tag,
            "version": source.version,
            "stage": source.stage.as_str(),
            "source": {"commit": source.source_commit},
            "artifacts": artifacts
        });
        let manifest_path = root.join(&source.manifest_file);
        write_pretty_json(&manifest_path, &manifest).expect("write package source manifest");
        source.manifest_sha256 = sha256_file(&manifest_path).expect("manifest fixture hash");
        validate_package_candidate_source_against_assets(
            &source,
            NATIVE_ALPHA_TAG,
            ReleaseStage::Alpha,
            &root,
        )
        .expect("candidate source binding");

        source.manifest_sha256 = "0".repeat(64);
        assert!(
            validate_package_candidate_source_against_assets(
                &source,
                NATIVE_ALPHA_TAG,
                ReleaseStage::Alpha,
                &root,
            )
            .is_err()
        );
        fs::remove_dir_all(root).expect("remove package source fixture");
    }

    #[test]
    fn channel_candidate_source_cannot_authorize_publication() {
        let mut value = sample_channel_source().to_value();
        value["publication_authorized"] = Value::Bool(true);
        assert!(channel_candidate_source_from_value(&value).is_err());
    }

    #[test]
    fn signing_evidence_binds_exact_final_archive() {
        let root = std::env::temp_dir().join(format!(
            "canisend-xtask-signing-evidence-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale signing fixture");
        }
        fs::create_dir_all(&root).expect("create signing fixture");
        let binary = root.join("canisend");
        fs::write(&binary, b"signed binary fixture").expect("write signed binary fixture");
        let evidence = root.join("evidence.json");
        let mut signing_evidence = sample_apple_signing_evidence();
        signing_evidence["binary"]["sha256"] =
            Value::String(sha256_file(&binary).expect("binary fixture hash"));
        signing_evidence["binary"]["size"] =
            Value::Number(file_size(&binary).expect("binary fixture size").into());
        write_pretty_json(&evidence, &signing_evidence).expect("write signing fixture");
        let archive = root.join(format!(
            "canisend-{}-aarch64-apple-darwin.tar.gz",
            env!("CARGO_PKG_VERSION")
        ));
        fs::write(&archive, b"signed archive fixture").expect("write archive fixture");
        bind_signing_evidence(
            &format!("v{}", env!("CARGO_PKG_VERSION")),
            "aarch64-apple-darwin",
            &evidence,
            &binary,
            &archive,
        )
        .expect("bind signing evidence");
        let bound: Value =
            serde_json::from_slice(&fs::read(&evidence).expect("read bound evidence"))
                .expect("parse bound evidence");
        assert_eq!(
            bound["archive"]["sha256"],
            sha256_file(&archive).expect("archive hash")
        );
        let target = release_targets()
            .expect("release targets")
            .into_iter()
            .find(|target| target.triple == "aarch64-apple-darwin")
            .expect("Apple target");
        read_bound_signing_evidence(&evidence, &target, env!("CARGO_PKG_VERSION"), &archive)
            .expect("verify bound evidence");
        fs::remove_dir_all(root).expect("remove signing fixture");
    }

    #[test]
    fn macos_gui_qualification_binds_exact_archive_and_checks() {
        let root = std::env::temp_dir().join(format!(
            "canisend-macos-gui-qualification-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale macOS GUI qualification fixture");
        }
        fs::create_dir_all(&root).expect("create macOS GUI qualification fixture");
        let version = env!("CARGO_PKG_VERSION");
        let tag = format!("v{version}");
        let archive = root.join(macos_gui_archive_name(version));
        fs::write(&archive, b"bounded desktop archive fixture")
            .expect("write macOS GUI archive fixture");
        let evidence_path = root.join(macos_gui_qualification_name(version));
        let mut evidence = json!({
            "schema": "canisend.macos-gui-qualification/v1",
            "record": "desktop-macos-aarch64",
            "target": "aarch64-apple-darwin",
            "environment": "macos-15",
            "profile": ReleaseStage::Alpha.cargo_profile(),
            "tag": tag,
            "version": version,
            "archive": {
                "file": macos_gui_archive_name(version),
                "sha256": sha256_file(&archive).expect("hash GUI archive fixture"),
                "size": file_size(&archive).expect("size GUI archive fixture")
            },
            "github_run_id": 42_u64,
            "checks": {
                "bounded_archive": true,
                "exact_top_level": true,
                "no_symlinks": true,
                "companion_integrity": true,
                "nested_adhoc_signatures": true,
                "outer_adhoc_signature": true,
                "version_match": true,
                "packaged_cli_doctor": true,
                "packaged_dual_pack_quickstart": true,
                "packaged_agent_v4_host_resources": true,
                "packaged_agent_v4_mcp_lifecycle": true,
                "packaged_gui_launch": true,
                "no_publication": true
            },
            "completed_at": "2026-07-25T20:00:00Z"
        });
        write_pretty_json(&evidence_path, &evidence)
            .expect("write canonical macOS GUI qualification evidence");
        read_macos_gui_qualification(&evidence_path, &tag, version, &archive)
            .expect("accept exact macOS GUI qualification evidence");

        evidence["profile"] = Value::String("release".to_owned());
        write_pretty_json(&evidence_path, &evidence)
            .expect("write profile-drifted macOS GUI qualification evidence");
        assert!(read_macos_gui_qualification(&evidence_path, &tag, version, &archive).is_err());

        evidence["profile"] = Value::String(ReleaseStage::Alpha.cargo_profile().to_owned());
        evidence["checks"]["packaged_gui_launch"] = Value::Bool(false);
        write_pretty_json(&evidence_path, &evidence)
            .expect("write malformed macOS GUI qualification evidence");
        assert!(read_macos_gui_qualification(&evidence_path, &tag, version, &archive).is_err());
        fs::remove_dir_all(root).expect("remove macOS GUI qualification fixture");
    }

    #[test]
    fn macos_gui_dmg_qualification_binds_exact_image_and_checks() {
        let root = std::env::temp_dir().join(format!(
            "canisend-macos-gui-dmg-qualification-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale macOS GUI DMG fixture");
        }
        fs::create_dir_all(&root).expect("create macOS GUI DMG fixture");
        let version = env!("CARGO_PKG_VERSION");
        let tag = format!("v{version}");
        let dmg = root.join(macos_gui_dmg_name(version));
        fs::write(&dmg, b"bounded desktop DMG fixture").expect("write macOS GUI DMG fixture");
        let evidence_path = root.join(macos_gui_dmg_qualification_name(version));
        let mut evidence = json!({
            "schema": "canisend.macos-gui-dmg-qualification/v1",
            "record": "desktop-macos-aarch64-dmg",
            "target": "aarch64-apple-darwin",
            "environment": "macos-15",
            "profile": ReleaseStage::Alpha.cargo_profile(),
            "tag": tag,
            "version": version,
            "image": {
                "file": macos_gui_dmg_name(version),
                "sha256": sha256_file(&dmg).expect("hash GUI DMG fixture"),
                "size": file_size(&dmg).expect("size GUI DMG fixture")
            },
            "github_run_id": 42_u64,
            "checks": {
                "bounded_image": true,
                "hdiutil_verify": true,
                "readonly_mount": true,
                "exact_top_level": true,
                "applications_link": true,
                "companion_integrity": true,
                "nested_adhoc_signatures": true,
                "outer_adhoc_signature": true,
                "version_match": true,
                "no_publication": true
            },
            "completed_at": "2026-07-28T20:00:00Z"
        });
        write_pretty_json(&evidence_path, &evidence)
            .expect("write canonical macOS GUI DMG qualification evidence");
        read_macos_gui_dmg_qualification(&evidence_path, &tag, version, &dmg)
            .expect("accept exact macOS GUI DMG qualification evidence");

        evidence["checks"]["readonly_mount"] = Value::Bool(false);
        write_pretty_json(&evidence_path, &evidence)
            .expect("write malformed macOS GUI DMG qualification evidence");
        assert!(read_macos_gui_dmg_qualification(&evidence_path, &tag, version, &dmg).is_err());
        fs::remove_dir_all(root).expect("remove macOS GUI DMG fixture");
    }

    #[test]
    fn macos_gui_intel_compilation_is_compile_only_and_commit_bound() {
        let root = std::env::temp_dir().join(format!(
            "canisend-macos-gui-intel-compilation-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale Intel GUI compilation fixture");
        }
        fs::create_dir_all(&root).expect("create Intel GUI compilation fixture");
        let version = env!("CARGO_PKG_VERSION");
        let tag = format!("v{version}");
        let commit = "a".repeat(40);
        let path = root.join(macos_gui_intel_compilation_name(version));
        let mut evidence = json!({
            "schema": "canisend.macos-gui-compilation/v1",
            "record": "desktop-macos-intel-compile-only",
            "target": "x86_64-apple-darwin",
            "environment": "macos-15-intel",
            "tag": tag,
            "version": version,
            "source_commit": commit,
            "binary": {
                "file": "canisend-gui",
                "architecture": "x86_64",
                "profile": "release",
                "sha256": "b".repeat(64),
                "size": 42_u64
            },
            "github_run_id": 43_u64,
            "checks": {
                "locked_build": true,
                "release_profile": true,
                "target_architecture": true,
                "archive_published": false,
                "native_runtime_qualified": false,
                "support_claim": false,
                "no_publication": true
            },
            "completed_at": "2026-07-25T20:00:00Z"
        });
        write_pretty_json(&path, &evidence).expect("write Intel GUI compilation evidence");
        read_macos_gui_intel_compilation(&path, &tag, version, &commit)
            .expect("accept canonical Intel GUI compilation evidence");

        evidence["checks"]["support_claim"] = Value::Bool(true);
        write_pretty_json(&path, &evidence)
            .expect("write overclaiming Intel GUI compilation evidence");
        assert!(read_macos_gui_intel_compilation(&path, &tag, version, &commit).is_err());
        fs::remove_dir_all(root).expect("remove Intel GUI compilation fixture");
    }

    #[test]
    fn signing_evidence_rejects_signed_binary_mismatch() {
        let root = std::env::temp_dir().join(format!(
            "canisend-xtask-signing-binary-mismatch-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale signing fixture");
        }
        fs::create_dir_all(&root).expect("create signing fixture");
        let binary = root.join("canisend");
        fs::write(&binary, b"different signed binary").expect("write signed binary fixture");
        let evidence = root.join("evidence.json");
        write_pretty_json(&evidence, &sample_apple_signing_evidence())
            .expect("write signing fixture");
        let archive = root.join(format!(
            "canisend-{}-aarch64-apple-darwin.tar.gz",
            env!("CARGO_PKG_VERSION")
        ));
        fs::write(&archive, b"signed archive fixture").expect("write archive fixture");
        assert!(
            bind_signing_evidence(
                &format!("v{}", env!("CARGO_PKG_VERSION")),
                "aarch64-apple-darwin",
                &evidence,
                &binary,
                &archive,
            )
            .is_err()
        );
        fs::remove_dir_all(root).expect("remove signing fixture");
    }

    #[test]
    fn signing_evidence_rejects_claimed_windows_public_trust() {
        let target = release_targets()
            .expect("release targets")
            .into_iter()
            .find(|target| target.triple == "x86_64-pc-windows-msvc")
            .expect("Windows target");
        let mut evidence = sample_windows_signing_evidence();
        evidence["verification"]["certificate_trusted"] = Value::Bool(true);
        assert!(
            canonical_signing_evidence(&evidence, &target, env!("CARGO_PKG_VERSION"), None)
                .is_err()
        );
    }

    #[test]
    fn approval_source_gate_rejects_duplicate_stores_and_missing_expiry_contracts() {
        let root = repository_root();
        let read = |relative: &str| {
            fs::read_to_string(root.join(relative)).expect("read approval source fixture")
        };
        let app = read("crates/canisend-app/src/approval.rs");
        let mut mcp = read("crates/canisend-mcp/src/lib.rs");
        let desktop_state = read("crates/canisend-desktop/src/approval.rs");
        let desktop_host = read("crates/canisend-desktop/src/lib.rs");
        let job = read("crates/canisend-desktop/src/job_intake.rs");
        let discovery = read("crates/canisend-desktop/src/discovery.rs");
        let workflow = read("crates/canisend-desktop/src/workflow.rs");
        let application = read("crates/canisend-desktop/src/application_intake.rs");
        let application_association = read("crates/canisend-app/src/association_v4.rs");
        let desktop_association = read("crates/canisend-desktop/src/association_v4.rs");
        let application_mutations = read("crates/canisend-app/src/application_mutations_v4.rs");
        let desktop_mutations = read("crates/canisend-desktop/src/application_mutations_v4.rs");
        let mut bridge = read("apps/canisend-desktop/src/lib/bridge.ts");
        validate_approval_broker_sources(ApprovalBrokerSources {
            app: &app,
            mcp: &mcp,
            desktop_state: &desktop_state,
            desktop_host: &desktop_host,
            associations: [&application_association, &desktop_association],
            mutations: [&application_mutations, &desktop_mutations],
            desktop_families: [&job, &discovery, &workflow, &application],
            bridge: &bridge,
        })
        .expect("current approval sources");

        mcp.push_str("\nstruct MutationPreviewStore;\n");
        assert!(
            validate_approval_broker_sources(ApprovalBrokerSources {
                app: &app,
                mcp: &mcp,
                desktop_state: &desktop_state,
                desktop_host: &desktop_host,
                associations: [&application_association, &desktop_association],
                mutations: [&application_mutations, &desktop_mutations],
                desktop_families: [&job, &discovery, &workflow, &application],
                bridge: &bridge,
            })
            .is_err()
        );

        mcp = read("crates/canisend-mcp/src/lib.rs");
        bridge = bridge.replacen("remaining_ttl_seconds: number", "ttl_removed: number", 1);
        assert!(
            validate_approval_broker_sources(ApprovalBrokerSources {
                app: &app,
                mcp: &mcp,
                desktop_state: &desktop_state,
                desktop_host: &desktop_host,
                associations: [&application_association, &desktop_association],
                mutations: [&application_mutations, &desktop_mutations],
                desktop_families: [&job, &discovery, &workflow, &application],
                bridge: &bridge,
            })
            .is_err()
        );
    }

    #[test]
    fn semantic_parity_policy_rejects_missing_markers_outcomes_and_shared_bindings() {
        let root = repository_root();
        let policy_path = root.join("crates/canisend-contracts/semantic-parity-v1.json");
        let policy: Value =
            serde_json::from_slice(&fs::read(&policy_path).expect("read semantic parity policy"))
                .expect("parse semantic parity policy");
        let registry = OperationRegistry::built_in().expect("operation registry");
        let current = validate_semantic_parity_policy(&policy, &registry, &root)
            .expect("current semantic parity policy");
        assert_eq!(current.shared_operations, 38);
        assert_eq!(current.preview_pairs, 14);
        assert!(!current.uncovered_bindings.is_empty());

        let mut missing_shared = policy.clone();
        missing_shared["shared_operations"]
            .as_array_mut()
            .expect("shared operations")
            .pop();
        assert!(validate_semantic_parity_policy(&missing_shared, &registry, &root).is_err());

        let mut missing_marker = policy.clone();
        missing_marker["fixtures"][0]["marker"] =
            Value::String("fn removed_semantic_fixture_marker()".to_owned());
        assert!(validate_semantic_parity_policy(&missing_marker, &registry, &root).is_err());

        let mut missing_no_mutation = policy.clone();
        let create = missing_no_mutation["shared_operations"]
            .as_array_mut()
            .expect("shared operations")
            .iter_mut()
            .find(|entry| entry["operation"] == "application.create")
            .expect("Application create coverage");
        create["outcomes"]
            .as_array_mut()
            .expect("Application create outcomes")
            .retain(|outcome| outcome != "no-mutation");
        assert!(validate_semantic_parity_policy(&missing_no_mutation, &registry, &root).is_err());

        let mut missing_surface_binding = policy;
        for cli in missing_surface_binding["pack_surface_cases"]
            .as_array_mut()
            .expect("Pack surface cases")
            .iter_mut()
            .filter(|entry| entry["surface"] == "cli")
        {
            cli["operations"]
                .as_array_mut()
                .expect("CLI operations")
                .retain(|operation| operation != "application.create");
        }
        assert!(
            validate_semantic_parity_policy(&missing_surface_binding, &registry, &root).is_err()
        );
    }
}
