#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use canisend_app::{
    Application, GENERIC_APPLICATION_WORKFLOW_PACK_ID, LocalFileIntakeCommitRequestV4,
    LocalFileIntakePreviewRequestV4, PastedTextIntakeCommitRequestV4,
    PastedTextIntakePreviewRequestV4, PrivateReadConsent,
};
use canisend_contracts::{RequirementPriorityV3, WorkflowPackId, WorkflowPackItemId};
use canisend_io::normalize_html_document;
use lopdf::{
    Document, Object, Stream,
    content::{Content, Operation},
    dictionary,
};
use serde::Serialize;
use serde_json::Value;

const STARTUP_VERSION_LIMIT_MS: u64 = 100;
const STARTUP_HOST_STATUS_LIMIT_MS: u64 = 150;
const LARGE_STATUS_LIMIT_MS: u64 = 500;
const HTML_INTAKE_LIMIT_MS: u64 = 2_000;
const PDF_INTAKE_LIMIT_MS: u64 = 5_000;
const TYPST_RENDER_LIMIT_MS: u64 = 1_000;
const RELEASE_BINARY_LIMIT_BYTES: u64 = 67_108_864;
const LARGE_WORKSPACE_APPLICATIONS: usize = 100;
const PDF_PAGES: usize = 50;

static NEXT: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "canisend-performance-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Debug, Serialize)]
struct PerformanceMetrics {
    format: &'static str,
    target: String,
    release_binary_bytes: u64,
    version_startup_median_ms: u64,
    host_status_startup_median_ms: u64,
    status_100_applications_median_ms: u64,
    html_1_mib_intake_median_ms: u64,
    pdf_50_page_intake_median_ms: u64,
    typst_render_median_ms: u64,
}

#[test]
#[ignore = "release-only performance regression gate"]
fn release_binary_stays_within_product_performance_budgets() {
    let root = TestDirectory::new("release-gate");
    let workspace_path = root.path().join("workspace");
    let pdf_path = root.path().join("fifty-pages.pdf");
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_canisend"));

    run_workspace(&binary, &workspace_path, &["workspace", "init", "--json"]);
    let html_samples = prepare_large_workspace(&workspace_path);
    fs::write(&pdf_path, make_text_pdf(PDF_PAGES)).expect("write PDF benchmark fixture");

    run_static(&binary, &["version", "--json"]);
    let version_startup_median_ms = median_command_millis(7, || {
        run_static(&binary, &["version", "--json"]);
    });

    let binary_text = binary.to_str().expect("UTF-8 release binary path");
    run_workspace(
        &binary,
        &workspace_path,
        &[
            "host",
            "setup",
            "--host",
            "codex",
            "--executable",
            binary_text,
            "--json",
        ],
    );
    let host_status_startup_median_ms = median_command_millis(7, || {
        run_workspace(
            &binary,
            &workspace_path,
            &[
                "host",
                "status",
                "--host",
                "codex",
                "--executable",
                binary_text,
                "--json",
            ],
        );
    });

    run_workspace(&binary, &workspace_path, &["workspace", "status", "--json"]);
    let status_100_applications_median_ms = median_command_millis(5, || {
        run_workspace(&binary, &workspace_path, &["workspace", "status", "--json"]);
    });

    let mut pdf_application_index = 0;
    let pdf_50_page_intake_median_ms = median_command_millis(PDF_PAGES.min(3), || {
        run_local_pdf_intake(&workspace_path, &pdf_path, pdf_application_index);
        pdf_application_index += 1;
    });

    let mut typst_samples = Vec::new();
    for _ in 0..3 {
        let output = run_static(&binary, &["doctor", "--json"]);
        let body: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
        typst_samples.push(
            body.pointer("/data/render_probe/elapsed_millis")
                .and_then(Value::as_u64)
                .expect("doctor render elapsed metric"),
        );
    }
    let typst_render_median_ms = median(typst_samples);
    let metrics = PerformanceMetrics {
        format: "canisend.performance/v1",
        target: format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        release_binary_bytes: fs::metadata(&binary)
            .expect("release binary metadata")
            .len(),
        version_startup_median_ms,
        host_status_startup_median_ms,
        status_100_applications_median_ms,
        html_1_mib_intake_median_ms: median(html_samples),
        pdf_50_page_intake_median_ms,
        typst_render_median_ms,
    };

    enforce(
        "version startup",
        metrics.version_startup_median_ms,
        STARTUP_VERSION_LIMIT_MS,
    );
    enforce(
        "Agent v4 host status startup",
        metrics.host_status_startup_median_ms,
        STARTUP_HOST_STATUS_LIMIT_MS,
    );
    enforce(
        "status for 100 Applications",
        metrics.status_100_applications_median_ms,
        LARGE_STATUS_LIMIT_MS,
    );
    enforce(
        "1 MiB HTML intake",
        metrics.html_1_mib_intake_median_ms,
        HTML_INTAKE_LIMIT_MS,
    );
    enforce(
        "50-page PDF intake",
        metrics.pdf_50_page_intake_median_ms,
        PDF_INTAKE_LIMIT_MS,
    );
    enforce(
        "Typst render",
        metrics.typst_render_median_ms,
        TYPST_RENDER_LIMIT_MS,
    );
    enforce(
        "release binary bytes",
        metrics.release_binary_bytes,
        RELEASE_BINARY_LIMIT_BYTES,
    );

    let compact = serde_json::to_string(&metrics).expect("performance metrics JSON");
    println!("CANISEND_PERFORMANCE_METRICS={compact}");
    if let Some(path) = std::env::var_os("CANISEND_PERFORMANCE_OUTPUT") {
        let mut pretty = serde_json::to_string_pretty(&metrics).expect("pretty metrics JSON");
        pretty.push('\n');
        fs::write(path, pretty).expect("write performance evidence");
    }
}

