use std::{path::PathBuf, str::FromStr};

use canisend_contracts::{
    PUBLIC_SCHEMA_VERSION, PublicSchemaId, ResourceCatalogData, ResourceCatalogEntry,
    SchemaCatalogData, SchemaCatalogEntry, SemanticVersion, Sha256Digest,
};
use canisend_resources::{
    ResourceCatalogExportData, ResourceId, ResourceKind, export_catalog, get, manifest,
};
use serde::{Deserialize, Serialize};

use crate::{ActionReceipt, Application, ApplicationError};

pub type ResourceCatalogExportReadModel = ResourceCatalogExportData;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceCatalogExportRequest {
    pub destination: PathBuf,
}

impl ResourceCatalogExportRequest {
    #[must_use]
    pub fn new(destination: impl Into<PathBuf>) -> Self {
        Self {
            destination: destination.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDetailReadModel {
    pub entry: ResourceCatalogEntry,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectionCatalogReadModel {
    pub schemas: SchemaCatalogData,
    pub resources: Vec<ResourceDetailReadModel>,
}

impl Application {
    pub fn schema_catalog() -> Result<ActionReceipt<SchemaCatalogData>, ApplicationError> {
        verify_resources()?;
        let schemas = PublicSchemaId::ALL
            .into_iter()
            .map(schema_entry)
            .collect::<Result<Vec<_>, _>>()?;
        let count = schemas.len();
        Ok(ActionReceipt::new(
            "schema.list",
            "available",
            format!("Loaded {count} verified public schema(s)"),
            SchemaCatalogData { schemas },
        ))
    }

    pub fn schema_detail(
        query: &str,
    ) -> Result<ActionReceipt<SchemaCatalogEntry>, ApplicationError> {
        verify_resources()?;
        let schema_id = PublicSchemaId::ALL
            .into_iter()
            .find(|schema_id| schema_id.as_str() == query || schema_id.slug() == query)
            .ok_or_else(|| ApplicationError::SchemaNotFound(query.to_owned()))?;
        Ok(ActionReceipt::new(
            "schema.show",
            "available",
            format!("Loaded verified schema {}", schema_id.as_str()),
            schema_entry(schema_id)?,
        ))
    }

    pub fn resource_catalog() -> Result<ActionReceipt<ResourceCatalogData>, ApplicationError> {
        verify_resources()?;
        let resources = resource_details()?
            .into_iter()
            .map(|detail| detail.entry)
            .collect::<Vec<_>>();
        let count = resources.len();
        Ok(ActionReceipt::new(
            "resource.list",
            "available",
            format!("Loaded {count} verified embedded resource(s)"),
            ResourceCatalogData { resources },
        ))
    }

    pub fn resource_detail(
        query: &str,
    ) -> Result<ActionReceipt<ResourceDetailReadModel>, ApplicationError> {
        verify_resources()?;
        let resource_id = ResourceId::from_str(query)?;
        let detail = resource_detail(resource_id)?;
        Ok(ActionReceipt::new(
            "resource.show",
            "available",
            format!("Loaded verified embedded resource {}", detail.entry.id),
            detail,
        ))
    }

    pub fn inspection_catalog()
    -> Result<ActionReceipt<InspectionCatalogReadModel>, ApplicationError> {
        verify_resources()?;
        let schemas = PublicSchemaId::ALL
            .into_iter()
            .map(schema_entry)
            .collect::<Result<Vec<_>, _>>()?;
        let resources = resource_details()?;
        Ok(ActionReceipt::new(
            "inspection.catalog",
            "available",
            format!(
                "Loaded {} public schema(s) and {} embedded resource(s)",
                schemas.len(),
                resources.len()
            ),
            InspectionCatalogReadModel {
                schemas: SchemaCatalogData { schemas },
                resources,
            },
        ))
    }

    pub fn export_resource_catalog(
        request: &ResourceCatalogExportRequest,
    ) -> Result<ActionReceipt<ResourceCatalogExportReadModel>, ApplicationError> {
        verify_resources()?;
        let exported = export_catalog(&ResourceId::ALL, &request.destination)?;
        let count = exported.manifest.files.len();
        Ok(ActionReceipt::new(
            "resource.export",
            "exported",
            format!(
                "Exported {count} verified public resource(s) with an integrity manifest; \
                 workspace bodies exported: no; host launched: no"
            ),
            exported,
        ))
    }
}

fn schema_entry(schema_id: PublicSchemaId) -> Result<SchemaCatalogEntry, ApplicationError> {
    let resource_id = ResourceId::from_str(&format!("schema.{}", schema_id.slug()))
        .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?;
    let descriptor = get(resource_id).descriptor;
    Ok(SchemaCatalogEntry {
        id: schema_id.as_str().to_owned(),
        version: semantic_version(PUBLIC_SCHEMA_VERSION)?,
        uri: schema_id.canonical_uri(),
        resource_id: resource_id.as_str().to_owned(),
        size: descriptor.size,
        sha256: sha256(descriptor.sha256)?,
    })
}

fn resource_details() -> Result<Vec<ResourceDetailReadModel>, ApplicationError> {
    manifest()
        .into_iter()
        .map(|descriptor| {
            let resource_id = ResourceId::from_str(descriptor.id)
                .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))?;
            resource_detail(resource_id)
        })
        .collect()
}

fn resource_detail(resource_id: ResourceId) -> Result<ResourceDetailReadModel, ApplicationError> {
    let descriptor = get(resource_id).descriptor;
    Ok(ResourceDetailReadModel {
        entry: ResourceCatalogEntry {
            id: descriptor.id.to_owned(),
            kind: resource_kind_name(descriptor.kind).to_owned(),
            version: semantic_version(descriptor.version)?,
            size: descriptor.size,
            sha256: sha256(descriptor.sha256)?,
        },
        path: descriptor.path.to_owned(),
    })
}

fn semantic_version(value: &str) -> Result<SemanticVersion, ApplicationError> {
    SemanticVersion::try_new(value)
        .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))
}

