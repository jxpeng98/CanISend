use std::{collections::BTreeMap, path::Path};

use canisend_contracts::{
    ApplicationFieldValueV3, ConsentScope, ContentRevisionReferenceV3, NextAction,
    PrivacyClassification, RequirementPriorityV3, Sha256Digest, WorkflowPackId, WorkflowPackItemId,
    WorkspaceSourceKindV4,
};
use canisend_io::{HttpFetcher, RemoteDocumentKind, extract_pdf_text};
use canisend_store::{
    ApplicationAssociationServiceV4, ApplicationFlowCreateRequestV3, ApplicationFlowReadModelV3,
    ApplicationFlowServiceV3, NewWorkspaceSourceV4, validate_application_flow_create_request,
};
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, NetworkFetchConsent,
    PastedTextIntakePreviewRequestV4, SourceDuplicateSignalV4, application::open_workspace,
    application_flow_v3::requested_built_in_pack, intake_v4::digest,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UrlDocumentKindV4 {
    Html,
    PlainText,
    Pdf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UrlIntakePreviewRequestV4 {
    pub pack_id: WorkflowPackId,
    pub title: String,
    pub opportunity_metadata: BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    pub application_metadata: BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    pub url: String,
    pub requirement_category: WorkflowPackItemId,
    pub requirement_priority: RequirementPriorityV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UrlIntakePreviewReadModelV4 {
    pub preview_sha256: Sha256Digest,
    pub original_sha256: Sha256Digest,
    pub normalized_sha256: Sha256Digest,
    pub document_kind: UrlDocumentKindV4,
    pub source_url: String,
    pub final_url: String,
    pub redirect_chain: Vec<String>,
    pub content_type: String,
    pub original_bytes: u64,
    pub normalized_text_bytes: u64,
    pub normalized_lines: u64,
    pub pdf_page_count: Option<u64>,
    pub duplicates: Vec<SourceDuplicateSignalV4>,
    pub application: ApplicationFlowCreateRequestV3,
    pub submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UrlIntakeCommitRequestV4 {
    pub preview: UrlIntakePreviewRequestV4,
    pub expected_preview_sha256: Sha256Digest,
}

struct PreparedUrlIntakeV4 {
    preview: UrlIntakePreviewReadModelV4,
    source: NewWorkspaceSourceV4,
}

impl Application {
    pub fn preview_url_intake_v4(
        workspace_root: &Path,
        request: UrlIntakePreviewRequestV4,
        consent: Option<NetworkFetchConsent>,
    ) -> Result<ActionReceipt<UrlIntakePreviewReadModelV4>, ApplicationError> {
        require_network_fetch_consent(consent)?;
        preview_url_intake_with_fetcher(workspace_root, request, &HttpFetcher::new())
    }

    pub fn commit_url_intake_v4(
        workspace_root: &Path,
        request: UrlIntakeCommitRequestV4,
        consent: Option<NetworkFetchConsent>,
    ) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, ApplicationError> {
        require_network_fetch_consent(consent)?;
        commit_url_intake_with_fetcher(workspace_root, request, &HttpFetcher::new())
    }
}

fn preview_url_intake_with_fetcher(
    workspace_root: &Path,
    request: UrlIntakePreviewRequestV4,
    fetcher: &HttpFetcher,
) -> Result<ActionReceipt<UrlIntakePreviewReadModelV4>, ApplicationError> {
    let prepared = prepare_url_intake(workspace_root, &request, fetcher)?;
    Ok(ActionReceipt::new(
        "application.intake.url.preview",
        "previewed",
        "Fetched one consented bounded public URL and proposed exact-span Requirements without Workspace mutation",
        prepared.preview,
    ))
}

fn commit_url_intake_with_fetcher(
    workspace_root: &Path,
    request: UrlIntakeCommitRequestV4,
    fetcher: &HttpFetcher,
) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, ApplicationError> {
    let pack = requested_built_in_pack(&request.preview.pack_id)?;
    let prepared = prepare_url_intake(workspace_root, &request.preview, fetcher)?;
    if prepared.preview.preview_sha256 != request.expected_preview_sha256 {
        return Err(ApplicationError::InvalidInput(
            "URL intake preview is stale or the fetched bytes and provenance no longer match the reviewed digest"
                .to_owned(),
        ));
    }
    let mut workspace = open_workspace(workspace_root)?;
    let root = workspace.paths.root.clone();
    let committed = ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
        .create_with_source(
            &pack,
            prepared.preview.application,
            prepared.source,
            Some(ConsentScope::FetchUserSuppliedUrl),
        )?;
    Ok(ActionReceipt::new(
        "application.intake.url.commit",
        "created",
        "Committed the reviewed URL Source, network-fetch consent, explicit Application link, and proposed Requirements",
        committed,
    ))
}

fn prepare_url_intake(
    workspace_root: &Path,
    request: &UrlIntakePreviewRequestV4,
    fetcher: &HttpFetcher,
) -> Result<PreparedUrlIntakeV4, ApplicationError> {
    Application::workspace_status_v4(workspace_root)?;
    if request.url.trim().is_empty() {
        return Err(ApplicationError::InvalidInput(
            "URL cannot be empty".to_owned(),
        ));
    }
    let document = fetcher.fetch(request.url.trim())?;
    let document_kind = match document.kind {
        RemoteDocumentKind::Html => UrlDocumentKindV4::Html,
        RemoteDocumentKind::PlainText => UrlDocumentKindV4::PlainText,
        RemoteDocumentKind::Pdf => UrlDocumentKindV4::Pdf,
    };
    let (normalized_text, pdf_page_count) = if document.kind == RemoteDocumentKind::Pdf {
        let extracted = extract_pdf_text(document.original_bytes.clone())?;
        (
            extracted.normalized_text,
            Some(u64::try_from(extracted.page_count).map_err(|_| {
                ApplicationError::InvalidInput("PDF page count overflow".to_owned())
            })?),
        )
    } else {
        (
            document
                .normalized_text
                .clone()
                .ok_or(canisend_io::IoAdapterError::TextUnavailable)?,
            None,
        )
    };
    let pasted = PastedTextIntakePreviewRequestV4 {
        pack_id: request.pack_id.clone(),
        title: request.title.clone(),
        opportunity_metadata: request.opportunity_metadata.clone(),
        application_metadata: request.application_metadata.clone(),
        source_text: normalized_text.clone(),
        requirement_category: request.requirement_category.clone(),
        requirement_priority: request.requirement_priority,
    };
    let mut application = Application::preview_pasted_text_intake_v4(workspace_root, pasted)?
        .data
        .application;
    if document_kind == UrlDocumentKindV4::Pdf {
        application
            .requirements
            .retain(|requirement| !is_pdf_page_marker(&requirement.statement));
        let pack = requested_built_in_pack(&request.pack_id)?;
        validate_application_flow_create_request(&pack, &application)?;
    }
    let original_sha256 = digest(&document.original_bytes)?;
    let normalized_sha256 = digest(normalized_text.as_bytes())?;
    let duplicates = {
        let mut workspace = open_workspace(workspace_root)?;
        ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs)
            .source_duplicates(&original_sha256, &normalized_sha256)?
            .into_iter()
            .map(|record| SourceDuplicateSignalV4 {
                source: ContentRevisionReferenceV3 {
                    id: record.id,
                    revision: record.revision,
                    sha256: record.normalized_sha256.clone(),
                },
                original_bytes_match: record.original_sha256 == original_sha256,
                normalized_text_match: record.normalized_sha256 == normalized_sha256,
            })
            .collect::<Vec<_>>()
    };
    let preview_bytes = serde_json::to_vec(&(
        "canisend.url-intake-preview/v4",
        request,
        document_kind,
        &document.source_url,
        &document.final_url,
        &document.redirect_chain,
        &document.content_type,
        &original_sha256,
        &normalized_sha256,
        pdf_page_count,
        &duplicates,
        &application,
    ))
    .map_err(|error| {
        ApplicationError::InvalidInput(format!("could not encode URL intake preview: {error}"))
    })?;
    let preview_sha256 = digest(&preview_bytes)?;
    let original_bytes = u64::try_from(document.original_bytes.len())
        .map_err(|_| ApplicationError::InvalidInput("Source byte count overflow".to_owned()))?;
    let normalized_text_bytes = u64::try_from(normalized_text.len())
        .map_err(|_| ApplicationError::InvalidInput("Source byte count overflow".to_owned()))?;
    let normalized_lines = u64::try_from(normalized_text.lines().count())
        .map_err(|_| ApplicationError::InvalidInput("Source line count overflow".to_owned()))?;
    Ok(PreparedUrlIntakeV4 {
        preview: UrlIntakePreviewReadModelV4 {
            preview_sha256,
            original_sha256,
            normalized_sha256,
            document_kind,
            source_url: document.source_url.clone(),
            final_url: document.final_url.clone(),
            redirect_chain: document.redirect_chain.clone(),
            content_type: document.content_type.clone(),
            original_bytes,
            normalized_text_bytes,
            normalized_lines,
            pdf_page_count,
            duplicates,
            application,
            submission_performed: false,
        },
        source: NewWorkspaceSourceV4 {
            kind: WorkspaceSourceKindV4::Url,
            locator: document.source_url,
            final_locator: Some(document.final_url),
            redirect_chain: document.redirect_chain,
            content_type: document.content_type,
            original_bytes: document.original_bytes,
            normalized_text,
            privacy: PrivacyClassification::Public,
        },
    })
}

fn require_network_fetch_consent(
    consent: Option<NetworkFetchConsent>,
) -> Result<(), ApplicationError> {
    if consent.is_some() {
        return Ok(());
    }
    Err(ApplicationError::ConsentRequired {
        message: "URL intake performs a bounded network fetch to a user-supplied public URL"
            .to_owned(),
        remediation: NextAction {
            action: "grant network fetch consent".to_owned(),
            description:
                "Confirm this one user-supplied URL fetch, then repeat the preview or commit"
                    .to_owned(),
        },
    })
}

fn is_pdf_page_marker(statement: &str) -> bool {
    statement
        .strip_prefix("--- Page ")
        .and_then(|value| value.strip_suffix(" ---"))
        .is_some_and(|page| !page.is_empty() && page.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::{Ipv4Addr, TcpListener},
        path::PathBuf,
        thread,
        time::Duration,
    };

    use canisend_contracts::{ApplicationFieldValueV3, Revision};
    use canisend_store::{ApplicationAssociationServiceV4, Workspace};

    use super::*;

    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-url-intake-v4-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("Pack item ID")
    }

    fn request(url: String) -> UrlIntakePreviewRequestV4 {
        UrlIntakePreviewRequestV4 {
            pack_id: WorkflowPackId::try_new(crate::GENERIC_APPLICATION_WORKFLOW_PACK_ID)
                .expect("Pack ID"),
            title: "Community programme".to_owned(),
            opportunity_metadata: BTreeMap::from([(
                item("organization"),
                ApplicationFieldValueV3::ShortText("Example Foundation".to_owned()),
            )]),
            application_metadata: BTreeMap::from([(
                item("status"),
                ApplicationFieldValueV3::Choice(item("planning")),
            )]),
            url,
            requirement_category: item("format"),
            requirement_priority: RequirementPriorityV3::Mandatory,
        }
    }

    fn academic_request(url: String) -> UrlIntakePreviewRequestV4 {
        UrlIntakePreviewRequestV4 {
            pack_id: WorkflowPackId::try_new(crate::ACADEMIC_JOB_WORKFLOW_PACK_ID)
                .expect("Pack ID"),
            title: "Research fellowship".to_owned(),
            opportunity_metadata: BTreeMap::from([(
                item("institution"),
                ApplicationFieldValueV3::ShortText("Example University".to_owned()),
            )]),
            application_metadata: BTreeMap::new(),
            url,
            requirement_category: item("qualification"),
            requirement_priority: RequirementPriorityV3::Mandatory,
        }
    }

    #[test]
    fn consent_precedes_workspace_and_network_policy_access() {
        let root = root("consent");
        let error = Application::preview_url_intake_v4(
            &root,
            request("http://127.0.0.1:9/private".to_owned()),
            None,
        )
        .expect_err("consent required before access");
        assert!(matches!(error, ApplicationError::ConsentRequired { .. }));
        assert!(!root.exists());
    }

    #[test]
    fn production_fetcher_rejects_private_destination_without_mutation() {
        let root = root("policy");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let error = Application::preview_url_intake_v4(
            &root,
            request("http://127.0.0.1:9/private".to_owned()),
            Some(NetworkFetchConsent::granted_by_user()),
        )
        .expect_err("private destination policy");
        assert!(matches!(error, ApplicationError::Input(_)));
        assert!(
            Application::list_application_models_v3(&root)
                .expect("Applications")
                .data
                .is_empty()
        );
        let workspace = Workspace::open_from(Some(&root), &root).expect("open Workspace");
        drop(workspace);
        std::fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn redirected_html_commit_preserves_provenance_consent_and_duplicate_signal() {
        let root = root("redirect");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let body = b"<!doctype html><html><body><h1>Programme requirements</h1><p>Narrative required.</p><p>Budget required.</p></body></html>";
        let redirect = redirect_response("/final");
        let success = response("text/html; charset=utf-8", body);
        let (base, server) = serve(vec![
            redirect.clone(),
            success.clone(),
            redirect.clone(),
            success.clone(),
            redirect.clone(),
            success.clone(),
            redirect,
            success,
        ]);
        let request = request(format!("{base}/start"));
        let fetcher = HttpFetcher::for_bounded_loopback_tests(Duration::from_secs(1));
        require_network_fetch_consent(Some(NetworkFetchConsent::granted_by_user()))
            .expect("network consent");
        let preview = preview_url_intake_with_fetcher(&root, request.clone(), &fetcher)
            .expect("URL preview")
            .data;
        assert_eq!(preview.document_kind, UrlDocumentKindV4::Html);
        assert_eq!(preview.final_url, format!("{base}/final"));
        assert_eq!(preview.redirect_chain, vec![format!("{base}/final")]);
        assert!(preview.duplicates.is_empty());
        assert!(preview.application.requirements.len() >= 2);

        let committed = commit_url_intake_with_fetcher(
            &root,
            UrlIntakeCommitRequestV4 {
                preview: request.clone(),
                expected_preview_sha256: preview.preview_sha256,
            },
            &fetcher,
        )
        .expect("URL commit")
        .data;
        let academic_request = academic_request(request.url.clone());
        let academic_preview =
            preview_url_intake_with_fetcher(&root, academic_request.clone(), &fetcher)
                .expect("academic duplicate URL preview")
                .data;
        assert_eq!(academic_preview.duplicates.len(), 1);
        let academic = commit_url_intake_with_fetcher(
            &root,
            UrlIntakeCommitRequestV4 {
                preview: academic_request,
                expected_preview_sha256: academic_preview.preview_sha256,
            },
            &fetcher,
        )
        .expect("academic URL commit")
        .data;
        server.join().expect("URL server");

        let mut workspace = Workspace::open_from(Some(&root), &root).expect("open Workspace");
        let associations =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs);
        let links = associations
            .source_associations(&committed.stored.snapshot.application.id)
            .expect("Source association");
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].consent_scope,
            Some(ConsentScope::FetchUserSuppliedUrl)
        );
        let source = associations
            .source(&links[0].source.id, Revision::try_new(1).expect("revision"))
            .expect("Source revision");
        assert_eq!(source.kind, WorkspaceSourceKindV4::Url);
        assert_eq!(source.locator, format!("{base}/start"));
        assert_eq!(source.final_locator, Some(format!("{base}/final")));
        assert_eq!(source.redirect_chain, vec![format!("{base}/final")]);
        let academic_links = associations
            .source_associations(&academic.stored.snapshot.application.id)
            .expect("academic Source association");
        assert_eq!(academic_links.len(), 1);
        assert_ne!(academic_links[0].source.id, links[0].source.id);
        drop(workspace);
        std::fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn changed_remote_bytes_reject_stale_commit_without_authority_mutation() {
        let root = root("stale");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let first = response("text/plain; charset=utf-8", b"Original requirement.\n");
        let second = response("text/plain; charset=utf-8", b"Changed requirement.\n");
        let (url, server) = serve(vec![first, second]);
        let request = request(url);
        let fetcher = HttpFetcher::for_bounded_loopback_tests(Duration::from_secs(1));
        let preview = preview_url_intake_with_fetcher(&root, request.clone(), &fetcher)
            .expect("URL preview")
            .data;
        let error = commit_url_intake_with_fetcher(
            &root,
            UrlIntakeCommitRequestV4 {
                preview: request,
                expected_preview_sha256: preview.preview_sha256,
            },
            &fetcher,
        )
        .expect_err("stale remote Source");
        assert!(matches!(error, ApplicationError::InvalidInput(_)));
        server.join().expect("URL server");
        assert!(
            Application::list_application_models_v3(&root)
                .expect("Applications")
                .data
                .is_empty()
        );
        let mut workspace = Workspace::open_from(Some(&root), &root).expect("open Workspace");
        let duplicates =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs)
                .source_duplicates(&preview.original_sha256, &preview.normalized_sha256)
                .expect("Source duplicate query");
        assert!(duplicates.is_empty());
        drop(workspace);
        std::fs::remove_dir_all(root).expect("remove fixture");
    }

    fn redirect_response(location: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .into_bytes()
    }

    fn response(content_type: &str, body: &[u8]) -> Vec<u8> {
        let mut response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        response.extend_from_slice(body);
        response
    }

    fn serve(responses: Vec<Vec<u8>>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("test listener");
        let address = listener.local_addr().expect("test address");
        let server = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().expect("test request");
                let mut request = [0_u8; 4096];
                let _ = stream.read(&mut request).expect("read request");
                stream.write_all(&response).expect("write response");
            }
        });
        (format!("http://{address}"), server)
    }
}
