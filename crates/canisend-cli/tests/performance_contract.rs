#![forbid(unsafe_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use canisend_contracts::{ActorKind, PrivacyClassification, SourceKind};
use canisend_io::normalize_html_document;
use canisend_store::{JobService, NewSource, Workspace};
use lopdf::{
    Document, Object, Stream,
    content::{Content, Operation},
    dictionary,
};
use serde::Serialize;
use serde_json::Value;

const STARTUP_VERSION_LIMIT_MS: u64 = 100;
const STARTUP_CAPABILITIES_LIMIT_MS: u64 = 150;
const LARGE_STATUS_LIMIT_MS: u64 = 500;
const HTML_INTAKE_LIMIT_MS: u64 = 2_000;
const PDF_INTAKE_LIMIT_MS: u64 = 5_000;
const TYPST_RENDER_LIMIT_MS: u64 = 1_000;
const RELEASE_BINARY_LIMIT_BYTES: u64 = 67_108_864;
const LARGE_WORKSPACE_JOBS: usize = 100;
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
    capabilities_startup_median_ms: u64,
    status_100_jobs_median_ms: u64,
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
    let (html_samples, pdf_job_ids) = prepare_large_workspace(&workspace_path);
    fs::write(&pdf_path, make_text_pdf(PDF_PAGES)).expect("write PDF benchmark fixture");

    run_static(&binary, &["version", "--json"]);
    let version_startup_median_ms = median_command_millis(7, || {
        run_static(&binary, &["version", "--json"]);
    });

    run_static(&binary, &["agent", "capabilities", "--json"]);
    let capabilities_startup_median_ms = median_command_millis(7, || {
        run_static(&binary, &["agent", "capabilities", "--json"]);
    });

    run_workspace(&binary, &workspace_path, &["workspace", "status", "--json"]);
    let status_100_jobs_median_ms = median_command_millis(5, || {
        run_workspace(&binary, &workspace_path, &["workspace", "status", "--json"]);
    });

    let mut pdf_job_index = 0;
    let pdf_50_page_intake_median_ms = median_command_millis(PDF_PAGES.min(3), || {
        let job_id = &pdf_job_ids[pdf_job_index];
        pdf_job_index += 1;
        run_job_import(&binary, &workspace_path, job_id, &pdf_path);
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
        capabilities_startup_median_ms,
        status_100_jobs_median_ms,
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
        "capabilities startup",
        metrics.capabilities_startup_median_ms,
        STARTUP_CAPABILITIES_LIMIT_MS,
    );
    enforce(
        "status for 100 jobs",
        metrics.status_100_jobs_median_ms,
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

fn prepare_large_workspace(workspace_path: &Path) -> (Vec<u64>, Vec<String>) {
    let mut workspace = Workspace::open_from(Some(workspace_path), workspace_path)
        .expect("open benchmark workspace");
    let mut jobs = JobService::new(&mut workspace.database, &workspace.blobs);
    for index in 0..LARGE_WORKSPACE_JOBS {
        jobs.create(
            &format!("Synthetic academic role {index:03}"),
            "Benchmark University",
            ActorKind::System,
        )
        .expect("large workspace job");
    }
    let html_job = jobs
        .create(
            "HTML intake benchmark",
            "Benchmark University",
            ActorKind::System,
        )
        .expect("HTML job");
    let pdf_job_ids = (0..3)
        .map(|index| {
            jobs.create(
                &format!("PDF intake benchmark {index}"),
                "Benchmark University",
                ActorKind::System,
            )
            .expect("PDF job")
            .id
            .to_string()
        })
        .collect::<Vec<_>>();

    let html = one_mib_html();
    let mut html_samples = Vec::new();
    for _ in 0..3 {
        let started = Instant::now();
        let normalized = normalize_html_document(&html).expect("normalize HTML benchmark fixture");
        jobs.import_source(
            &html_job.id,
            NewSource {
                kind: SourceKind::UserUrl,
                original_bytes: html.clone(),
                normalized_text: normalized,
                source_url: Some("https://jobs.example.edu/benchmark".to_owned()),
                final_url: Some("https://jobs.example.edu/benchmark".to_owned()),
                content_type: "text/html; charset=utf-8".to_owned(),
                redirect_chain: Vec::new(),
                privacy: PrivacyClassification::PrivateLocal,
            },
            ActorKind::System,
        )
        .expect("commit normalized HTML");
        html_samples.push(duration_millis(started.elapsed()));
    }
    (html_samples, pdf_job_ids)
}

fn one_mib_html() -> Vec<u8> {
    let mut html = String::from("<!doctype html><html><body><h1>Lecturer in Economics</h1>");
    while html.len() < 1_048_576 {
        html.push_str("<p>Teach economics, publish rigorous research, and support students.</p>");
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

fn run_job_import(binary: &Path, workspace: &Path, job_id: &str, file: &Path) -> Output {
    let mut command = Command::new(binary);
    command
        .arg("--workspace")
        .arg(workspace)
        .args(["job", "import"])
        .arg(job_id)
        .arg("--file")
        .arg(file)
        .arg("--json");
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
                "Lecturer in Economics benchmark fixture",
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
