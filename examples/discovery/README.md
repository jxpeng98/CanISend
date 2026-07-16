# Stage 4 Discovery Examples

These public, synthetic fixtures demonstrate the Stage 4 discovery contracts without containing applicant data,
credentials, connector identifiers, email addresses, or real private exports.

## Configure Read-Only Sources

Copy `discovery-sources.example.yaml` to the private workspace root as `discovery-sources.yaml`, replace every
placeholder identifier or URL, then run:

```bash
canisend discovery refresh --workspace <private-workspace>
```

RSS/Atom, Greenhouse, and Lever sources produce candidate leads only. Greenhouse and Lever accept published board
identifiers, not arbitrary API URLs or credentials. See `docs/discovery-adapters.md` in the source repository for
the exact read-only endpoint contract.

## Import Local Or Host Results

Import the synthetic CSV export:

```bash
canisend discovery import \
  --workspace <private-workspace> \
  --input examples/discovery/local-leads.example.csv \
  --source-name "Saved Search Export"
```

Import the normalized host-agent handoff:

```bash
canisend discovery import-search \
  --workspace <private-workspace> \
  --input examples/discovery/normalized-search.example.json
```

Both commands merge into `job_leads/catalog.json`. Choose a candidate by stable ID:

```bash
canisend new-job-from-lead \
  --workspace <private-workspace> \
  --leads-file job_leads/catalog.json \
  --lead-id <lead_id> \
  --institution "<institution>"
```

The selected lead is still not a full advert. Paste the advert or use `new-job --advert-file` / the explicitly
approved single-URL `new-job --source-url ... --fetch-url` intake before relying on parsing or application drafts.

`greenhouse-list.fixture.json` and `lever-list.fixture.json` are offline public-response shapes used by packaged
adapter smoke tests. They are not sent to either service.
