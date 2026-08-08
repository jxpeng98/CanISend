use std::{
    collections::{BTreeMap, btree_map::Entry},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use canisend_contracts::{
    ApplicationPackBindingV3, EntityId, Revision, Sha256Digest, WorkflowPackId,
    WorkflowPackManifest,
};
use canisend_store::StoreError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    Application, ApplicationError, built_in_academic_job_pack, built_in_generic_application_pack,
};

pub const APPROVAL_DEFAULT_CAPACITY: usize = 16;
pub const APPROVAL_DEFAULT_TTL: Duration = Duration::from_secs(10 * 60);
const APPROVAL_DEFAULT_SWEEP_INTERVAL: Duration = Duration::from_secs(30);
const APPROVAL_TOKEN_BYTES: usize = 32;
const APPROVAL_TOKEN_PREFIX: &str = "apv1_";
const APPROVAL_TOKEN_GENERATION_ATTEMPTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalKind {
    ApplicationApproval,
    ApplicationIntake,
    DiscoveryImport,
    DiscoveryRefresh,
    JobIntake,
    TaskCompletion,
    WorkflowRerun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
pub enum ApprovalSourceVersion {
    Revision(Revision),
    Snapshot(Sha256Digest),
    RevisionAndSnapshot {
        revision: Revision,
        snapshot_sha256: Sha256Digest,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalScope {
    pub workspace: PathBuf,
    pub workspace_id: EntityId,
    pub pack: ApplicationPackBindingV3,
}

impl ApprovalScope {
    pub fn for_workspace_pack(
        workspace: &Path,
        pack_id: &WorkflowPackId,
    ) -> Result<Self, ApplicationError> {
        let status = Application::workspace_status_v4(workspace)?.data;
        let pack = match pack_id.as_str() {
            crate::ACADEMIC_JOB_WORKFLOW_PACK_ID => built_in_academic_job_pack()?,
            crate::GENERIC_APPLICATION_WORKFLOW_PACK_ID => built_in_generic_application_pack()?,
            unknown => {
                return Err(ApplicationError::InvalidInput(format!(
                    "approval scope cannot resolve unknown workflow Pack {unknown}"
                )));
            }
        };
        Ok(Self {
            workspace: status.path,
            workspace_id: status.status.workspace_id,
            pack: pack_binding(pack.manifest()),
        })
    }

    pub fn for_workspace(workspace: &Path) -> Result<Self, ApplicationError> {
        let status = Application::workspace_status(workspace)?.data;
        let pack = match status.pack_id.as_str() {
            crate::ACADEMIC_JOB_WORKFLOW_PACK_ID => built_in_academic_job_pack()?,
            crate::GENERIC_APPLICATION_WORKFLOW_PACK_ID => built_in_generic_application_pack()?,
            pack_id => {
                return Err(ApplicationError::InvalidInput(format!(
                    "approval scope cannot resolve unknown workflow Pack {pack_id}"
                )));
            }
        };
        Ok(Self {
            workspace: status.path,
            workspace_id: status.status.workspace_id,
            pack: pack_binding(pack.manifest()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalBinding {
    pub kind: ApprovalKind,
    pub scope: ApprovalScope,
    pub application_id: Option<String>,
    pub source: ApprovalSourceVersion,
}

impl ApprovalBinding {
    #[must_use]
    pub fn new(
        kind: ApprovalKind,
        scope: ApprovalScope,
        application_id: Option<String>,
        source: ApprovalSourceVersion,
    ) -> Self {
        Self {
            kind,
            scope,
            application_id,
            source,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalLease {
    pub token: String,
    pub expires_at_unix_ms: u64,
    pub remaining_ttl_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDisposition {
    Consume,
    RestoreSameApproval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalBrokerConfig {
    pub capacity: usize,
    pub ttl: Duration,
    pub sweep_interval: Duration,
}

impl Default for ApprovalBrokerConfig {
    fn default() -> Self {
        Self {
            capacity: APPROVAL_DEFAULT_CAPACITY,
            ttl: APPROVAL_DEFAULT_TTL,
            sweep_interval: APPROVAL_DEFAULT_SWEEP_INTERVAL,
        }
    }
}

pub trait ApprovalClock: Send + Sync + 'static {
    fn monotonic_now(&self) -> Duration;
    fn wall_now(&self) -> SystemTime;
}

#[derive(Debug)]
pub struct SystemApprovalClock {
    monotonic_origin: Instant,
}

impl Default for SystemApprovalClock {
    fn default() -> Self {
        Self {
            monotonic_origin: Instant::now(),
        }
    }
}

impl ApprovalClock for SystemApprovalClock {
    fn monotonic_now(&self) -> Duration {
        self.monotonic_origin.elapsed()
    }

    fn wall_now(&self) -> SystemTime {
        SystemTime::now()
    }
}

pub trait ApprovalTokenSource: Send + Sync + 'static {
    fn fill(&self, destination: &mut [u8]) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct SystemApprovalTokenSource;

impl ApprovalTokenSource for SystemApprovalTokenSource {
    fn fill(&self, destination: &mut [u8]) -> Result<(), String> {
        getrandom::fill(destination).map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ApprovalBrokerError {
    #[error("approval broker configuration is invalid: {0}")]
    InvalidConfiguration(String),
    #[error("approval state is unavailable")]
    Unavailable,
    #[error("approval token generation failed: {0}")]
    TokenGeneration(String),
    #[error("approval token generation repeatedly collided with a live token")]
    TokenCollision,
    #[error("approval capacity of {capacity} live entries is full")]
    CapacityFull { capacity: usize },
    #[error("approval token is malformed")]
    MalformedToken,
    #[error("approval is missing or was already consumed")]
    Missing,
    #[error("approval expired")]
    Expired,
    #[error("approval belongs to {actual:?}, not {expected:?}")]
    WrongKind {
        expected: ApprovalKind,
        actual: ApprovalKind,
    },
    #[error("approval belongs to a different Workspace or workflow Pack")]
    WrongContext,
    #[error("approval cannot be restored because its token is already live")]
    RestoreCollision,
}

struct ApprovalEntry<T> {
    binding: ApprovalBinding,
    payload: T,
    expires_monotonic: Duration,
    expires_at_unix_ms: u64,
}

struct ApprovalState<T> {
    entries: BTreeMap<String, ApprovalEntry<T>>,
    in_flight: BTreeMap<String, Duration>,
}

impl<T> Default for ApprovalState<T> {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            in_flight: BTreeMap::new(),
        }
    }
}

struct ApprovalBrokerInner<T> {
    state: Mutex<ApprovalState<T>>,
    clock: Arc<dyn ApprovalClock>,
    token_source: Arc<dyn ApprovalTokenSource>,
    config: ApprovalBrokerConfig,
}

pub struct ApprovalBroker<T> {
    inner: Arc<ApprovalBrokerInner<T>>,
}

impl<T> Clone for ApprovalBroker<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> std::fmt::Debug for ApprovalBroker<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApprovalBroker")
            .field("config", &self.inner.config)
            .finish_non_exhaustive()
    }
}

pub struct ApprovalGrant<T> {
    owner: Weak<ApprovalBrokerInner<T>>,
    token: String,
    binding: ApprovalBinding,
    payload: T,
    expires_monotonic: Duration,
    expires_at_unix_ms: u64,
}

impl<T> ApprovalGrant<T> {
    #[must_use]
    pub fn binding(&self) -> &ApprovalBinding {
        &self.binding
    }

    #[must_use]
    pub fn payload(&self) -> &T {
        &self.payload
    }

    #[must_use]
    pub fn into_payload(self) -> T {
        self.payload
    }
}

impl<T: Send + 'static> Default for ApprovalBroker<T> {
    fn default() -> Self {
        Self::new(ApprovalBrokerConfig::default()).expect("default approval broker is valid")
    }
}

impl<T: Send + 'static> ApprovalBroker<T> {
    pub fn new(config: ApprovalBrokerConfig) -> Result<Self, ApprovalBrokerError> {
        Self::with_components(
            config,
            Arc::new(SystemApprovalClock::default()),
            Arc::new(SystemApprovalTokenSource),
        )
    }

    pub fn with_components(
        config: ApprovalBrokerConfig,
        clock: Arc<dyn ApprovalClock>,
        token_source: Arc<dyn ApprovalTokenSource>,
    ) -> Result<Self, ApprovalBrokerError> {
        validate_config(&config)?;
        let broker = Self {
            inner: Arc::new(ApprovalBrokerInner {
                state: Mutex::new(ApprovalState::default()),
                clock,
                token_source,
                config,
            }),
        };
        broker.start_sweeper()?;
        Ok(broker)
    }

    pub fn insert(
        &self,
        binding: ApprovalBinding,
        payload: T,
    ) -> Result<ApprovalLease, ApprovalBrokerError> {
        let now = self.inner.clock.monotonic_now();
        let expires_monotonic = now.checked_add(self.inner.config.ttl).ok_or_else(|| {
            ApprovalBrokerError::InvalidConfiguration("TTL overflows monotonic time".to_owned())
        })?;
        let expires_wall = self
            .inner
            .clock
            .wall_now()
            .checked_add(self.inner.config.ttl)
            .ok_or_else(|| {
                ApprovalBrokerError::InvalidConfiguration("TTL overflows wall time".to_owned())
            })?;
        let expires_at_unix_ms = unix_milliseconds(expires_wall)?;
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| ApprovalBrokerError::Unavailable)?;
        sweep_state(&mut state, now);
        if retained_len(&state) >= self.inner.config.capacity {
            return Err(ApprovalBrokerError::CapacityFull {
                capacity: self.inner.config.capacity,
            });
        }
        let token = self.generate_unique_token(&state)?;
        match state.entries.entry(token.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(ApprovalEntry {
                    binding,
                    payload,
                    expires_monotonic,
                    expires_at_unix_ms,
                });
            }
            Entry::Occupied(_) => return Err(ApprovalBrokerError::TokenCollision),
        }
        Ok(ApprovalLease {
            token,
            expires_at_unix_ms,
            remaining_ttl_seconds: self.inner.config.ttl.as_secs(),
        })
    }

    pub fn take(
        &self,
        token: &str,
        expected_kind: ApprovalKind,
        expected_scope: &ApprovalScope,
    ) -> Result<ApprovalGrant<T>, ApprovalBrokerError> {
        validate_token(token)?;
        let now = self.inner.clock.monotonic_now();
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| ApprovalBrokerError::Unavailable)?;
        let Some(entry) = state.entries.remove(token) else {
            return Err(ApprovalBrokerError::Missing);
        };
        if now >= entry.expires_monotonic {
            return Err(ApprovalBrokerError::Expired);
        }
        if entry.binding.kind != expected_kind {
            return Err(ApprovalBrokerError::WrongKind {
                expected: expected_kind,
                actual: entry.binding.kind,
            });
        }
        if &entry.binding.scope != expected_scope {
            return Err(ApprovalBrokerError::WrongContext);
        }
        state
            .in_flight
            .insert(token.to_owned(), entry.expires_monotonic);
        Ok(ApprovalGrant {
            owner: Arc::downgrade(&self.inner),
            token: token.to_owned(),
            binding: entry.binding,
            payload: entry.payload,
            expires_monotonic: entry.expires_monotonic,
            expires_at_unix_ms: entry.expires_at_unix_ms,
        })
    }

    pub fn resolve(
        &self,
        grant: ApprovalGrant<T>,
        disposition: ApprovalDisposition,
    ) -> Result<(), ApprovalBrokerError> {
        if !Weak::ptr_eq(&grant.owner, &Arc::downgrade(&self.inner)) {
            return Err(ApprovalBrokerError::WrongContext);
        }
        let now = self.inner.clock.monotonic_now();
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| ApprovalBrokerError::Unavailable)?;
        sweep_state(&mut state, now);
        if now >= grant.expires_monotonic {
            state.in_flight.remove(&grant.token);
            return Err(ApprovalBrokerError::Expired);
        }
        let Some(reserved_expiry) = state.in_flight.remove(&grant.token) else {
            return Err(ApprovalBrokerError::Missing);
        };
        if reserved_expiry != grant.expires_monotonic {
            return Err(ApprovalBrokerError::WrongContext);
        }
        if disposition == ApprovalDisposition::Consume {
            return Ok(());
        }
        match state.entries.entry(grant.token) {
            Entry::Vacant(entry) => {
                entry.insert(ApprovalEntry {
                    binding: grant.binding,
                    payload: grant.payload,
                    expires_monotonic: grant.expires_monotonic,
                    expires_at_unix_ms: grant.expires_at_unix_ms,
                });
                Ok(())
            }
            Entry::Occupied(_) => Err(ApprovalBrokerError::RestoreCollision),
        }
    }

    pub fn discard(
        &self,
        token: &str,
        expected_kind: ApprovalKind,
        expected_scope: &ApprovalScope,
    ) -> Result<(), ApprovalBrokerError> {
        let grant = self.take(token, expected_kind, expected_scope)?;
        self.resolve(grant, ApprovalDisposition::Consume)
    }

    pub fn sweep_expired(&self) -> Result<usize, ApprovalBrokerError> {
        let now = self.inner.clock.monotonic_now();
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| ApprovalBrokerError::Unavailable)?;
        Ok(sweep_state(&mut state, now))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.inner
            .state
            .lock()
            .map(|state| retained_len(&state))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn generate_unique_token(
        &self,
        state: &ApprovalState<T>,
    ) -> Result<String, ApprovalBrokerError> {
        for _ in 0..APPROVAL_TOKEN_GENERATION_ATTEMPTS {
            let mut bytes = [0_u8; APPROVAL_TOKEN_BYTES];
            self.inner
                .token_source
                .fill(&mut bytes)
                .map_err(ApprovalBrokerError::TokenGeneration)?;
            let token = format!("{APPROVAL_TOKEN_PREFIX}{}", hex::encode(bytes));
            if !state.entries.contains_key(&token) && !state.in_flight.contains_key(&token) {
                return Ok(token);
            }
        }
        Err(ApprovalBrokerError::TokenCollision)
    }

    fn start_sweeper(&self) -> Result<(), ApprovalBrokerError> {
        let weak = Arc::downgrade(&self.inner);
        let interval = self.inner.config.sweep_interval;
        thread::Builder::new()
            .name("canisend-approval-sweep".to_owned())
            .spawn(move || {
                loop {
                    thread::sleep(interval);
                    let Some(inner) = weak.upgrade() else {
                        break;
                    };
                    let now = inner.clock.monotonic_now();
                    if let Ok(mut state) = inner.state.lock() {
                        sweep_state(&mut state, now);
                    }
                }
            })
            .map(|_| ())
            .map_err(|_| ApprovalBrokerError::Unavailable)
    }
}

#[must_use]
pub fn approval_disposition_for_application_error(error: &ApplicationError) -> ApprovalDisposition {
    match error {
        ApplicationError::Store(error) if store_error_is_transient(error) => {
            ApprovalDisposition::RestoreSameApproval
        }
        ApplicationError::Input(canisend_io::IoAdapterError::Io { source, .. })
            if io_error_is_transient(source) =>
        {
            ApprovalDisposition::RestoreSameApproval
        }
        _ => ApprovalDisposition::Consume,
    }
}

fn store_error_is_transient(error: &StoreError) -> bool {
    error.is_transient_operation_failure()
}

fn io_error_is_transient(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
    )
}

fn validate_config(config: &ApprovalBrokerConfig) -> Result<(), ApprovalBrokerError> {
    if config.capacity == 0 {
        return Err(ApprovalBrokerError::InvalidConfiguration(
            "capacity must be greater than zero".to_owned(),
        ));
    }
    if config.ttl.is_zero() {
        return Err(ApprovalBrokerError::InvalidConfiguration(
            "TTL must be greater than zero".to_owned(),
        ));
    }
    if config.sweep_interval.is_zero() {
        return Err(ApprovalBrokerError::InvalidConfiguration(
            "sweep interval must be greater than zero".to_owned(),
        ));
    }
    Ok(())
}

fn sweep_state<T>(state: &mut ApprovalState<T>, now: Duration) -> usize {
    let before = retained_len(state);
    state
        .entries
        .retain(|_, entry| now < entry.expires_monotonic);
    state
        .in_flight
        .retain(|_, expires_monotonic| now < *expires_monotonic);
    before - retained_len(state)
}

fn retained_len<T>(state: &ApprovalState<T>) -> usize {
    state.entries.len() + state.in_flight.len()
}

fn validate_token(token: &str) -> Result<(), ApprovalBrokerError> {
    let encoded = token
        .strip_prefix(APPROVAL_TOKEN_PREFIX)
        .ok_or(ApprovalBrokerError::MalformedToken)?;
    if encoded.len() != APPROVAL_TOKEN_BYTES * 2
        || !encoded.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ApprovalBrokerError::MalformedToken);
    }
    Ok(())
}

fn unix_milliseconds(time: SystemTime) -> Result<u64, ApprovalBrokerError> {
    let milliseconds = time
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            ApprovalBrokerError::InvalidConfiguration(
                "wall clock is before the Unix epoch".to_owned(),
            )
        })?
        .as_millis();
    u64::try_from(milliseconds).map_err(|_| {
        ApprovalBrokerError::InvalidConfiguration("wall-clock milliseconds overflow u64".to_owned())
    })
}

