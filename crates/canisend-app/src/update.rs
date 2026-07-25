use canisend_io::{HttpFetcher, RemotePayloadKind};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{ActionReceipt, ApplicationError, NetworkFetchConsent};

const RELEASES_URL: &str = "https://api.github.com/repos/jxpeng98/CanISend/releases?per_page=50";
const RELEASES_HOSTS: &[&str] = &["api.github.com"];
const RELEASE_PAGE_PREFIX: &str = "https://github.com/jxpeng98/CanISend/releases/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateCheckReadModel {
    pub current_version: String,
    pub latest_version: String,
    pub latest_tag: String,
    pub release_name: String,
    pub release_url: String,
    pub published_at: Option<String>,
    pub prerelease: bool,
    pub channel: String,
    pub update_available: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    name: Option<String>,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
}

pub(crate) fn check(
    _consent: NetworkFetchConsent,
) -> Result<ActionReceipt<UpdateCheckReadModel>, ApplicationError> {
    let payload = HttpFetcher::new()
        .fetch_discovery_for_hosts(RELEASES_URL, RELEASES_HOSTS)
        .map_err(|error| ApplicationError::UpdateCheck(error.to_string()))?;
    if payload.kind != RemotePayloadKind::Json {
        return Err(ApplicationError::UpdateCheck(
            "GitHub release endpoint returned a non-JSON response".to_owned(),
        ));
    }
    let current = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|error| {
        ApplicationError::UpdateCheck(format!("current product version is invalid: {error}"))
    })?;
    let data = select_release(&payload.bytes, &current)?;
    let (status, summary) = if data.update_available {
        (
            "update-available",
            format!(
                "CanISend {} is available; this app is {}",
                data.latest_version, data.current_version
            ),
        )
    } else {
        (
            "up-to-date",
            format!(
                "CanISend {} is up to date on the {} channel",
                data.current_version, data.channel
            ),
        )
    };
    Ok(ActionReceipt::new(
        "product.update.check",
        status,
        summary,
        data,
    ))
}

fn select_release(
    bytes: &[u8],
    current: &Version,
) -> Result<UpdateCheckReadModel, ApplicationError> {
    let releases: Vec<GitHubRelease> = serde_json::from_slice(bytes).map_err(|error| {
        ApplicationError::UpdateCheck(format!("GitHub release response is invalid: {error}"))
    })?;
    let preview_channel = !current.pre.is_empty();
    let selected = releases
        .into_iter()
        .filter(|release| !release.draft && (preview_channel || !release.prerelease))
        .filter_map(|release| {
            parse_release_version(&release.tag_name).map(|version| (version, release))
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .ok_or_else(|| {
            ApplicationError::UpdateCheck(
                "GitHub has no compatible published CanISend release".to_owned(),
            )
        })?;
    let (latest, release) = selected;
    if !release.html_url.starts_with(RELEASE_PAGE_PREFIX) {
        return Err(ApplicationError::UpdateCheck(
            "GitHub returned an unexpected release page URL".to_owned(),
        ));
    }
    Ok(UpdateCheckReadModel {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        latest_tag: release.tag_name.clone(),
        release_name: release.name.unwrap_or(release.tag_name),
        release_url: release.html_url,
        published_at: release.published_at,
        prerelease: release.prerelease,
        channel: if preview_channel {
            "preview".to_owned()
        } else {
            "stable".to_owned()
        },
        update_available: latest > *current,
    })
}

fn parse_release_version(tag: &str) -> Option<Version> {
    Version::parse(tag.trim().strip_prefix('v').unwrap_or(tag.trim())).ok()
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use crate::NetworkFetchConsent;

    use super::{check, select_release};

    const RELEASES: &str = r#"[
      {
        "tag_name": "v1.0.0-alpha.1",
        "html_url": "https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.1",
        "name": "CanISend 1.0.0-alpha.1",
        "draft": false,
        "prerelease": true,
        "published_at": "2026-08-01T10:00:00Z"
      },
      {
        "tag_name": "v0.7.0",
        "html_url": "https://github.com/jxpeng98/CanISend/releases/tag/v0.7.0",
        "name": "CanISend 0.7.0",
        "draft": false,
        "prerelease": false,
        "published_at": "2026-07-30T10:00:00Z"
      },
      {
        "tag_name": "v9.0.0",
        "html_url": "https://github.com/jxpeng98/CanISend/releases/tag/v9.0.0",
        "name": "Draft",
        "draft": true,
        "prerelease": false,
        "published_at": null
      }
    ]"#;

    #[test]
    fn preview_channel_includes_published_prereleases() {
        let current = Version::parse("0.7.0-rc.2").expect("current");
        let result = select_release(RELEASES.as_bytes(), &current).expect("release");
        assert_eq!(result.latest_version, "1.0.0-alpha.1");
        assert!(result.prerelease);
        assert!(result.update_available);
        assert_eq!(result.channel, "preview");
    }

    #[test]
    fn stable_channel_ignores_prereleases() {
        let current = Version::parse("0.7.0").expect("current");
        let result = select_release(RELEASES.as_bytes(), &current).expect("release");
        assert_eq!(result.latest_version, "0.7.0");
        assert!(!result.prerelease);
        assert!(!result.update_available);
        assert_eq!(result.channel, "stable");
    }

    #[test]
    fn release_page_must_stay_on_the_project() {
        let fixture = RELEASES.replace(
            "https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.1",
            "https://example.invalid/releases/tag/v1.0.0-alpha.1",
        );
        let current = Version::parse("0.7.0-rc.2").expect("current");
        assert!(select_release(fixture.as_bytes(), &current).is_err());
    }

    #[test]
    #[ignore = "requires the public GitHub Releases endpoint"]
    fn public_release_endpoint_matches_the_bounded_contract() {
        let receipt =
            check(NetworkFetchConsent::granted_by_user()).expect("public release response");
        assert_eq!(receipt.operation, "product.update.check");
        assert!(
            receipt
                .data
                .release_url
                .starts_with("https://github.com/jxpeng98/CanISend/releases/")
        );
    }
}