fn prepare_large_workspace(workspace_path: &Path) -> Vec<u64> {
    for index in 0..LARGE_WORKSPACE_APPLICATIONS {
        let preview_request = pasted_text_request(
            format!("Synthetic application {index:03}"),
            "Synthetic benchmark Requirement.".to_owned(),
        );
        let preview =
            Application::preview_pasted_text_intake_v4(workspace_path, preview_request.clone())
                .expect("preview large Workspace Application")
                .data;
        Application::commit_pasted_text_intake_v4(
            workspace_path,
            PastedTextIntakeCommitRequestV4 {
                preview: preview_request,
                expected_preview_sha256: preview.preview_sha256,
            },
        )
        .expect("commit large Workspace Application");
    }
    let html = one_mib_html();
    let normalized_path = workspace_path.join("benchmark-input.txt");
    let mut html_samples = Vec::new();
    for _ in 0..3 {
        let started = Instant::now();
        let normalized = normalize_html_document(&html).expect("normalize bounded HTML fixture");
        fs::write(&normalized_path, normalized).expect("write normalized HTML fixture");
        html_samples.push(duration_millis(started.elapsed()));
    }
    html_samples
}

fn pasted_text_request(title: String, source_text: String) -> PastedTextIntakePreviewRequestV4 {
    PastedTextIntakePreviewRequestV4 {
        pack_id: WorkflowPackId::try_new(GENERIC_APPLICATION_WORKFLOW_PACK_ID)
            .expect("generic Pack ID"),
        title,
        opportunity_metadata: BTreeMap::new(),
        application_metadata: BTreeMap::new(),
        source_text,
        requirement_category: WorkflowPackItemId::try_new("format")
            .expect("generic Requirement category"),
        requirement_priority: RequirementPriorityV3::Mandatory,
    }
}

fn run_local_pdf_intake(workspace_path: &Path, file: &Path, index: usize) {
    let preview_request = LocalFileIntakePreviewRequestV4 {
        pack_id: WorkflowPackId::try_new(GENERIC_APPLICATION_WORKFLOW_PACK_ID)
            .expect("generic Pack ID"),
        title: format!("PDF intake benchmark {index}"),
        opportunity_metadata: BTreeMap::new(),
        application_metadata: BTreeMap::new(),
        path: file.to_path_buf(),
        requirement_category: WorkflowPackItemId::try_new("format")
            .expect("generic Requirement category"),
        requirement_priority: RequirementPriorityV3::Mandatory,
    };
    let consent = PrivateReadConsent::granted_by_user();
    let preview = Application::preview_local_file_intake_v4(
        workspace_path,
        preview_request.clone(),
        Some(consent),
    )
    .expect("preview 50-page PDF intake")
    .data;
    Application::commit_local_file_intake_v4(
        workspace_path,
        LocalFileIntakeCommitRequestV4 {
            preview: preview_request,
            expected_preview_sha256: preview.preview_sha256,
        },
        Some(consent),
    )
    .expect("commit 50-page PDF intake");
}

fn one_mib_html() -> Vec<u8> {
    let mut html = String::from("<!doctype html><html><body><h1>Programme Manager</h1>");
    while html.len() < 1_048_576 {
        html.push_str(
            "<p>Lead delivery, coordinate stakeholders, document evidence, and support users.</p>",
        );
    }
    html.push_str("</body></html>");
    html.into_bytes()
}

fn run_static(binary: &Path, arguments: &[&str]) -> Output {
    run_success(Command::new(binary).args(arguments))
}

fn run_workspace(binary: &Path, workspace: &Path, arguments: &[&str]) -> Output {
    let mut command = Command::new(binary);
    command.arg("--workspace").arg(workspace).args(arguments);
    run_success(&mut command)
}

fn run_success(command: &mut Command) -> Output {
    let output = command.output().expect("run release binary");
    assert!(
        output.status.success(),
        "command failed with {}: stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn median_command_millis(mut samples: usize, mut command: impl FnMut()) -> u64 {
    let mut values = Vec::with_capacity(samples);
    while samples > 0 {
        let started = Instant::now();
        command();
        values.push(duration_millis(started.elapsed()));
        samples -= 1;
    }
    median(values)
}

fn median(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn duration_millis(duration: Duration) -> u64 {
    let micros = u64::try_from(duration.as_micros()).unwrap_or(u64::MAX);
    micros.saturating_add(999) / 1_000
}

fn enforce(name: &str, actual: u64, limit: u64) {
    assert!(
        actual <= limit,
        "{name} measured {actual}, exceeding the release threshold {limit}"
    );
}

fn make_text_pdf(page_count: usize) -> Vec<u8> {
    let operations = vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["F1".into(), 12.into()]),
        Operation::new("Td", vec![50.into(), 750.into()]),
        Operation::new(
            "Tj",
            vec![Object::string_literal(
                "Programme Manager benchmark fixture",
            )],
        ),
        Operation::new("ET", vec![]),
    ];
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
            Object::string_literal(b"CANISEND-PERF-A"),
            Object::string_literal(b"CANISEND-PERF-B"),
        ]),
    );
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("PDF serialization");
    bytes
}