fn pack_binding(manifest: &WorkflowPackManifest) -> ApplicationPackBindingV3 {
    ApplicationPackBindingV3 {
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        content_digest: manifest.content_digest.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        path::PathBuf,
        sync::{
            Condvar,
            atomic::{AtomicU64, Ordering},
        },
    };

    use canisend_contracts::{ApplicationPackBindingV3, SemanticVersion, WorkflowPackId};

    use super::*;

    #[derive(Debug)]
    struct ManualClock {
        monotonic_millis: AtomicU64,
        wall_millis: AtomicU64,
    }

    impl ManualClock {
        fn new() -> Self {
            Self {
                monotonic_millis: AtomicU64::new(0),
                wall_millis: AtomicU64::new(1_800_000_000_000),
            }
        }

        fn advance_monotonic(&self, duration: Duration) {
            self.monotonic_millis.fetch_add(
                u64::try_from(duration.as_millis()).expect("test duration"),
                Ordering::SeqCst,
            );
        }

        fn set_wall(&self, milliseconds: u64) {
            self.wall_millis.store(milliseconds, Ordering::SeqCst);
        }
    }

    impl ApprovalClock for ManualClock {
        fn monotonic_now(&self) -> Duration {
            Duration::from_millis(self.monotonic_millis.load(Ordering::SeqCst))
        }

        fn wall_now(&self) -> SystemTime {
            UNIX_EPOCH + Duration::from_millis(self.wall_millis.load(Ordering::SeqCst))
        }
    }

    #[derive(Debug)]
    struct QueueTokenSource {
        values: Mutex<VecDeque<[u8; APPROVAL_TOKEN_BYTES]>>,
    }

    impl QueueTokenSource {
        fn new(values: impl IntoIterator<Item = [u8; APPROVAL_TOKEN_BYTES]>) -> Self {
            Self {
                values: Mutex::new(values.into_iter().collect()),
            }
        }
    }

    impl ApprovalTokenSource for QueueTokenSource {
        fn fill(&self, destination: &mut [u8]) -> Result<(), String> {
            let value = self
                .values
                .lock()
                .map_err(|_| "token queue poisoned".to_owned())?
                .pop_front()
                .ok_or_else(|| "token queue exhausted".to_owned())?;
            destination.copy_from_slice(&value);
            Ok(())
        }
    }

    fn scope(label: &str) -> ApprovalScope {
        ApprovalScope {
            workspace: PathBuf::from(format!("/{label}")),
            workspace_id: EntityId::try_new(format!("01900000-0000-7000-8000-{label:0>12}"))
                .expect("workspace ID"),
            pack: ApplicationPackBindingV3 {
                id: WorkflowPackId::try_new("org.canisend.generic-application").expect("Pack ID"),
                version: SemanticVersion::try_new("1.0.0").expect("Pack version"),
                content_digest: Sha256Digest::try_new("a".repeat(64)).expect("Pack digest"),
            },
        }
    }

    fn binding(label: &str, kind: ApprovalKind) -> ApprovalBinding {
        ApprovalBinding::new(
            kind,
            scope(label),
            Some(format!("application-{label}")),
            ApprovalSourceVersion::Revision(
                Revision::try_new(1).expect("positive source revision"),
            ),
        )
    }

    fn broker<T: Send + 'static>(
        capacity: usize,
        ttl: Duration,
        sweep_interval: Duration,
        clock: Arc<ManualClock>,
        values: impl IntoIterator<Item = [u8; APPROVAL_TOKEN_BYTES]>,
    ) -> ApprovalBroker<T> {
        ApprovalBroker::with_components(
            ApprovalBrokerConfig {
                capacity,
                ttl,
                sweep_interval,
            },
            clock,
            Arc::new(QueueTokenSource::new(values)),
        )
        .expect("test broker")
    }

    #[test]
    fn ttl_uses_monotonic_time_and_wall_clock_only_describes_expiry() {
        let clock = Arc::new(ManualClock::new());
        let broker = broker(
            2,
            Duration::from_secs(600),
            Duration::from_secs(60),
            Arc::clone(&clock),
            [[1; APPROVAL_TOKEN_BYTES]],
        );
        let expected_scope = scope("1");
        let lease = broker
            .insert(binding("1", ApprovalKind::JobIntake), "private payload")
            .expect("insert");
        assert_eq!(lease.remaining_ttl_seconds, 600);
        assert_eq!(lease.expires_at_unix_ms, 1_800_000_600_000);

        clock.set_wall(1_700_000_000_000);
        clock.advance_monotonic(Duration::from_secs(601));
        assert!(matches!(
            broker.take(&lease.token, ApprovalKind::JobIntake, &expected_scope),
            Err(ApprovalBrokerError::Expired)
        ));
        assert_eq!(broker.len(), 0);
    }

    #[test]
    fn collision_never_replaces_payload_and_capacity_never_evicts_live_entry() {
        let clock = Arc::new(ManualClock::new());
        let broker = broker(
            2,
            Duration::from_secs(600),
            Duration::from_secs(60),
            clock,
            [
                [7; APPROVAL_TOKEN_BYTES],
                [7; APPROVAL_TOKEN_BYTES],
                [8; APPROVAL_TOKEN_BYTES],
            ],
        );
        let expected_scope = scope("2");
        let first = broker
            .insert(binding("2", ApprovalKind::TaskCompletion), "first")
            .expect("first");
        let second = broker
            .insert(binding("2", ApprovalKind::TaskCompletion), "second")
            .expect("collision retries with new token");
        assert_ne!(first.token, second.token);
        assert_eq!(
            broker.insert(binding("2", ApprovalKind::TaskCompletion), "third"),
            Err(ApprovalBrokerError::CapacityFull { capacity: 2 })
        );
        assert_eq!(
            broker
                .take(&first.token, ApprovalKind::TaskCompletion, &expected_scope)
                .expect("original survives")
                .into_payload(),
            "first"
        );
    }

    #[test]
    fn wrong_kind_context_replay_and_expiry_consume_without_mutation() {
        let clock = Arc::new(ManualClock::new());
        let broker = broker(
            4,
            Duration::from_secs(600),
            Duration::from_secs(60),
            Arc::clone(&clock),
            [
                [1; APPROVAL_TOKEN_BYTES],
                [2; APPROVAL_TOKEN_BYTES],
                [3; APPROVAL_TOKEN_BYTES],
                [4; APPROVAL_TOKEN_BYTES],
            ],
        );
        let right_scope = scope("3");
        let wrong_scope = scope("4");

        let wrong_kind = broker
            .insert(binding("3", ApprovalKind::JobIntake), 1_u8)
            .expect("wrong-kind fixture");
        assert!(matches!(
            broker.take(
                &wrong_kind.token,
                ApprovalKind::TaskCompletion,
                &right_scope
            ),
            Err(ApprovalBrokerError::WrongKind { .. })
        ));
        assert!(matches!(
            broker.take(&wrong_kind.token, ApprovalKind::JobIntake, &right_scope),
            Err(ApprovalBrokerError::Missing)
        ));

        let wrong_context = broker
            .insert(binding("3", ApprovalKind::JobIntake), 2_u8)
            .expect("wrong-context fixture");
        assert!(matches!(
            broker.take(&wrong_context.token, ApprovalKind::JobIntake, &wrong_scope),
            Err(ApprovalBrokerError::WrongContext)
        ));
        assert_eq!(broker.len(), 0);

        let wrong_pack = broker
            .insert(binding("3", ApprovalKind::JobIntake), 3_u8)
            .expect("wrong-Pack fixture");
        let mut wrong_pack_scope = right_scope.clone();
        wrong_pack_scope.pack.content_digest =
            Sha256Digest::try_new("b".repeat(64)).expect("different Pack digest");
        assert!(matches!(
            broker.take(
                &wrong_pack.token,
                ApprovalKind::JobIntake,
                &wrong_pack_scope
            ),
            Err(ApprovalBrokerError::WrongContext)
        ));
        assert!(matches!(
            broker.take(&wrong_pack.token, ApprovalKind::JobIntake, &right_scope),
            Err(ApprovalBrokerError::Missing)
        ));

        let expired = broker
            .insert(binding("3", ApprovalKind::JobIntake), 4_u8)
            .expect("expiry fixture");
        clock.advance_monotonic(Duration::from_secs(601));
        assert!(matches!(
            broker.take(&expired.token, ApprovalKind::JobIntake, &right_scope),
            Err(ApprovalBrokerError::Expired)
        ));
    }

    #[test]
    fn same_approval_restore_preserves_deadline_and_only_one_commit_wins() {
        let clock = Arc::new(ManualClock::new());
        let restored_broker = broker(
            1,
            Duration::from_secs(600),
            Duration::from_secs(60),
            Arc::clone(&clock),
            [[5; APPROVAL_TOKEN_BYTES]],
        );
        let expected_scope = scope("5");
        let lease = restored_broker
            .insert(binding("5", ApprovalKind::ApplicationApproval), 42_u8)
            .expect("insert");
        let grant = restored_broker
            .take(
                &lease.token,
                ApprovalKind::ApplicationApproval,
                &expected_scope,
            )
            .expect("take");
        assert_eq!(
            restored_broker.insert(binding("5", ApprovalKind::ApplicationApproval), 99_u8),
            Err(ApprovalBrokerError::CapacityFull { capacity: 1 }),
            "an in-flight grant keeps its bounded broker slot"
        );
        clock.advance_monotonic(Duration::from_secs(590));
        restored_broker
            .resolve(grant, ApprovalDisposition::RestoreSameApproval)
            .expect("restore");
        clock.advance_monotonic(Duration::from_secs(11));
        assert!(matches!(
            restored_broker.take(
                &lease.token,
                ApprovalKind::ApplicationApproval,
                &expected_scope
            ),
            Err(ApprovalBrokerError::Expired)
        ));

        let clock = Arc::new(ManualClock::new());
        let concurrent_broker = broker(
            2,
            Duration::from_secs(600),
            Duration::from_secs(60),
            clock,
            [[6; APPROVAL_TOKEN_BYTES]],
        );
        let lease = concurrent_broker
            .insert(binding("5", ApprovalKind::ApplicationApproval), 7_u8)
            .expect("insert concurrent fixture");
        let barrier = Arc::new((Mutex::new(0_u8), Condvar::new()));
        let mut threads = Vec::new();
        for _ in 0..2 {
            let broker = concurrent_broker.clone();
            let scope = expected_scope.clone();
            let token = lease.token.clone();
            let barrier = Arc::clone(&barrier);
            threads.push(thread::spawn(move || {
                let (lock, signal) = &*barrier;
                let mut ready = lock.lock().expect("barrier");
                *ready += 1;
                signal.notify_all();
                while *ready < 2 {
                    ready = signal.wait(ready).expect("barrier wait");
                }
                broker.take(&token, ApprovalKind::ApplicationApproval, &scope)
            }));
        }
        let successes = threads
            .into_iter()
            .map(|thread| usize::from(thread.join().expect("join").is_ok()))
            .sum::<usize>();
        assert_eq!(successes, 1);
    }

    #[test]
    fn deterministic_and_idle_sweeps_release_expired_payloads() {
        let clock = Arc::new(ManualClock::new());
        let idle_broker = broker(
            2,
            Duration::from_millis(10),
            Duration::from_millis(5),
            Arc::clone(&clock),
            [[8; APPROVAL_TOKEN_BYTES]],
        );
        idle_broker
            .insert(
                binding("6", ApprovalKind::DiscoveryImport),
                vec![9_u8; 1024],
            )
            .expect("insert private payload");
        clock.advance_monotonic(Duration::from_millis(11));
        for _ in 0..50 {
            if idle_broker.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            idle_broker.len(),
            0,
            "idle sweeper releases expired payload"
        );

        let deterministic_broker = broker(
            2,
            Duration::from_millis(10),
            Duration::from_secs(60),
            Arc::clone(&clock),
            [[9; APPROVAL_TOKEN_BYTES]],
        );
        deterministic_broker
            .insert(binding("6", ApprovalKind::DiscoveryRefresh), 1_u8)
            .expect("insert deterministic fixture");
        clock.advance_monotonic(Duration::from_millis(11));
        assert_eq!(deterministic_broker.sweep_expired().expect("sweep"), 1);
    }

    #[test]
    fn expired_entries_are_swept_before_capacity_and_capacity_race_is_bounded() {
        let clock = Arc::new(ManualClock::new());
        let expiry_broker = broker(
            1,
            Duration::from_millis(10),
            Duration::from_secs(60),
            Arc::clone(&clock),
            [[10; APPROVAL_TOKEN_BYTES], [11; APPROVAL_TOKEN_BYTES]],
        );
        expiry_broker
            .insert(binding("7", ApprovalKind::JobIntake), 1_u8)
            .expect("first entry");
        clock.advance_monotonic(Duration::from_millis(11));
        expiry_broker
            .insert(binding("7", ApprovalKind::JobIntake), 2_u8)
            .expect("expired entry is swept before capacity decision");
        assert_eq!(expiry_broker.len(), 1);

        let clock = Arc::new(ManualClock::new());
        let broker = broker(
            1,
            Duration::from_secs(600),
            Duration::from_secs(60),
            clock,
            [[12; APPROVAL_TOKEN_BYTES], [13; APPROVAL_TOKEN_BYTES]],
        );
        let barrier = Arc::new((Mutex::new(0_u8), Condvar::new()));
        let mut threads = Vec::new();
        for payload in [3_u8, 4_u8] {
            let broker = broker.clone();
            let barrier = Arc::clone(&barrier);
            threads.push(thread::spawn(move || {
                let (lock, signal) = &*barrier;
                let mut ready = lock.lock().expect("barrier");
                *ready += 1;
                signal.notify_all();
                while *ready < 2 {
                    ready = signal.wait(ready).expect("barrier wait");
                }
                broker.insert(binding("8", ApprovalKind::TaskCompletion), payload)
            }));
        }
        let results = threads
            .into_iter()
            .map(|thread| thread.join().expect("join"))
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| {
                    matches!(
                        result,
                        Err(ApprovalBrokerError::CapacityFull { capacity: 1 })
                    )
                })
                .count(),
            1
        );
        assert_eq!(broker.len(), 1);
    }

    #[test]
    fn malformed_tokens_and_process_restart_do_not_recover_payloads() {
        let clock = Arc::new(ManualClock::new());
        let first_process = broker(
            2,
            Duration::from_secs(600),
            Duration::from_secs(60),
            Arc::clone(&clock),
            [[14; APPROVAL_TOKEN_BYTES]],
        );
        let lease = first_process
            .insert(binding("9", ApprovalKind::ApplicationApproval), 1_u8)
            .expect("first process token");
        assert!(matches!(
            first_process.take(
                "timestamp-token",
                ApprovalKind::ApplicationApproval,
                &scope("9")
            ),
            Err(ApprovalBrokerError::MalformedToken)
        ));

        let restarted_process = broker::<u8>(
            2,
            Duration::from_secs(600),
            Duration::from_secs(60),
            clock,
            [[14; APPROVAL_TOKEN_BYTES]],
        );
        assert!(matches!(
            restarted_process.take(&lease.token, ApprovalKind::ApplicationApproval, &scope("9")),
            Err(ApprovalBrokerError::Missing)
        ));
        assert_eq!(first_process.len(), 1);
    }

    #[test]
    fn disposition_is_explicit_and_does_not_follow_retryable_flag() {
        let stale = ApplicationError::Store(StoreError::TaskStale("changed".to_owned()));
        assert!(
            stale.classify().retryable,
            "legacy transport classification remains retryable"
        );
        assert_eq!(
            approval_disposition_for_application_error(&stale),
            ApprovalDisposition::Consume
        );

        let validation = ApplicationError::Store(StoreError::CandidateStructural(Vec::new()));
        assert!(
            validation.classify().retryable,
            "legacy validation transport classification remains retryable"
        );
        assert_eq!(
            approval_disposition_for_application_error(&validation),
            ApprovalDisposition::Consume
        );

        let transient = ApplicationError::Store(StoreError::Io {
            path: PathBuf::from("workspace.db"),
            source: std::io::Error::from(std::io::ErrorKind::WouldBlock),
        });
        assert_eq!(
            approval_disposition_for_application_error(&transient),
            ApprovalDisposition::RestoreSameApproval
        );
        let permanent = ApplicationError::Store(StoreError::Io {
            path: PathBuf::from("workspace.db"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        });
        assert_eq!(
            approval_disposition_for_application_error(&permanent),
            ApprovalDisposition::Consume
        );

        let busy = ApplicationError::Store(StoreError::Sqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some("database busy".to_owned()),
        )));
        assert_eq!(
            approval_disposition_for_application_error(&busy),
            ApprovalDisposition::RestoreSameApproval
        );
    }
}
