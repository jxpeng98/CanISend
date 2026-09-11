use super::*;

#[derive(Debug, Subcommand)]
pub(super) enum LocalTaskCommand {
    /// List up to 100 recent worker tasks.
    List(ApplicationIdArgs),
    /// Create a worker task for the current application revision.
    Prepare {
        /// Application ID from `app list`.
        #[arg(short = 'a', long, value_name = "ID")]
        application: String,
        /// Current Application revision.
        #[arg(long, value_name = "REVISION")]
        expected_revision: u64,
    },
    /// Show task state, generation and lease details.
    Show(TaskIdArgs),
    /// Claim a task or renew an expired lease.
    Claim {
        #[command(flatten)]
        task: TaskIdArgs,
        /// Current generation from `local-task show`.
        #[arg(long, value_name = "GENERATION")]
        expected_generation: u64,
    },
    /// Save a worker result locally; does not submit an application.
    Submit {
        #[command(flatten)]
        lease: LeaseArgs,
        /// JSON result file to save under the current lease.
        #[arg(short = 'f', long, visible_alias = "file", value_name = "JSON_FILE")]
        candidate: PathBuf,
    },
    /// Cancel a claimed task; keep the application unchanged.
    Cancel(LeaseArgs),
    /// Read a saved result with private-read consent.
    CandidateShow {
        #[command(flatten)]
        task: TaskIdArgs,
        /// Allow reading this private task result.
        #[arg(long)]
        confirm_private_read: bool,
    },
}

#[derive(Debug, Args)]
pub(super) struct TaskIdArgs {
    /// Task ID from `local-task list`.
    #[arg(long, value_name = "ID")]
    task: String,
}

#[derive(Debug, Args)]
pub(super) struct LeaseArgs {
    #[command(flatten)]
    task: TaskIdArgs,
    /// Current generation from `local-task show`.
    #[arg(long, value_name = "GENERATION")]
    expected_generation: u64,
    /// Lease ID returned by `local-task claim`.
    #[arg(long, value_name = "ID")]
    lease: String,
}

impl LocalTaskCommand {
    fn operation(&self) -> &'static str {
        match self {
            Self::List(_) => "local-task.list",
            Self::Prepare { .. } => "local-task.prepare",
            Self::Show(_) => "local-task.show",
            Self::Claim { .. } => "local-task.claim",
            Self::Submit { .. } => "local-task.submit",
            Self::Cancel(_) => "local-task.cancel",
            Self::CandidateShow { .. } => "local-task.candidate.show",
        }
    }
}

pub(super) fn execute(
    workspace: Option<PathBuf>,
    command: LocalTaskCommand,
) -> CommandResult<CommandOutput> {
    let operation = command.operation();
    let root = app_adapter::workspace_root_v4(workspace, operation)?;
    if let LocalTaskCommand::List(arguments) = command {
        let receipt = Application::list_local_tasks_v4(&root, &arguments.application)
            .map_err(|error| app_adapter::failure(operation, error))?;
        return success(
            operation,
            &receipt.status,
            &receipt.data,
            std::iter::once(format!("Worker tasks: {}", receipt.data.len()))
                .chain(receipt.data.iter().map(|task| {
                    format!(
                        "{}  [{:?}; generation {}]",
                        task.id, task.state, task.generation
                    )
                }))
                .collect(),
        );
    }
    if let LocalTaskCommand::CandidateShow {
        task,
        confirm_private_read,
    } = command
    {
        let receipt = Application::local_task_candidate_v4(
            &root,
            &task.task,
            confirm_private_read.then(PrivateReadConsent::granted_by_user),
        )
        .map_err(|error| app_adapter::failure(operation, error))?;
        return success(
            operation,
            &receipt.status,
            &receipt.data,
            candidate_text(&receipt.data),
        );
    }
    let result = match command {
        LocalTaskCommand::Prepare {
            application,
            expected_revision,
            ..
        } => {
            let revision = Revision::try_new(expected_revision).map_err(|error| {
                app_adapter::failure(operation, ApplicationError::InvalidInput(error.to_string()))
            })?;
            Application::prepare_local_task_v4(&root, &application, revision)
        }
        LocalTaskCommand::Show(task) => Application::show_local_task_v4(&root, &task.task),
        LocalTaskCommand::Claim {
            task,
            expected_generation,
        } => Application::claim_local_task_v4(&root, &task.task, expected_generation),
        LocalTaskCommand::Submit { lease, candidate } => Application::submit_local_task_v4(
            &root,
            &lease.task.task,
            lease.expected_generation,
            &lease.lease,
            &candidate,
        ),
        LocalTaskCommand::Cancel(lease) => Application::cancel_local_task_v4(
            &root,
            &lease.task.task,
            lease.expected_generation,
            &lease.lease,
        ),
        LocalTaskCommand::CandidateShow { .. } | LocalTaskCommand::List(_) => {
            unreachable!("read handled above")
        }
    };
    let receipt = result.map_err(|error| app_adapter::failure(operation, error))?;
    success(
        operation,
        &receipt.status,
        &receipt.data,
        task_text(&receipt.data),
    )
}

fn task_text(task: &canisend_contracts::LocalTaskV4) -> Vec<String> {
    let mut lines = vec![
        format!("Task: {}", task.id),
        format!("State: {:?}; generation: {}", task.state, task.generation),
    ];
    if let Some(lease) = &task.lease_id {
        lines.push(format!("Lease: {lease}"));
    }
    if let Some(expires) = &task.lease_expires_at {
        lines.push(format!("Lease expires: {expires}"));
    }
    if let Some(bytes) = task.candidate_bytes {
        lines.push(format!("Saved result: {bytes} bytes"));
    }
    lines
}

// Worker results are arbitrary JSON; render strings as text without JSON escaping.
fn candidate_text(value: &Value) -> Vec<String> {
    match value {
        Value::Object(fields) if !fields.is_empty() => fields
            .iter()
            .flat_map(|(key, value)| {
                std::iter::once(format!("{}:", key.replace('_', " "))).chain(candidate_text(value))
            })
            .collect(),
        Value::Array(values) if !values.is_empty() => values
            .iter()
            .enumerate()
            .flat_map(|(index, value)| {
                std::iter::once(format!("{}.", index + 1)).chain(candidate_text(value))
            })
            .collect(),
        Value::String(text) => vec![text.clone()],
        value => vec![value.to_string()],
    }
}
