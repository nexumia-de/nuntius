---
type: Decision Record
title: Own version line for Nuntius
description: Nuntius counts its own semantic versions from 0.1.0 in a release list of its own, merged into the metainfo at build time, and releases with phosh-toolkit's changelog and release workflow; upstream's version and metainfo stay untouched.
tags: [decision, release, version, fork]
generated: { by: claude-code/2.1.330, at: 2026-11-22T00:00:00Z }
sources:
  - id: toolkit-releases
    resource: ../../../phosh-toolkit/docs/releases.md
    title: phosh-toolkit, releases, changelog and screenshots
  - id: release-py
    resource: ../../../phosh-toolkit/scripts/release.py
    title: phosh-toolkit, scripts/release.py (parse_version, latest_release, insert_release)
---

# Status

Accepted (2026-11-22). Issue #16 left the choice to the owner; the owner
asked the agent to decide by best practice and record why.

# Context

phosh-toolkit's Flatpak workflow publishes a tag `v<version>` only if
the first `<release>` of the installed metainfo has that version, and
its release workflow writes `<release>` elements and the version from
changelog fragments[^toolkit-releases]. In Nuntius both live in upstream
files (`meson.build`, `data/org.gnome.Fractal.metainfo.xml.in.in`,
version 14.3), which every upstream release changes.

Options of the issue:

1. Follow upstream's versions (tag `v14.3`, next tag after 14.4).
2. Fork patch versions on top of upstream's (`14.3.1`, …), written into
   upstream's metainfo by hand.
3. An own version line with the toolkit's release workflow.

Facts that decide it:

- `phosh` follows upstream's **development** branch, merged daily. A
  build of `phosh` is never Fractal 14.3; any number derived from 14.3
  would describe code that 14.3 does not contain.
- With 1, a fix of the fork (push notifications are the fork's reason
  to exist) reaches the stable app only when upstream releases, and a
  version can be tagged only once.
- With 2, each release is typed by hand into an upstream file, which
  conflicts with the next upstream release; `release.py` refuses
  versions that are not `MAJOR.MINOR.PATCH` and would compute `14.4.0`
  for the next feature, a number upstream will use[^release-py].
- The objection to 3 in the issue was the diff against upstream. It
  goes away when the fork's releases live in a file of their own.

# Decision

- Nuntius has its **own semantic version line**, starting at 0.1.0 (the
  existing tag `v0.1.0`, the first build of the fork). Tags are
  `v<version>`.
- The releases are in `data/de.nexumia.Nuntius.releases.xml`, a file
  only the fork has. `build-aux/nuntius-releases.py` gives meson the
  version (newest stable release) and, in `data/meson.build`, replaces
  the content of `<releases>` in the configured metainfo with the
  fork's. Upstream's `version: '14.3'` and metainfo template are not
  edited.
- Releases use the toolkit's workflow like the other apps:
  `release.toml` points `metainfo` at the release list,
  `changelog.d/<slug>.<feature|improvement|fix>.md` per pull request,
  a weekly release pull request, a tag when the owner merges it
  (`.github/workflows/release.yml`).
- The daily pull request `main` -> `phosh` gets the label
  `no-changelog` from the sync workflow; its head is the mirror and
  cannot carry a fragment. When a merge brings in an upstream
  **release**, the next fork pull request adds a fragment such as
  "Includes the changes of Fractal 14.4."
- No screenshots in `release.toml` yet: a shot needs a state with
  recorded data, and a Matrix client has none without a login. The
  metainfo keeps upstream's screenshots until then.

# Consequences

- The About dialog and GNOME Software show Nuntius' version (0.x) and
  the fork's release notes; Fractal's release history is no longer in
  the installed metainfo. The About dialog still says "Based on
  Fractal" and links to it.
- The diff against upstream is two short blocks (`meson.build` after
  the version lines, the metainfo input in `data/meson.build`) and new
  files; no conflict on upstream releases.
- The nightly app shows the newest stable version plus the commit; the
  toolkit adds its development `<release>` to the release list before
  the build.
- If screenshots are added later, `release.py` writes `<screenshots>`
  into the release list; `nuntius-releases.py` then has to carry them
  into the metainfo too.
- Rejected: `<releases type="external">` (AppStream's external release
  file): `release.py` reads `releases/release` below the root, and
  upstream's template would still need an edit.