fn sha256(value: &str) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::try_new(value)
        .map_err(|error| ApplicationError::ResourceIntegrity(error.to_string()))
}

fn verify_resources() -> Result<(), ApplicationError> {
    canisend_resources::verify().map_err(ApplicationError::ResourceIntegrity)
}

const fn resource_kind_name(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Agent => "agent",
        ResourceKind::Example => "example",
        ResourceKind::Prompt => "prompt",
        ResourceKind::Schema => "schema",
        ResourceKind::Template => "template",
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{ErrorCode, PublicSchemaId};
    use canisend_resources::ResourceId;
    use sha2::{Digest, Sha256};

    use crate::{
        ActionReceipt, Application, InspectionCatalogReadModel, ResourceCatalogExportReadModel,
        ResourceCatalogExportRequest,
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-catalog-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn catalogs_are_deterministic_verified_and_workspace_independent() {
        let first = Application::inspection_catalog().expect("inspection catalog");
        let second = Application::inspection_catalog().expect("repeat catalog");
        assert_eq!(first, second);
        assert_eq!(first.data.schemas.schemas.len(), PublicSchemaId::ALL.len());
        assert_eq!(first.data.resources.len(), ResourceId::ALL.len());
        assert!(first.data.resources.iter().all(|detail| {
            !detail.path.is_empty()
                && detail.entry.sha256.as_str().len() == 64
                && !detail.path.contains(".canisend")
        }));
        let round_trip: ActionReceipt<InspectionCatalogReadModel> =
            serde_json::from_slice(&serde_json::to_vec(&first).expect("encode catalog"))
                .expect("decode catalog");
        assert_eq!(round_trip, first);

        let by_id =
            Application::schema_detail(PublicSchemaId::Document.as_str()).expect("schema by ID");
        let by_slug =
            Application::schema_detail(PublicSchemaId::Document.slug()).expect("schema by slug");
        assert_eq!(by_id.data, by_slug.data);
        assert_eq!(
            Application::schema_detail("missing")
                .expect_err("missing schema")
                .classify()
                .code,
            ErrorCode::SchemaNotFound
        );
        assert_eq!(
            Application::resource_detail("missing")
                .expect_err("missing resource")
                .classify()
                .code,
            ErrorCode::ResourceNotFound
        );
    }

    #[test]
    fn complete_catalog_export_is_create_new_and_digest_bound() {
        let parent = temporary_root("export-parent");
        fs::create_dir(&parent).expect("export parent");
        let destination = parent.join("catalog");
        let request = ResourceCatalogExportRequest::new(&destination);
        let exported = Application::export_resource_catalog(&request).expect("catalog export");
        assert_eq!(exported.operation, "resource.export");
        assert_eq!(exported.data.manifest.files.len(), ResourceId::ALL.len());
        for file in &exported.data.manifest.files {
            let bytes = fs::read(destination.join(&file.path)).expect("exported file");
            assert_eq!(bytes.len(), file.size);
            assert_eq!(hex::encode(Sha256::digest(bytes)), file.sha256);
        }
        let round_trip: ActionReceipt<ResourceCatalogExportReadModel> =
            serde_json::from_slice(&serde_json::to_vec(&exported).expect("encode export"))
                .expect("decode export");
        assert_eq!(round_trip, exported);
        assert_eq!(
            Application::export_resource_catalog(&request)
                .expect_err("repeat export")
                .classify()
                .code,
            ErrorCode::InputPathRejected
        );

        let internal = parent.join(".canisend/catalog");
        assert_eq!(
            Application::export_resource_catalog(&ResourceCatalogExportRequest::new(&internal))
                .expect_err("internal export")
                .classify()
                .code,
            ErrorCode::InputPathRejected
        );
        assert!(!internal.exists());
        fs::remove_dir_all(parent).expect("cleanup catalog");
    }
}
