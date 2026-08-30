# Beta.1 package-channel candidate design

## Boundary

This task reuses the existing verified-byte generator and changes only the shared renderer metadata
needed for the current generic framework. It produces local review candidates, not a publication
record or external repository submission.

## Owning metadata drift

`render_channel_manifest_files` supplies one static academic-job description to Homebrew, Scoop,
and WinGet and one academic-only WinGet tag. Changing only the generated Beta files would fail the
repository regeneration gate. Changing the renderer without a version boundary would rewrite the
audited 0.7 candidate history.

The minimum root fix follows the existing version-aware license pattern:

~~~text
version < 1.0.0-alpha.6
  -> historical academic description and tags
version >= 1.0.0-alpha.6
  -> generic evidence-application description and tags
~~~

One small metadata selector feeds all three renderers. Historical files remain untouched; the new
Beta.1 output becomes the first checked-in candidate using the current product boundary.

## Source and output chain

~~~text
qualified ledger Q
  -> independent public asset directory A
  -> complete release verification V
  -> deterministic channel generator G
  -> candidate source plus five manifests C
  -> exact directory revalidation R
  -> protected source gate and PR M
~~~

`Q/A/V/G/C/R/M` must agree on tag `v1.0.0-beta.1`, source
`6e1397b79031cad54e794ccdc9edca2153f23b3e`, and manifest SHA-256
`2435c335f2edd31e1a59afd4065380112f4e24924f68f76a26be84acef0041f8`.

## Generated tree

~~~text
packaging/candidates/v1.0.0-beta.1/
├── candidate-source.json
├── homebrew/Casks/canisend.rb
├── scoop/bucket/canisend.json
└── winget/manifests/p/PengJiaxin/CanISend/1.0.0-beta.1/
    ├── PengJiaxin.CanISend.installer.yaml
    ├── PengJiaxin.CanISend.locale.en-US.yaml
    └── PengJiaxin.CanISend.yaml
~~~

The source record remains `candidate_only: true` and `publication_authorized: false`. The three
package formats reuse exact public archive URLs, digests, and nested executable paths.

## Failure and rollback

- Asset or manifest mismatch: stop before generation.
- Existing output directory: stop rather than merge or overwrite.
- Partial generation or exact revalidation failure: remove only the newly created uncommitted
  Beta.1 candidate tree and fix the owning renderer or source input.
- Protected CI failure: fix the owning generator, deterministic output, or projection; never edit
  hashes by hand or rewrite public assets.

## Compatibility

No runtime or schema compatibility path is added. The single rendering boundary preserves immutable
pre-generic candidate history while applying the accepted 1.0 product identity to current output.
