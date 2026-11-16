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
than the toolkit's default 30. That input comes with
nexumia-de/phosh-toolkit#51, which must be merged first.

| Channel | App | Fed by |
|---|---|---|
| Stable | `de.nexumia.Nuntius` | tag `v<version>`; the newest metainfo `<release>` must have that version |
| Nightly | `de.nexumia.Nuntius.Devel` (meson `-Dprofile=development`, debug cargo build) | every push to `phosh` |
| Rolling (until the first stable release) | `de.nexumia.Nuntius` | every push to `phosh`, release `rolling` |

The toolkit's nightly build passes `-Dbuild_label=…`; meson rejects
unknown options, so `meson.options` defines `build_label` (unused,
Fractal's development profile already shows the commit).

No `release.toml` and no `changelog.d/` yet: phosh-toolkit's release
workflow writes the version into `meson.build` and `<release>` entries
into the metainfo, both upstream files, so it would collide with every
upstream release merge. Which versions the fork uses is open (issue
linked from the pull request of #3).

# First stable release

1. Merge nexumia-de/phosh-toolkit#51, then this repository's pull request
   for #3; wait for the `phosh` run to be green (it also publishes the
   nightly app and `rolling`).
2. Tag the merge commit with the version of the newest metainfo
   `<release>` (`data/org.gnome.Fractal.metainfo.xml.in.in`, 14.3 on
   2026-11-16) or a fork version added there:
   `git tag v14.3 <commit> && git push origin v14.3`.
3. The tag run attaches the bundles, source bundles and the Flathub
   manifest to the release `v14.3`.
4. In phosh-apps' `apps.txt` replace `nexumia-de/nuntius rolling` with
   `nexumia-de/nuntius`.
5. Remove the jobs `rolling` and `notify` from `flatpak.yml` and delete
   the release and tag `rolling` (and the asset-less `v0.1.0`), so
   GitHub's "latest release" is always the newest tag.

# Status (2026-11-16)

Done: vendored sources (cover all 565 packages of `Cargo.lock` with
matching checksums; grass' file is identical to Flathub's), manifest,
CI, REUSE (`reuse lint` 6.2.0 passes), docs. Local build: see the log.
