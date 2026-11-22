---
type: Playbook
title: Flatpak build and releases
description: How Nuntius builds offline with vendored crates on phosh-toolkit's Flatpak workflow, keeps the crates in step with upstream, and publishes stable, nightly and rolling releases for phosh-apps.
tags: [flatpak, build, ci, release, cargo]
generated: { by: claude-code/2.1.317, at: 2026-11-16T23:00:00Z }
sources:
  - id: flathub-fractal
    resource: https://github.com/flathub/org.gnome.Fractal/blob/master/org.gnome.Fractal.json
    title: Fractal's Flathub manifest (cargo sources, shell source appending cargo/config)
  - id: toolkit-flatpak
    resource: ../../phosh-toolkit/docs/flatpak-workflow.md
    title: phosh-toolkit, Flatpak build and install workflow
  - id: toolkit-releases
    resource: ../../phosh-toolkit/docs/releases.md
    title: phosh-toolkit, releases, changelog and screenshots
---

# Manifest

`build-aux/de.nexumia.Nuntius.json` builds without network, the way
Fractal's Flathub manifest does[^flathub-fractal]:

- `nuntius-cargo-sources.json` and `grass-cargo-sources.json` (generated,
  never edited by hand) download every crate into `cargo/vendor` of the
  module and write `cargo/config` with the source replacement.
- grass reads it through `CARGO_HOME=/run/build/grass/cargo` and builds
  with `--offline --locked`.
- The app module appends `cargo/config` to upstream's
  `.cargo/config.toml` (a `type: shell` source), because meson sets its
  own `CARGO_HOME`; cargo finds the file from meson's build directory.
- Each third-party module installs its licence text into
  `/app/share/licenses/de.nexumia.Nuntius/<module>/`; grass' `cleanup`
  removes only its binary. List: [dependencies](dependencies.md).
  `post-install` of a meson module runs in meson's build directory, so
  libshumate's licence is `../COPYING` (a plain `COPYING` failed the
  build, locally and in CI run 44031343579, 2026-11-17); autotools
  (protobuf-c) builds in the source tree.
- Every git source has `x-checker-data` (phosh-toolkit's update check).
  After the checker bumps grass, regenerate the cargo sources.

# Keeping the crates in step

Run after every change of `Cargo.lock` or of the grass module:

```sh
build-aux/update-cargo-sources.sh
```

It fetches flatpak-cargo-generator at a pinned commit, installs its
Python packages from `build-aux/cargo-sources-requirements.txt` into a
temporary venv, and needs `github.com` and `pypi.org`. The CI job
*Cargo sources* runs it with `--check` and fails with a hint when a file
is out of date.

Upstream merges: the daily pull request `main` -> `phosh` cannot carry
the regenerated files (its head is the mirror). When upstream changed
`Cargo.lock`, *Cargo sources* and the builds fail on it; then merge
`main` into a branch from `phosh`, run the script, commit, and open
that branch as the merge pull request instead
([fork workflow](fork-workflow.md)).

# CI and channels

`.github/workflows/flatpak.yml` calls phosh-toolkit's reusable workflow
(`release: true`, `nightly: true`, `publish-repo:
nexumia-de/phosh-apps`)[^toolkit-flatpak] with
`build-timeout-minutes: 120`: the builds took 25–32 minutes per
architecture with a warm cache under the old workflow (2026-11-08), more
than the toolkit's default 30. That input came with
nexumia-de/phosh-toolkit#51 (merged 2026-11-17).

| Channel | App | Fed by |
|---|---|---|
| Stable | `de.nexumia.Nuntius` | tag `v<version>`; the first `<release>` of the installed metainfo must have that version |
| Nightly | `de.nexumia.Nuntius.Devel` (meson `-Dprofile=development`, debug cargo build) | every push to `phosh` |
| Rolling (until the first stable release) | `de.nexumia.Nuntius` | every push to `phosh`, release `rolling` |

The toolkit's nightly build passes `-Dbuild_label=…`; meson rejects
unknown options, so `meson.options` defines `build_label` (unused,
Fractal's development profile already shows the commit).

# Releases

Nuntius has its own version line
([decision](decisions/version-scheme.md)) and releases with
phosh-toolkit's workflow[^toolkit-releases]:

- `data/de.nexumia.Nuntius.releases.xml` holds the fork's `<release>`
  elements (`release.toml`: `metainfo`). Nobody edits them by hand.
- `build-aux/nuntius-releases.py version` gives meson the version
  (`meson.build`, after upstream's version lines), `merge` replaces the
  content of `<releases>` in the configured metainfo
  (`data/meson.build`). Tests: `python3 -m unittest discover -s
  build-aux/tests`, CI job *Build scripts*.
- A pull request users notice adds
  `changelog.d/<slug>.<feature|improvement|fix>.md`; others get the
  label `no-changelog` (check *Changelog* of
  `.github/workflows/release.yml`). The upstream sync pull request gets
  the label by itself.
- Mondays 13:47 UTC, or *Actions → Release → Run workflow*: the pull
  request "Release v<version>" collects the fragments. Merging it tags
  `v<version>`; the tag run publishes the stable app. Both run with the
  secret `PUBLISH_TOKEN`, which also serves as the release token.
- No screenshots are declared: a shot needs recorded data, which a
  Matrix client does not have without a login.

# First stable release

1. Run *Release* by hand (it uses the secret `PUBLISH_TOKEN`); it opens
   "Release v0.2.0" from the fragments in `changelog.d/`.
2. Merge it when its checks are green. *Tag release* creates `v0.2.0`,
   the tag run attaches the bundles, source bundles and the Flathub
   manifest to the release.
3. In phosh-apps' `apps.txt` replace `nexumia-de/nuntius rolling` with
   `nexumia-de/nuntius`.
4. Remove the jobs `rolling` and `notify` from `flatpak.yml` and delete
   the release and tag `rolling`.

# Status (2026-11-16)

Done: vendored sources (cover all 565 packages of `Cargo.lock` with
matching checksums; grass' file is identical to Flathub's), manifest,
CI, REUSE (`reuse lint` 6.2.0 passes), docs. Local offline build
(2026-11-17, GNOME 51 SDK, rust-stable 26.08, 4 cores, 15 GB): grass,
protobuf-c, libshumate and every crate of the app (both git
repositories included) built without network; the final `fractal` crate
was killed by the container's memory limit (rustc at 13.6 GB with thin
LTO, `codegen-units = 1` and full debug info). GitHub's runners build it
(16 GB).
