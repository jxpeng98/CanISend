use std::{
    io::Read,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs},
    time::Duration,
};

use reqwest::{
    blocking::{Client, Response},
    header::{CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, LOCATION},
    redirect::Policy,
};
use url::{Host, Url};

use crate::{IoAdapterError, normalize_utf8_text};

pub const MAX_REMOTE_SOURCE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;
const MAX_URL_BYTES: usize = 4096;
const MAX_NORMALIZED_TEXT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteDocumentKind {
    Html,
    PlainText,
    Pdf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDocument {
    pub kind: RemoteDocumentKind,
    pub original_bytes: Vec<u8>,
    pub normalized_text: Option<String>,
    pub source_url: String,
    pub final_url: String,
    pub content_type: String,
    pub redirect_chain: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemotePayloadKind {
    Json,
    Xml,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePayload {
    pub kind: RemotePayloadKind,
    pub bytes: Vec<u8>,
    pub source_url: String,
    pub final_url: String,
    pub content_type: String,
    pub redirect_chain: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct HttpFetcher {
    allow_loopback_for_tests: bool,
    connect_timeout: Duration,
    request_timeout: Duration,
}

impl Default for HttpFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpFetcher {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            allow_loopback_for_tests: false,
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
        }
    }

    #[cfg(test)]
    const fn allowing_loopback_for_tests(request_timeout: Duration) -> Self {
        Self {
            allow_loopback_for_tests: true,
            connect_timeout: Duration::from_secs(1),
            request_timeout,
        }
    }

    pub fn fetch(&self, input: &str) -> Result<RemoteDocument, IoAdapterError> {
        let (response, source_url, final_url, redirect_chain) = self.request(input, None)?;
        decode_response(response, source_url, final_url, redirect_chain)
    }

    pub fn fetch_discovery(&self, input: &str) -> Result<RemotePayload, IoAdapterError> {
        let (response, source_url, final_url, redirect_chain) = self.request(input, None)?;
        decode_discovery_response(response, source_url, final_url, redirect_chain)
    }

    pub fn fetch_discovery_for_hosts(
        &self,
        input: &str,
        allowed_hosts: &[&str],
    ) -> Result<RemotePayload, IoAdapterError> {
        if allowed_hosts.is_empty() {
            return Err(IoAdapterError::UrlPolicy(
                "provider redirect host allowlist cannot be empty".to_owned(),
            ));
        }
        let (response, source_url, final_url, redirect_chain) =
            self.request(input, Some(allowed_hosts))?;
        decode_discovery_response(response, source_url, final_url, redirect_chain)
    }

    fn request(
        &self,
        input: &str,
        allowed_hosts: Option<&[&str]>,
    ) -> Result<(Response, String, String, Vec<String>), IoAdapterError> {
        let source_url = parse_url(input)?;
        let source_url_string = source_url.as_str().to_owned();
        let mut current = source_url;
        let mut redirect_chain = Vec::new();
        for redirect_count in 0..=MAX_REDIRECTS {
            if let Some(allowed_hosts) = allowed_hosts {
                validate_allowed_host(&current, allowed_hosts)?;
            }
            let resolved = resolve_checked(&current, self.allow_loopback_for_tests)?;
            let client = build_client(
                &current,
                &resolved,
                self.connect_timeout,
                self.request_timeout,
            )?;
            let response = client.get(current.clone()).send()?;
            if response.status().is_redirection() {
                if redirect_count == MAX_REDIRECTS {
                    return Err(IoAdapterError::InvalidRedirect(
                        "redirect limit exceeded".to_owned(),
                    ));
                }
                let location = response
                    .headers()
                    .get(LOCATION)
                    .ok_or_else(|| {
                        IoAdapterError::InvalidRedirect(
                            "redirect response has no Location header".to_owned(),
                        )
                    })?
                    .to_str()
                    .map_err(|_| {
                        IoAdapterError::InvalidRedirect(
                            "Location header is not valid text".to_owned(),
                        )
                    })?;
                if location.len() > MAX_URL_BYTES {
                    return Err(IoAdapterError::InvalidRedirect(
                        "Location header is too long".to_owned(),
                    ));
                }
                let next = current.join(location).map_err(|error| {
                    IoAdapterError::InvalidRedirect(format!("cannot resolve Location: {error}"))
                })?;
                let next = validate_url(next)?;
                if current.scheme() == "https" && next.scheme() != "https" {
                    return Err(IoAdapterError::InvalidRedirect(
                        "HTTPS redirect cannot downgrade to HTTP".to_owned(),
                    ));
                }
                redirect_chain.push(next.as_str().to_owned());
                current = next;
                continue;
            }
            if !response.status().is_success() {
                return Err(IoAdapterError::HttpStatus(response.status().as_u16()));
            }
            return Ok((
                response,
                source_url_string,
                current.as_str().to_owned(),
                redirect_chain,
            ));
        }
        Err(IoAdapterError::InvalidRedirect(
            "redirect state is inconsistent".to_owned(),
        ))
    }
}

fn validate_allowed_host(url: &Url, allowed_hosts: &[&str]) -> Result<(), IoAdapterError> {
    let host = url
        .host_str()
        .ok_or_else(|| IoAdapterError::InvalidUrl("URL has no host".to_owned()))?;
    if allowed_hosts
        .iter()
        .any(|allowed| host.eq_ignore_ascii_case(allowed))
    {
        return Ok(());
    }
    Err(IoAdapterError::InvalidRedirect(format!(
        "provider redirect host is not allowed: {host}"
    )))
}

fn parse_url(input: &str) -> Result<Url, IoAdapterError> {
    if input.len() > MAX_URL_BYTES {
        return Err(IoAdapterError::InvalidUrl("URL is too long".to_owned()));
    }
    validate_url(Url::parse(input).map_err(|error| IoAdapterError::InvalidUrl(error.to_string()))?)
}

fn validate_url(mut url: Url) -> Result<Url, IoAdapterError> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(IoAdapterError::UrlPolicy(
            "only http and https are allowed".to_owned(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(IoAdapterError::UrlPolicy(
            "embedded credentials are forbidden".to_owned(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| IoAdapterError::InvalidUrl("URL has no host".to_owned()))?;
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return Err(IoAdapterError::UrlPolicy(
            "localhost names are forbidden".to_owned(),
        ));
    }
    url.set_fragment(None);
    Ok(url)
}

fn resolve_checked(url: &Url, allow_loopback: bool) -> Result<Vec<SocketAddr>, IoAdapterError> {
    let host = url
        .host()
        .ok_or_else(|| IoAdapterError::InvalidUrl("URL has no host".to_owned()))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| IoAdapterError::InvalidUrl("URL has no usable port".to_owned()))?;
    let mut addresses = match host {
        Host::Ipv4(address) => vec![SocketAddr::new(IpAddr::V4(address), port)],
        Host::Ipv6(address) => vec![SocketAddr::new(IpAddr::V6(address), port)],
        Host::Domain(domain) => (domain, port)
            .to_socket_addrs()
            .map_err(|_| IoAdapterError::DnsResolution(domain.to_owned()))?
            .collect(),
    };
    addresses.sort_unstable();
    addresses.dedup();
    if addresses.is_empty() {
        return Err(IoAdapterError::DnsResolution(
            url.host_str().unwrap_or_default().to_owned(),
        ));
    }
    if addresses
        .iter()
        .any(|address| !address_allowed(address.ip(), allow_loopback))
    {
        return Err(IoAdapterError::UrlPolicy(format!(
            "host resolves to a non-public address: {}",
            url.host_str().unwrap_or_default()
        )));
    }
    Ok(addresses)
}

fn address_allowed(address: IpAddr, allow_loopback: bool) -> bool {
    if allow_loopback && address.is_loopback() {
        return true;
    }
    match address {
        IpAddr::V4(address) => !forbidden_ipv4(address),
        IpAddr::V6(address) => !forbidden_ipv6(address),
    }
}

fn forbidden_ipv4(address: Ipv4Addr) -> bool {
    let [a, b, c, _] = address.octets();
    a == 0
        || a == 10
        || a == 127
        || a >= 224
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 0)
        || (a == 192 && b == 168)
        || (a == 198 && matches!(b, 18 | 19 | 51))
        || (a == 203 && b == 0 && c == 113)
}

fn forbidden_ipv6(address: Ipv6Addr) -> bool {
    if let Some(mapped) = address.to_ipv4_mapped() {
        return forbidden_ipv4(mapped);
    }
    let segments = address.segments();
    address.is_unspecified()
        || address.is_loopback()
        || address.is_multicast()
        || address.is_unique_local()
        || address.is_unicast_link_local()
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || (segments[0] & 0xffc0) == 0xfec0
}

fn build_client(
    url: &Url,
    addresses: &[SocketAddr],
    connect_timeout: Duration,
    request_timeout: Duration,
) -> Result<Client, IoAdapterError> {
    let host = url
        .host_str()
        .ok_or_else(|| IoAdapterError::InvalidUrl("URL has no host".to_owned()))?;
    Client::builder()
        .tls_backend_rustls()
        .redirect(Policy::none())
        .no_proxy()
        .connect_timeout(connect_timeout)
        .timeout(request_timeout)
        .user_agent("CanISend/0.7 (+https://github.com/jxpeng98/CanISend)")
        .resolve_to_addrs(host, addresses)
        .build()
        .map_err(IoAdapterError::from)
}

fn decode_response(
    response: Response,
    source_url: String,
    final_url: String,
    redirect_chain: Vec<String>,
) -> Result<RemoteDocument, IoAdapterError> {
    let (original_bytes, declared) = read_response_bytes(response, MAX_REMOTE_SOURCE_BYTES)?;
    let kind = classify_content(&declared, &original_bytes)?;
    let normalized_text = match kind {
        RemoteDocumentKind::Html => Some(normalize_html(&original_bytes)?),
        RemoteDocumentKind::PlainText => Some(normalize_utf8_text(&original_bytes)?),
        RemoteDocumentKind::Pdf => None,
    };
    Ok(RemoteDocument {
        kind,
        original_bytes,
        normalized_text,
        source_url,
        final_url,
        content_type: canonical_content_type(kind).to_owned(),
        redirect_chain,
    })
}

fn decode_discovery_response(
    response: Response,
    source_url: String,
    final_url: String,
    redirect_chain: Vec<String>,
) -> Result<RemotePayload, IoAdapterError> {
    let (bytes, declared) = read_response_bytes(
        response,
        u64::try_from(crate::MAX_DISCOVERY_BATCH_BYTES).expect("discovery limit fits u64"),
    )?;
    let kind = classify_payload(&declared, &bytes)?;
    Ok(RemotePayload {
        kind,
        bytes,
        source_url,
        final_url,
        content_type: canonical_payload_content_type(kind).to_owned(),
        redirect_chain,
    })
}

fn read_response_bytes(
    mut response: Response,
    limit: u64,
) -> Result<(Vec<u8>, String), IoAdapterError> {
    if let Some(encoding) = response.headers().get(CONTENT_ENCODING) {
        let encoding = encoding.to_str().unwrap_or("invalid");
        if !encoding.eq_ignore_ascii_case("identity") {
            return Err(IoAdapterError::UnsupportedContentType(format!(
                "content encoding {encoding} is not enabled"
            )));
        }
    }
    if response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > limit)
    {
        return Err(IoAdapterError::InputTooLarge { limit });
    }
    let declared = response
        .headers()
        .get(CONTENT_TYPE)
        .map(|value| value.to_str().unwrap_or("invalid").to_owned())
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    validate_charset(&declared)?;
    let mut original_bytes = Vec::new();
    response
        .by_ref()
        .take(limit + 1)
        .read_to_end(&mut original_bytes)
        .map_err(IoAdapterError::ResponseRead)?;
    if u64::try_from(original_bytes.len()).expect("vector length fits u64") > limit {
        return Err(IoAdapterError::InputTooLarge { limit });
    }
    Ok((original_bytes, declared))
}

fn validate_charset(content_type: &str) -> Result<(), IoAdapterError> {
    for parameter in content_type.split(';').skip(1) {
        let parameter = parameter.trim();
        if let Some(charset) = parameter.strip_prefix("charset=")
            && !charset.trim_matches('"').eq_ignore_ascii_case("utf-8")
        {
            return Err(IoAdapterError::UnsupportedContentType(format!(
                "unsupported charset {charset}"
            )));
        }
    }
    Ok(())
}

fn classify_content(declared: &str, bytes: &[u8]) -> Result<RemoteDocumentKind, IoAdapterError> {
    if bytes.is_empty() {
        return Err(IoAdapterError::TextUnavailable);
    }
    let media_type = declared
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let sniffed = if bytes.starts_with(b"%PDF-") {
        RemoteDocumentKind::Pdf
    } else {
        let text = std::str::from_utf8(bytes).map_err(|_| IoAdapterError::InvalidTextEncoding)?;
        let prefix = text
            .chars()
            .take(2048)
            .collect::<String>()
            .to_ascii_lowercase();
        if ["<!doctype html", "<html", "<head", "<body"]
            .iter()
            .any(|marker| prefix.contains(marker))
        {
            RemoteDocumentKind::Html
        } else {
            RemoteDocumentKind::PlainText
        }
    };
    let expected = match media_type.as_str() {
        "application/pdf" => Some(RemoteDocumentKind::Pdf),
        "text/html" | "application/xhtml+xml" => Some(RemoteDocumentKind::Html),
        "text/plain" | "text/markdown" => Some(RemoteDocumentKind::PlainText),
        "application/octet-stream" | "" => None,
        _ => {
            return Err(IoAdapterError::UnsupportedContentType(media_type));
        }
    };
    if expected.is_some_and(|expected| expected != sniffed) {
        return Err(IoAdapterError::UnsupportedContentType(format!(
            "declared {media_type}, but content sniffing found {}",
            canonical_content_type(sniffed)
        )));
    }
    Ok(expected.unwrap_or(sniffed))
}

fn classify_payload(declared: &str, bytes: &[u8]) -> Result<RemotePayloadKind, IoAdapterError> {
    if bytes.is_empty() {
        return Err(IoAdapterError::TextUnavailable);
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| IoAdapterError::InvalidTextEncoding)?;
    let first = text.trim_start().chars().next();
    let sniffed = match first {
        Some('<') => RemotePayloadKind::Xml,
        Some('{' | '[') if serde_json::from_str::<serde_json::Value>(text).is_ok() => {
            RemotePayloadKind::Json
        }
        _ => {
            return Err(IoAdapterError::UnsupportedContentType(
                "body is neither JSON nor XML".to_owned(),
            ));
        }
    };
    let media_type = declared
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let expected = if media_type == "application/json" || media_type.ends_with("+json") {
        Some(RemotePayloadKind::Json)
    } else if matches!(
        media_type.as_str(),
        "application/xml" | "text/xml" | "application/rss+xml" | "application/atom+xml"
    ) || media_type.ends_with("+xml")
    {
        Some(RemotePayloadKind::Xml)
    } else if matches!(media_type.as_str(), "application/octet-stream" | "") {
        None
    } else {
        return Err(IoAdapterError::UnsupportedContentType(media_type));
    };
    if expected.is_some_and(|expected| expected != sniffed) {
        return Err(IoAdapterError::UnsupportedContentType(format!(
            "declared {media_type}, but payload sniffing found {}",
            canonical_payload_content_type(sniffed)
        )));
    }
    Ok(expected.unwrap_or(sniffed))
}

fn normalize_html(bytes: &[u8]) -> Result<String, IoAdapterError> {
    std::str::from_utf8(bytes).map_err(|_| IoAdapterError::InvalidTextEncoding)?;
    let rendered = html2text::from_read(bytes, 1000)
        .map_err(|error| IoAdapterError::Html(error.to_string()))?;
    if rendered.len() > MAX_NORMALIZED_TEXT_BYTES {
        return Err(IoAdapterError::InputTooLarge {
            limit: u64::try_from(MAX_NORMALIZED_TEXT_BYTES).expect("limit fits u64"),
        });
    }
    normalize_utf8_text(rendered.as_bytes())
}

const fn canonical_content_type(kind: RemoteDocumentKind) -> &'static str {
    match kind {
        RemoteDocumentKind::Html => "text/html; charset=utf-8",
        RemoteDocumentKind::PlainText => "text/plain; charset=utf-8",
        RemoteDocumentKind::Pdf => "application/pdf",
    }
}

const fn canonical_payload_content_type(kind: RemotePayloadKind) -> &'static str {
    match kind {
        RemotePayloadKind::Json => "application/json; charset=utf-8",
        RemotePayloadKind::Xml => "application/xml; charset=utf-8",
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener},
        thread,
        time::Duration,
    };
    use url::Url;

    use super::{
        HttpFetcher, MAX_REMOTE_SOURCE_BYTES, RemoteDocumentKind, RemotePayloadKind,
        address_allowed, classify_content, normalize_html, parse_url, validate_allowed_host,
    };

    #[test]
    fn url_and_address_policy_rejects_credentials_and_non_public_ranges() {
        assert!(parse_url("file:///tmp/advert").is_err());
        assert!(parse_url("https://user:secret@example.com/job").is_err());
        assert!(parse_url("https://localhost/job").is_err());
        for address in [
            Ipv4Addr::new(0, 0, 0, 0),
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv4Addr::new(127, 0, 0, 1),
            Ipv4Addr::new(169, 254, 1, 1),
            Ipv4Addr::new(172, 16, 0, 1),
            Ipv4Addr::new(192, 168, 0, 1),
            Ipv4Addr::new(100, 64, 0, 1),
            Ipv4Addr::new(224, 0, 0, 1),
        ] {
            assert!(!address_allowed(IpAddr::V4(address), false));
        }
        assert!(address_allowed(
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            false
        ));
        assert!(!address_allowed(IpAddr::V6(Ipv6Addr::LOCALHOST), false));
        assert!(!address_allowed(
            IpAddr::V6("fc00::1".parse().expect("IPv6 fixture")),
            false
        ));
    }

    #[test]
    fn provider_redirect_host_policy_is_exact_and_case_insensitive() {
        let accepted =
            Url::parse("https://API.LEVER.CO/v0/postings/example?mode=json").expect("provider URL");
        validate_allowed_host(&accepted, &["api.lever.co", "api.eu.lever.co"])
            .expect("case-insensitive exact host");
        for rejected in [
            "https://api.lever.co.example.invalid/jobs",
            "https://evil.example/jobs",
        ] {
            let rejected = Url::parse(rejected).expect("rejected URL");
            assert!(
                validate_allowed_host(&rejected, &["api.lever.co", "api.eu.lever.co"]).is_err()
            );
        }
        assert!(validate_allowed_host(&accepted, &[]).is_err());
    }

    #[test]
    fn content_sniffing_rejects_misleading_mime_and_html_is_normalized() {
        assert_eq!(
            classify_content("application/pdf", b"%PDF-1.7\n").expect("PDF"),
            RemoteDocumentKind::Pdf
        );
        assert!(classify_content("application/pdf", b"plain text").is_err());
        assert!(classify_content("text/plain", b"<!doctype html><p>text</p>").is_err());
        let text = normalize_html(
            b"<!doctype html><html><head><script>secret()</script></head><body><h1>Role</h1><p>Teach &amp; research.</p></body></html>",
        )
        .expect("HTML normalization");
        assert!(text.contains("Role"));
        assert!(text.contains("Teach & research."));
        assert!(!text.contains("secret"));
    }

    #[test]
    fn local_server_covers_redirect_size_mime_timeout_and_private_address_policy() {
        let body = "<!doctype html><html><body><h1>Economics role</h1></body></html>";
        let (url, server) = serve(vec![
            "HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ),
        ]);
        let fetched = HttpFetcher::allowing_loopback_for_tests(Duration::from_secs(1))
            .fetch(&format!("{url}/start"))
            .expect("redirected HTML fetch");
        server.join().expect("redirect server");
        assert_eq!(fetched.kind, RemoteDocumentKind::Html);
        assert_eq!(fetched.redirect_chain.len(), 1);
        assert!(
            fetched
                .normalized_text
                .expect("normalized HTML")
                .contains("Economics role")
        );

        assert!(
            HttpFetcher::new()
                .fetch("http://127.0.0.1:9/private")
                .is_err()
        );

        let (url, server) = serve(vec![format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            MAX_REMOTE_SOURCE_BYTES + 1
        )]);
        assert!(
            HttpFetcher::allowing_loopback_for_tests(Duration::from_secs(1))
                .fetch(&url)
                .is_err()
        );
        server.join().expect("size server");

        let body = "<!doctype html><html><body>misleading</body></html>";
        let (url, server) = serve(vec![format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )]);
        assert!(
            HttpFetcher::allowing_loopback_for_tests(Duration::from_secs(1))
                .fetch(&url)
                .is_err()
        );
        server.join().expect("MIME server");

        let (url, server) = serve(vec![
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 100\r\nConnection: close\r\n\r\nshort"
                .to_owned(),
        ]);
        assert!(
            HttpFetcher::allowing_loopback_for_tests(Duration::from_secs(1))
                .fetch(&url)
                .is_err()
        );
        server.join().expect("truncation server");

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("timeout listener");
        let address = listener.local_addr().expect("timeout address");
        let server = thread::spawn(move || {
            let (_stream, _) = listener.accept().expect("timeout request");
            thread::sleep(Duration::from_millis(200));
        });
        assert!(
            HttpFetcher::allowing_loopback_for_tests(Duration::from_millis(25))
                .fetch(&format!("http://{address}/slow"))
                .is_err()
        );
        server.join().expect("timeout server");
    }

    #[test]
    fn discovery_transport_accepts_only_bounded_matching_json_or_xml() {
        let json = r#"{"jobs":[]}"#;
        let xml = r#"<?xml version="1.0"?><rss version="2.0"></rss>"#;
        let (url, server) = serve(vec![
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
                json.len()
            ),
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/rss+xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{xml}",
                xml.len()
            ),
        ]);
        let fetcher = HttpFetcher::allowing_loopback_for_tests(Duration::from_secs(1));
        assert_eq!(
            fetcher
                .fetch_discovery(&format!("{url}/jobs.json"))
                .expect("JSON payload")
                .kind,
            RemotePayloadKind::Json
        );
        assert_eq!(
            fetcher
                .fetch_discovery(&format!("{url}/jobs.xml"))
                .expect("XML payload")
                .kind,
            RemotePayloadKind::Xml
        );
        server.join().expect("payload server");

        let (url, server) = serve(vec![format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{xml}",
            xml.len()
        )]);
        assert!(fetcher.fetch_discovery(&url).is_err());
        server.join().expect("mismatched payload server");
    }

    fn serve(responses: Vec<String>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("test listener");
        let address = listener.local_addr().expect("test address");
        let server = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().expect("test request");
                let mut request = [0_u8; 4096];
                let _ = stream.read(&mut request).expect("read test request");
                stream
                    .write_all(response.as_bytes())
                    .expect("write test response");
                stream.flush().expect("flush test response");
            }
        });
        (format!("http://{address}"), server)
    }
}
