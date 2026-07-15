# Discovery Public API Adapters

CanISend discovery adapters are read-only importers for public published-job data. They use the
shared bounded GET transport, emit untrusted Lead v2 records, and have no account, application,
upload, portal, or private-API authority. Runtime and release tests use offline fixtures; these
external documentation links are not live-test dependencies.

## Verification Record

The endpoint contracts below were verified against the official vendor documentation on 2026-07-15.

### Greenhouse Job Board API

- Official documentation: <https://developers.greenhouse.io/job-board>
- Configured identifier: explicit lowercase `board_token`
- CanISend request: `GET https://boards-api.greenhouse.io/v1/boards/{board_token}/jobs?content=true`
- Authentication: none for the documented Job Board GET endpoints
- Accepted response root: an object containing a `jobs` list
- Mapped fields: `id`, `title`, `absolute_url`, `content`, `first_published`/`updated_at`,
  `application_deadline`, `location.name`, and optional `company_name`

CanISend does not expose or call Greenhouse's application submission endpoint:
`POST /v1/boards/{board_token}/jobs/{id}`. Questions, application fields, compliance structures,
metadata, and other vendor fields are not copied into discovery provenance or control responses.

### Lever Postings API

- Official documentation: <https://github.com/lever/postings-api>
- Configured identifier: explicit lowercase `site_id`; region is `global` or `eu`
- Global request: `GET https://api.lever.co/v0/postings/{site_id}?limit={max_leads}&mode=json`
- EU request: `GET https://api.eu.lever.co/v0/postings/{site_id}?limit={max_leads}&mode=json`
- Authentication: none for the public Postings API
- Accepted response root: a JSON list of published postings
- Mapped fields: `id`, `text`, `hostedUrl`, `descriptionPlain`/`openingPlain`/`description`, and
  `categories.location`

CanISend does not expose or call Lever's application endpoint:
`POST /v0/postings/{site_id}/{posting_id}`. It ignores `applyUrl`, application-form content, and
pagination-like response fields and makes exactly one bounded list request per configured refresh.

## Conformance Boundary

Every registered adapter provides one fixed adapter ID, one derived public GET URL, one redacted
source locator, one transport media contract, an exact final-endpoint check for public APIs, and one
mapper into Lead v2. Configuration for Greenhouse and Lever accepts identifiers rather than
arbitrary URLs, so a source cannot redirect the adapter to an application path, authenticated API,
or unrelated host.

Malformed identifiers, credential or auth fields, user-supplied API URLs, undocumented response
roots, unexpected redirects, non-JSON API responses, record-limit violations, and invalid required
job fields fail closed. A failed refresh may reuse only the last validated complete batch and
reports a body-free stable error code.
