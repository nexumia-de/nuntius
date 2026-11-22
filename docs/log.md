# Directory Update Log

## 2026-11-22
* **Fix**: [Flatpak build and releases](flatpak.md): `build-aux/nuntius-cargo-sources.json` regenerated; the upstream merge #13 changed `Cargo.lock` (e.g. `mime` 0.3.18) without it, so the offline Flatpak build failed.
* **Update**: [Flatpak build and releases](flatpak.md): the release workflow uses the existing secret `PUBLISH_TOKEN` as its release token, as the owner decided; no separate `RELEASE_TOKEN`.
* **Creation**: [App icon](decisions/app-icon.md) (#5): own icon `data/icons/de.nexumia.Nuntius*.svg` (app, nightly, symbolic), installed through `icon_base_id` in `data/icons/meson.build`.
* **Creation**: [Version scheme](decisions/version-scheme.md) (#16): own version line from 0.1.0 in `data/de.nexumia.Nuntius.releases.xml`, merged into the metainfo by `build-aux/nuntius-releases.py`; `release.toml`, `changelog.d/` and `.github/workflows/release.yml` (phosh-toolkit's release workflow); the upstream sync pull request gets the label `no-changelog`. [Flatpak build and releases](flatpak.md), [fork workflow](fork-workflow.md) and [owner checks](owner-checks.md) updated.
* **Creation**: [Owner checks](owner-checks.md) (#2): the checks of push notifications that need the phone, and whether KDE's push server may be used.
* **Update**: [Push notifications](push-notifications.md): unit tests for the gateway URL, the gateway answer, the language tag and the choice of the distributor; endpoints that are not `http(s)` are refused.

## 2026-11-17
* **Fix**: [Flatpak build and releases](flatpak.md): libshumate's licence path is `../COPYING` (meson `post-install` runs in the build directory).

## 2026-11-16
* **Creation**: [Flatpak build and releases](flatpak.md) and [bundled dependencies](dependencies.md) (#3): crates vendored with flatpak-cargo-generator (`build-aux/*-cargo-sources.json`, `build-aux/update-cargo-sources.sh`, CI check *Cargo sources*); manifest without build network, licence texts for grass, protobuf-c and libshumate, `x-checker-data` on every source; CI on phosh-toolkit's reusable workflow with stable tags, nightly `de.nexumia.Nuntius.Devel` and `build-timeout-minutes: 120` (nexumia-de/phosh-toolkit#51); `rolling` kept until the first stable tag; `meson.options` gets `build_label`.
* **Update**: [Development environment](dev-environment.md): offline build; `github.com` and `pypi.org` reachable; GNOME 51 needs the Rust extension branch 26.08.

## 2026-11-08
* **Update**: `AGENTS.md` imports `agents/core.md` and `agents/gtk-app.md` from phosh-toolkit instead of its whole `AGENTS.md`; repeated shared rules removed (toolkit decision "lean agent rules", to use fewer tokens per session).

## 2026-11-04
* **Update**: [Push notifications](push-notifications.md): fix for no notification after a UnifiedPush wake-up: the notification handler is registered before the first sync of a restored session, and `show_push` waits for the account notification setting instead of dropping the event while it loads.
* **Update**: [Push notifications](push-notifications.md): fix for Nuntius unregistering right after registering (race between "session ready" with the not yet loaded account setting and the setting's load); registrations are now serialized per session and a ready session is never unregistered. Diagnosed on the phone with `dbus-monitor` and KUnifiedPush's `/Management` interface: ntfy and D-Bus activation worked, the distributor held no client.
* **Update**: [Development environment](dev-environment.md), "Distribution": pushes to `phosh` publish the bundles as the rolling release `rolling` and notify phosh-apps, which lists `nexumia-de/nuntius rolling`; stable and nightly releases stay with #3.
* **Update**: [Push notifications](push-notifications.md): UnifiedPush D-Bus interface read from the KUnifiedPush package; first implementation in `src/unifiedpush/` (one file per job: `connector`, `distributor`, `pusher`, `endpoints`, and `mod.rs` keeping registrations in sync with the sessions) (registration per session, pusher with the KDE server's Matrix gateway, connector exported in `dbus_register`, 60 s hold on a push message so the catch-up sync shows notifications); Flatpak talks to `org.unifiedpush.Distributor.*`.
* **Update**: [Push notifications](push-notifications.md): KUnifiedPush is in Alpine edge `community` (26.08.2); its autostart file and default configuration (KDE's ntfy server) read from the package, dependency size estimated from the APKINDEX.
* **Update**: [Push notifications](push-notifications.md): `matrix-sdk-ui` 0.19.1's `NotificationClient` read (process setups, `get_notification`, cross-process lock needed in both processes); design proposal: single process, Nuntius started by D-Bus activation through upstream's existing service file.
* **Update**: [Push notifications](push-notifications.md): the owner chose KDE's ntfy server `unifiedpush.kde.org`; its health endpoint, Matrix gateway and anonymous-only configuration checked. [Development environment](dev-environment.md): the host is reachable.
* **Creation**: Fork of Fractal at upstream `9fbc7283` as Nuntius ([fork and name](decisions/fork-and-name.md)): `main` mirrors GNOME's GitLab, `phosh` carries the fork ([fork workflow](fork-workflow.md)); app id `de.nexumia.Nuntius` with upstream's file names kept; own Flatpak manifest on GNOME 51 and a build workflow for x86_64 and aarch64 ([development environment](dev-environment.md)); the push notification goal ([push notifications](push-notifications.md)).
