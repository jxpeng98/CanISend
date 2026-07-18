use std::{fmt, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{code}: {message}")]
pub struct PrimitiveError {
    pub code: &'static str,
    pub message: String,
}

impl PrimitiveError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

macro_rules! validated_string {
    ($name:ident, $validator:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn try_new(value: impl Into<String>) -> Result<Self, PrimitiveError> {
                let value = value.into();
                $validator(&value)?;
                Ok(Self(value))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = PrimitiveError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::try_new(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::try_new(value).map_err(D::Error::custom)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }
    };
}

validated_string!(SemanticVersion, validate_semantic_version);
validated_string!(EntityId, validate_entity_id);
validated_string!(Sha256Digest, validate_sha256_digest);
validated_string!(UtcTimestamp, validate_utc_timestamp);
validated_string!(SafeRelativePath, validate_relative_path);

fn validate_semantic_version(value: &str) -> Result<(), PrimitiveError> {
    semver::Version::parse(value)
        .map(|_| ())
        .map_err(|error| PrimitiveError::new("version.invalid", error.to_string()))
}

fn validate_entity_id(value: &str) -> Result<(), PrimitiveError> {
    let bytes = value.as_bytes();
    let shape_ok = bytes.len() == 36
        && bytes[8] == b'-'
        && bytes[13] == b'-'
        && bytes[18] == b'-'
        && bytes[23] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 8 | 13 | 18 | 23) || byte.is_ascii_hexdigit());
    if !shape_ok || bytes[14] != b'7' || !matches!(bytes[19].to_ascii_lowercase(), b'8'..=b'b') {
        return Err(PrimitiveError::new(
            "id.invalid",
            "expected a canonical UUIDv7 identifier",
        ));
    }
    Ok(())
}

fn validate_sha256_digest(value: &str) -> Result<(), PrimitiveError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(PrimitiveError::new(
            "digest.invalid",
            "expected 64 lowercase hexadecimal SHA-256 characters",
        ));
    }
    Ok(())
}

fn validate_utc_timestamp(value: &str) -> Result<(), PrimitiveError> {
    if !value.ends_with('Z') {
        return Err(PrimitiveError::new(
            "timestamp.not_utc",
            "timestamp must use the UTC Z suffix",
        ));
    }
    OffsetDateTime::parse(value, &Rfc3339)
        .map(|_| ())
        .map_err(|error| PrimitiveError::new("timestamp.invalid", error.to_string()))
}

fn validate_relative_path(value: &str) -> Result<(), PrimitiveError> {
    if value.is_empty() || value.len() > 512 {
        return Err(PrimitiveError::new(
            "path.invalid_length",
            "relative path must contain between 1 and 512 bytes",
        ));
    }
    if value.starts_with('/') || value.contains('\\') || value.contains('\0') {
        return Err(PrimitiveError::new(
            "path.unsafe",
            "path must be a portable forward-slash relative path",
        ));
    }
    if value
        .split('/')
        .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(PrimitiveError::new(
            "path.unsafe",
            "path cannot contain empty, current-directory, or parent-directory components",
        ));
    }
    let first = value
        .split('/')
        .next()
        .expect("non-empty path has a component");
    if first.eq_ignore_ascii_case(".canisend") || first.ends_with(':') {
        return Err(PrimitiveError::new(
            "path.reserved",
            "path cannot target internal state or a drive prefix",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct Revision(u64);

impl Revision {
    pub fn try_new(value: u64) -> Result<Self, PrimitiveError> {
        if value == 0 {
            return Err(PrimitiveError::new(
                "revision.invalid",
                "revision numbers begin at one",
            ));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn get(self) -> u64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for Revision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_new(u64::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    SourceOriginal,
    SourceNormalizedText,
    ParsedJob,
    EvidenceCatalog,
    Criteria,
    EvidenceMatches,
    ApplicationPlan,
    CoverLetter,
    ResearchStatement,
    TeachingStatement,
    Cv,
    DocumentSet,
    ReviewFindings,
    PackageManifest,
    TypstSource,
    Pdf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PrivacyClassification {
    Public,
    PrivateLocal,
    ProviderBound,
    Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ConsentScope {
    ReadPrivateInputs,
    SendToConfiguredProvider,
    FetchUserSuppliedUrl,
    ExportPrivateArtifacts,
    UseSystemFonts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ActorKind {
    User,
    HostAgent,
    ConfiguredProvider,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionMode {
    Deterministic,
    HostAgent,
    ConfiguredProvider,
    UserDecision,
    ManualImport,
}

#[cfg(test)]
mod tests {
    use super::{EntityId, Revision, SafeRelativePath, Sha256Digest, UtcTimestamp};

    #[test]
    fn strong_primitives_accept_canonical_values() {
        EntityId::try_new("019f2f55-7c00-7000-8000-000000000002").expect("UUIDv7");
        Sha256Digest::try_new("a".repeat(64)).expect("SHA-256");
        UtcTimestamp::try_new("2026-07-17T12:30:00Z").expect("UTC timestamp");
        Revision::try_new(1).expect("positive revision");
        SafeRelativePath::try_new("jobs/example/source/job.pdf").expect("relative path");
    }

    #[test]
    fn safe_relative_path_rejects_escape_and_internal_state() {
        for path in [
            "../secret",
            "/tmp/file",
            "jobs\\file",
            ".canisend/state.sqlite3",
        ] {
            assert!(SafeRelativePath::try_new(path).is_err(), "accepted {path}");
        }
    }

    #[test]
    fn deserialization_cannot_bypass_validation() {
        assert!(serde_json::from_str::<EntityId>("\"not-an-id\"").is_err());
        assert!(serde_json::from_str::<Revision>("0").is_err());
    }
}
