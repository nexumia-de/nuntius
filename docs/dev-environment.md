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
nightly runtime), without the development profile. The cargo crates are
vendored, so the build needs no network
([Flatpak build and releases](flatpak.md)).

CI (`.github/workflows/flatpak.yml`, phosh-toolkit's reusable workflow)
builds x86_64 and aarch64 bundles on every pull request, push to `phosh`
and tag `v<version>`.

## Distribution

Channels and the first stable release: [Flatpak build and
releases](flatpak.md). Until then, every push to `phosh` replaces the bundles of the GitHub release
`rolling` (tag moved to the pushed commit) and, with the secret
`PUBLISH_TOKEN`, notifies `nexumia-de/phosh-apps`. phosh-apps lists
Nuntius as `nexumia-de/nuntius rolling` in `apps.txt` and publishes it
as `de.nexumia.Nuntius` in the nexumia Flatpak repository; the release
must exist before that line is added, or phosh-apps' publish run fails.
Stable tag releases and the nightly `.Devel` app come from
phosh-toolkit's reusable workflow (#3).

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

`github.com` and `pypi.org` are reachable too (2026-11-16, needed by
`build-aux/update-cargo-sources.sh`). The GNOME 51 SDK uses the
freedesktop 26.08 base, so its Rust extension is
`org.freedesktop.Sdk.Extension.rust-stable//26.08`.
