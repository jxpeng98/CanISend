#![forbid(unsafe_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use canisend_app::{Application, ContentCatalogFilter, ContentSearchRequest, PrivateReadConsent};
use serde::Serialize;

const LARGE_FIXTURE_JOBS: usize = 128;
const SAMPLES: usize = 5;
const DOSSIER_LIST_LIMIT_MS: u128 = 2_000;
const INDEXED_SEARCH_LIMIT_MS: u128 = 1_000;

static NEXT: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "canisend-read-model-performance-{label}-{}-{}",
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
struct ReadModelPerformanceMetrics {
    format: &'static str,
    target: String,
    fixture_jobs: usize,
    catalog_entries: u64,
    dossier_list_median_ms: u128,
    dossier_list_max_ms: u128,
    indexed_search_median_ms: u128,
    indexed_search_max_ms: u128,
}

#[test]
#[ignore = "scheduled bounded large-fixture latency baseline"]
fn dossier_list_and_indexed_search_stay_within_local_latency_budgets() {
    let root = TestDirectory::new("workspace");
    let source = TestDirectory::new("source");
    fs::create_dir_all(source.path()).expect("create source fixture directory");
    let source_path = source.path().join("advert.md");
    fs::write(
        &source_path,
        "# Lecturer benchmark\n\nTeach, research, and support students.\n",
    )
    .expect("write source fixture");

    Application::initialize_workspace(root.path()).expect("initialize performance workspace");
    for index in 0..LARGE_FIXTURE_JOBS {
        let job = Application::create_job(
            root.path(),
            &format!("Lecturer benchmark {index:03}"),
            "Performance University",
        )
        .expect("create performance job")
        .data;
        Application::import_local_job_source(
            root.path(),
            job.id.as_str(),
            &source_path,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import performance source");
    }

    let dossier_count = Application::list_application_dossiers(root.path(), false)
        .expect("warm dossier list")
        .data
        .applications
        .len();
    assert_eq!(dossier_count, LARGE_FIXTURE_JOBS);
    let warm_search = Application::search_content(
        root.path(),
        ContentSearchRequest {
            query: "Lecturer".to_owned(),
            filter: ContentCatalogFilter::default(),
            include_private_bodies: false,
            limit: 100,
        },
        None,
    )
    .expect("warm indexed search")
    .data;
    assert!(warm_search.total_matches >= LARGE_FIXTURE_JOBS as u64);

    let dossier_samples = samples(|| {
        let dossiers = Application::list_application_dossiers(root.path(), false)
            .expect("measure dossier list")
            .data;
        assert_eq!(dossiers.applications.len(), LARGE_FIXTURE_JOBS);
    });
    let search_samples = samples(|| {
        let search = Application::search_content(
            root.path(),
            ContentSearchRequest {
                query: "Lecturer".to_owned(),
                filter: ContentCatalogFilter::default(),
                include_private_bodies: false,
                limit: 100,
            },
            None,
        )
        .expect("measure indexed search")
        .data;
        assert_eq!(search.index.private_body_entries, 0);
        assert!(search.total_matches >= LARGE_FIXTURE_JOBS as u64);
    });

    let metrics = ReadModelPerformanceMetrics {
        format: "canisend.read-model-performance/v1",
        target: format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        fixture_jobs: LARGE_FIXTURE_JOBS,
        catalog_entries: warm_search.index.metadata_entries,
        dossier_list_median_ms: median(&dossier_samples),
        dossier_list_max_ms: maximum(&dossier_samples),
        indexed_search_median_ms: median(&search_samples),
        indexed_search_max_ms: maximum(&search_samples),
    };

    assert!(
        metrics.dossier_list_max_ms <= DOSSIER_LIST_LIMIT_MS,
        "Dossier list exceeded {DOSSIER_LIST_LIMIT_MS} ms: {metrics:?}"
    );
    assert!(
        metrics.indexed_search_max_ms <= INDEXED_SEARCH_LIMIT_MS,
        "indexed search exceeded {INDEXED_SEARCH_LIMIT_MS} ms: {metrics:?}"
    );

    println!(
        "CANISEND_READ_MODEL_PERFORMANCE={}",
        serde_json::to_string(&metrics).expect("serialize read-model metrics")
    );
}

fn samples(mut operation: impl FnMut()) -> Vec<Duration> {
    (0..SAMPLES)
        .map(|_| {
            let started = Instant::now();
            operation();
            started.elapsed()
        })
        .collect()
}

fn median(samples: &[Duration]) -> u128 {
    let mut millis = samples.iter().map(Duration::as_millis).collect::<Vec<_>>();
    millis.sort_unstable();
    millis[millis.len() / 2]
}

fn maximum(samples: &[Duration]) -> u128 {
    samples.iter().map(Duration::as_millis).max().unwrap_or(0)
}
