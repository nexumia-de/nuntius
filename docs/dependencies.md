---
type: Reference
title: Bundled dependencies
description: Every module the Nuntius Flatpak bundles, the vendored Rust crates and the tools that generate their sources, with version and licence.
tags: [dependencies, licensing, flatpak]
generated: { by: claude-code/2.1.317, at: 2026-11-16T23:00:00Z }
sources:
  - id: grass
    resource: https://github.com/connorskees/grass/blob/e0bb9e2eabfc3a58e42b03089cd7b22c68d09d0b/LICENSE
    title: grass 0.13.4, LICENSE
  - id: protobuf-c
    resource: https://github.com/protobuf-c/protobuf-c/blob/4719fdd7760624388c2c5b9d6759eb6a47490626/LICENSE
    title: protobuf-c 1.5.2, LICENSE
  - id: libshumate
    resource: https://gitlab.gnome.org/GNOME/libshumate/-/blob/e345c805a0dfc46cdbff46cac287b0b60a386f36/COPYING
    title: libshumate 1.7.0, COPYING
  - id: generator
    resource: https://github.com/flatpak/flatpak-builder-tools/tree/9d86d336a39b9af38b86f114b6ce5820fe260f91/cargo
    title: flatpak-builder-tools, cargo/pyproject.toml and flatpak-cargo-generator.py (licence MIT)
  - id: deny
    resource: ../deny.toml
    title: Fractal's cargo-deny configuration, [licenses] allow
---

# Rules

Shared rules:
[licensing](../../phosh-toolkit/docs/licensing.md) and
[dependency updates](../../phosh-toolkit/docs/dependency-updates.md).
CI (phosh-toolkit's Flatpak workflow) fails if a module of
`build-aux/de.nexumia.Nuntius.json` other than the app is missing here
or installs no licence files into
`/app/share/licenses/de.nexumia.Nuntius/<module>/`.

Nuntius, like Fractal, is `GPL-3.0-or-later`; every licence below allows
redistribution with it. The GNOME runtime (GTK, libadwaita, GStreamer,
glycin, …) is not bundled; Flathub distributes it under its own terms.
The source bundle `de.nexumia.Nuntius-sources-<arch>.flatpak` next to
each release carries every source below, the vendored crates included.

# Modules

| Module | Version | Licence | Licence file | Shipped |
|---|---|---|---|---|
| `grass` | 0.13.4 (`e0bb9e2e`) | MIT[^grass] | `LICENSE` | No: Sass compiler for the build; its binary is removed by `cleanup`, the licence stays |
| `protobuf-c` | 1.5.2 (`4719fdd7`) | BSD-2-Clause[^protobuf-c] | `LICENSE` | Yes (library for libshumate's vector renderer) |
| `libshumate` | 1.7.0 (`e345c805`) | LGPL-2.1-or-later[^libshumate] | `COPYING` | Yes (maps of location messages) |

The versions are the ones upstream's development manifest
`build-aux/org.gnome.Fractal.Devel.json` uses. Updates: every source has
`x-checker-data` for phosh-toolkit's update check.

# Rust crates

`build-aux/nuntius-cargo-sources.json` (from `Cargo.lock`, 565 crates
from crates.io and two git repositories, matrix-rust-sdk and ruma) and
`build-aux/grass-cargo-sources.json` (grass' `Cargo.lock`) vendor the
crates, so the build needs no network. Upstream allows only these crate
licences (`deny.toml`, checked by its CI with cargo-deny)[^deny]:
Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause,
BSL-1.0, GPL-3.0-or-later, ISC, MIT, MPL-2.0, Unicode-3.0, Zlib. The
crate versions follow upstream's `Cargo.lock`.

# Tools (not shipped)

`build-aux/update-cargo-sources.sh` generates both files with:

| Tool | Version | Licence |
|---|---|---|
| flatpak-cargo-generator (flatpak-builder-tools) | commit `9d86d336` (2026-10-25) | MIT[^generator] |
| aiohttp and its dependencies, PyYAML, tomlkit | exact versions in `build-aux/cargo-sources-requirements.txt` | Apache-2.0 (aiohttp, aiosignal, frozenlist, multidict, propcache, yarl), PSF-2.0 (aiohappyeyeballs), MIT (attrs, PyYAML, tomlkit), BSD-3-Clause (idna); read from the installed package metadata |

[^grass]: grass 0.13.4, LICENSE
[^protobuf-c]: protobuf-c 1.5.2, LICENSE
[^libshumate]: libshumate 1.7.0, COPYING
[^generator]: flatpak-builder-tools, cargo/pyproject.toml and flatpak-cargo-generator.py (licence MIT)
[^deny]: Fractal's cargo-deny configuration, [licenses] allow
