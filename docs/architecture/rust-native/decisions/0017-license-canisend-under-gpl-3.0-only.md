# ADR-RN-0017: License CanISend under GPL-3.0-only

- Status: Accepted
- Date: 2026-08-02
- Decision owner: CanISend maintainer

## Context

CanISend was previously distributed under the MIT License. The project now needs a single,
machine-readable license identity that keeps improvements to the application available as free
software, is visible in the interactive desktop product, and is carried through every release
channel without rewriting the legal facts of already published versions.

CanISend also embeds and links third-party components under compatible licenses. Relicensing the
project does not replace those upstream notices or change the terms attached to historical tags.

## Decision

CanISend-authored source, documentation, and distributable assets are licensed under GNU General
Public License version 3 only, expressed with the SPDX identifier `GPL-3.0-only`, unless a file or
directory states different terms.

The repository will:

- keep the complete GPLv3 text in the root `LICENSE` file;
- declare `GPL-3.0-only` in Rust and JavaScript package metadata;
- include the project license and third-party notices in native bundles;
- expose copyright, redistribution, warranty, license, and source-code notices in the desktop UI;
- render `GPL-3.0-only` in package-manager metadata beginning with `v1.0.0-alpha.6`; and
- keep package metadata for earlier release tags reproducible under the MIT terms present in those
  source trees.

Corresponding source for a release is the matching Git tag in the public repository. Release
qualification must bind the distributed artifacts, license files, notices, package metadata, and
source identity before publication.

## Consequences

Anyone conveying CanISend or a modified version must follow GPLv3's source-availability, notice,
and license obligations. The `-only` choice does not automatically permit use under a later GPL
version.

Dependency and artifact checks must continue to preserve compatible upstream license texts and
attributions. Historical releases are not retroactively relabeled, and their checked-in candidate
manifests remain reproducible. A future project-license change requires a new ADR and an explicit
release boundary.
