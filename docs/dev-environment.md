---
type: Concept
title: Development environment
description: Building Nuntius with flatpak-builder on the GNOME 51 SDK; hosts the cloud environment can and cannot reach.
tags: [build, flatpak, environment]
generated: { by: claude-code/2.1.317, at: 2026-11-04T18:00:00Z }
---

# Development environment

## Build

Fractal needs GTK ≥ 4.20.2 and libadwaita ≥ 1.8 (`meson.build`), newer
than Ubuntu 24.04 has. Build only with Flatpak:

```sh
flatpak-builder --user --install-deps-from=flathub --force-clean build-dir build-aux/de.nexumia.Nuntius.json
```

`build-aux/de.nexumia.Nuntius.json` is upstream's development manifest
on the stable GNOME 51 runtime from Flathub (upstream uses GNOME's
nightly runtime), without the development profile. Cargo fetches crates
during the build (`build-args: --share=network`), as upstream does.

CI (`.github/workflows/flatpak.yml`) builds x86_64 and aarch64 bundles
on every pull request and push to `phosh`.

## Distribution

Every push to `phosh` replaces the bundles of the GitHub release
`rolling` (tag moved to the pushed commit) and, with the secret
`PUBLISH_TOKEN`, notifies `nexumia-de/phosh-apps`. phosh-apps lists
Nuntius as `nexumia-de/nuntius rolling` in `apps.txt` and publishes it
as `de.nexumia.Nuntius` in the nexumia Flatpak repository; the release
must exist before that line is added, or phosh-apps' publish run fails.
Stable tag releases and a nightly `.Devel` app come with phosh-toolkit's
reusable workflow, which needs vendored crates first (#3).

## Hosts (cloud environment, 2026-11-04)

Reachable: `gitlab.gnome.org` (upstream), `dl.flathub.org`,
`index.crates.io`, `static.crates.io` (crate files), and
`nightly.gnome.org` (GNOME's nightly Flatpak runtime, which upstream's
development manifest uses; allowed by the owner on 2026-11-04), and
`unifiedpush.kde.org` (KDE's ntfy server for push notifications,
allowed by the owner on 2026-11-04), `pkgs.alpinelinux.org`,
`dl-cdn.alpinelinux.org` (Alpine package search, indexes and packages)
and `gitlab.alpinelinux.org` (allowed 2026-11-04, but it answers
scripted requests with an anti-bot challenge; read APKBUILDs from the
packages or the index instead).

A full local build was not tried yet: it needs the GNOME 51 SDK and the
Rust extension (several GB) and cargo network access inside the build
sandbox through the proxy.
