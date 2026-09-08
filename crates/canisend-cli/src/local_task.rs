use super::*;

#[derive(Debug, Subcommand)]
pub(super) enum LocalTaskCommand {
    /// List up to 100 recent task records for an Application without candidate bodies.
    List(ApplicationIdArgs),
    /// Prepare coordination for an exact Application revision; this does not authorize a write.
    Prepare {
        #[arg(long)]
        application: String,
        #[arg(long)]
        expected_revision: u64,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Read task metadata without exposing the candidate body.
    Show(TaskIdArgs),
    /// Claim a prepared task or reclaim an expired lease atomically.
    Claim {
        #[command(flatten)]
        task: TaskIdArgs,
        #[arg(long)]
        expected_generation: u64,
    },
    /// Store a bounded, untrusted JSON candidate under the exact live lease.
    Submit {
        #[command(flatten)]
        lease: LeaseArgs,
        #[arg(long)]
        candidate: PathBuf,
    },
    /// Abandon a claimed task without changing the Application.
    Cancel(LeaseArgs),
    /// Read private candidate bytes for a subsequent independently approved preview.
    CandidateShow {
        #[command(flatten)]
        task: TaskIdArgs,
        #[arg(long)]
        confirm_private_read: bool,
    },
}

#[derive(Debug, Args)]
pub(super) struct TaskIdArgs {
    #[arg(long)]
    task: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct LeaseArgs {
    #[command(flatten)]
    task: TaskIdArgs,
    #[arg(long)]
    expected_generation: u64,
    #[arg(long)]
    lease: String,
}

impl LocalTaskCommand {
    pub(super) fn json(&self) -> bool {
        match self {
            Self::List(arguments) => arguments.output.json,
            Self::Prepare { output, .. } => output.json,
            Self::Show(task) | Self::Claim { task, .. } | Self::CandidateShow { task, .. } => {
                task.output.json
            }
            Self::Submit { lease, .. } | Self::Cancel(lease) => lease.task.output.json,
        }
    }

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
            vec![serde_json::to_string_pretty(&receipt.data).unwrap_or_default()],
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
            vec![serde_json::to_string_pretty(&receipt.data).unwrap_or_default()],
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
        vec![serde_json::to_string_pretty(&receipt.data).unwrap_or_default()],
    )
}
