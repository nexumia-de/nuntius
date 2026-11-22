---
type: Index
title: Nuntius knowledge
description: Entry point of the Nuntius fork's knowledge bundle.
tags: [index]
generated: { by: claude-code/2.1.317, at: 2026-11-04T18:00:00Z }
---

# Nuntius

Matrix chat app for Phosh, a fork of Fractal. Shared knowledge of all
apps is in `../phosh-toolkit/docs/index.md`.

## Concepts

* [Fork workflow](fork-workflow.md) - Branches, the daily upstream sync, the small-diff rule and why nothing goes upstream.
* [Push notifications](push-notifications.md) - The goal of the fork: notifications through UnifiedPush and ntfy while the app is closed.
* [Development environment](dev-environment.md) - How to build (Flatpak only), blocked hosts.
* [Flatpak build and releases](flatpak.md) - Offline build with vendored crates, keeping them in step with upstream, stable/nightly/rolling channels, releases from changelog fragments and the first stable release.
* [Owner checks](owner-checks.md) - What only the owner can do or confirm: checks on the phone, secrets, the first stable release.
* [Report a problem](feedback.md) - Report a Problem… in the main menu: GitHub issues with the user's own token or through the filled-in issue form, code in `src/feedback/`.
* [Bundled dependencies](dependencies.md) - Modules, vendored crates and generator tools with versions and licences.

## Decision records

* [Fork and name](decisions/fork-and-name.md) - Why a fork of Fractal instead of a new client, and why it is called Nuntius with the app id de.nexumia.Nuntius.
* [Version scheme](decisions/version-scheme.md) - Why Nuntius counts its own versions in a release list of its own and uses the toolkit's release workflow.
* [App icon](decisions/app-icon.md) - Why the icon is a purple speech bubble with a wing, and which variants exist.

## Log

* [Update log](log.md)
