---
type: Concept
title: Push notifications
description: The fork's goal: Matrix notifications through UnifiedPush and ntfy while Nuntius is closed. Plan and open questions; nothing is implemented yet.
tags: [notifications, unifiedpush, ntfy, matrix]
generated: { by: claude-code/2.1.317, at: 2026-11-04T22:00:00Z }
sources:
  - id: honk
    title: Unified Push (Guido Günther's notes, Chatty with KUnifiedPush)
    url: https://honk.sigxcpu.org/piki/development/unified-push
  - id: distributors
    title: UnifiedPush distributors
    url: https://unifiedpush.org/users/distributors/
  - id: kup
    title: kunifiedpush 1.0.0 is out (KDE blog)
    url: https://blogs.kde.org/2024/10/19/kunifiedpush-1.0.0-is-out/
  - id: continuwuity
    title: Continuwuity, push notifications guide
    url: https://continuwuity.org/guides/push
---

# Push notifications

## Problem

Fractal shows notifications only while it runs. On a phone the app is
closed most of the time, so messages arrive unnoticed.

## How Matrix push works

1. The client registers a *pusher* with its homeserver: a push gateway
   URL and a push key.
2. For a new event the homeserver posts to the gateway. With encrypted
   rooms the useful payload is only the room and event id.
3. The gateway forwards it to the device; the client fetches the event,
   decrypts it with its own keys and shows the notification.

## Pieces on the owner's side

- **Push server and gateway**: ntfy, which has an embedded Matrix push
  gateway (`/_matrix/push/v1/notify`)[^continuwuity]. The owner chose
  KDE's instance `https://unifiedpush.kde.org`, KUnifiedPush's default
  provider since 1.0[^kup]. Checked on 2026-11-04: `/v1/health` answers
  `{"healthy":true}`; `GET /_matrix/push/v1/notify` answers
  `{"unifiedpush":{"gateway":"matrix"}}`, so the Matrix gateway is on;
  `config.js` shows anonymous use only (no login, no sign-up, no
  reservations). Fallbacks: `ntfy.sh` (rate-limits hard[^continuwuity])
  or a self-hosted ntfy. With encrypted rooms the server sees only room
  and event ids.
- **Distributor on the phone**: KUnifiedPush, a Linux UnifiedPush
  distributor that can use an ntfy server[^distributors]. Chatty on
  Phosh is documented with it[^honk]. Alpine edge packages it in
  `community` as `kunifiedpush` 26.08.2-r0 (aarch64 APKINDEX, checked
  2026-11-04), so the owner installs it with `sudo apk add
  kunifiedpush`. The package ships
  `/etc/xdg/autostart/org.kde.kunifiedpush-distributor.desktop` without
  `OnlyShowIn`, so Phosh's session starts `kunifiedpush-distributor` at
  login, and `/etc/xdg/KDE/kunifiedpush-distributor.conf` with
  `[PushProvider] Type=Ntfy` and `[Ntfy] Url=https://unifiedpush.kde.org/`,
  the chosen server; no configuration is needed. Its settings page is a
  Plasma System Settings module (`kcm_push_notifications`), not
  available in Phosh; another server would be set in
  `~/.config/KDE/kunifiedpush-distributor.conf`. It pulls in Qt 6 and
  part of KDE Frameworks 6 (kcmutils, kio, kirigami, …): at most about
  570 MB for the whole dependency closure, less where Qt 6 is already
  installed.

## matrix-rust-sdk's notification client

Fractal pins `matrix-sdk` and `matrix-sdk-ui` 0.19.1 at git commit
`2d6248ed` (`Cargo.lock`). Read in the crates.io release 0.19.1 of
`matrix-sdk-ui` (`src/notification_client.rs`); the pinned commit may
differ in details.

- `NotificationClient::new(client, setup)` with
  `NotificationProcessSetup::MultipleProcesses` (own process, as iOS'
  notification extension; writes to the stores are coordinated by a
  cross-process lock) or `SingleProcess { sync_service }` (inside the
  app, as on Android; uses the app's running `SyncService`).
- `get_notification(room_id, event_id)` fetches the event with a short
  sliding sync (or `/context`), runs an encryption sync when the room
  key is missing, and returns it decrypted or, after the
  `decryption_deadline`, undecrypted.
- A second process needs the cross-process lock in **both** processes:
  `ClientBuilder::cross_process_store_config` with a different holder
  name per process (`matrix-sdk` `src/client/builder/mod.rs`).
  Fractal's clients (`src/utils/matrix/mod.rs`) do not set it.

## Design

Use the **single-process** setup with D-Bus activation of Nuntius
itself, not a second helper binary:

- Fractal already installs a D-Bus service file
  (`data/org.gnome.Fractal.service.in`: `Exec=fractal
  --gapplication-service`), so the session bus can start Nuntius
  without a window when the distributor delivers a message to its bus
  name.
- In that mode Nuntius restores the sessions and syncs; Fractal's own
  notification handler decrypts and shows the new messages, and the
  app quits after a timeout unless a window was opened.
  `NotificationClient` (single process) stays an option if the sync
  turns out too slow.
- No cross-process lock and no change to how Fractal opens its stores,
  which keeps the diff against upstream small.

## UnifiedPush D-Bus interface

KUnifiedPush 26.08.2 speaks versions 1 and 2 of the UnifiedPush D-Bus
interface; read from the introspection data embedded in
`kunifiedpush-distributor` and `libKUnifiedPush.so` of the Alpine
package (the specification site is blocked in the cloud environment).
Nuntius uses version 1:

| Side | Bus name, path | Interface | Methods |
|---|---|---|---|
| Distributor | `org.unifiedpush.Distributor.kde`, `/org/unifiedpush/Distributor` | `org.unifiedpush.Distributor1` | `Register(s serviceName, s token, s description) → (s registrationResult, s registrationResultReason)`, `Unregister(s token)` (no reply) |
| App | the app id, `/org/unifiedpush/Connector` | `org.unifiedpush.Connector1` | `Message(s token, ay message, s messageIdentifier)`, `NewEndpoint(s token, s endpoint)`, `Unregistered(s token)` (all without reply) |

`registrationResult` is `REGISTRATION_SUCCEEDED` or
`REGISTRATION_FAILED`.

## Implementation (`src/unifiedpush/`)

One file per job: `connector.rs` (the D-Bus interface the distributor
calls), `distributor.rs` (finding the distributor, `Register`,
`Unregister`), `pusher.rs` (Matrix gateway check, pusher on the
homeserver), `endpoints.rs` (stored endpoints), and `mod.rs`, which
keeps the registrations in sync with the sessions and their settings.

- The connector is exported in `Application::dbus_register`, before the
  bus name is owned, so it exists when the distributor starts Nuntius
  by D-Bus activation.
- Each ready session with notifications enabled (account and session
  switch of the existing notification settings) is registered with the
  first distributor on the bus (`ListNames`, then
  `ListActivatableNames`); the token is the session id. Turning
  notifications off, or logging out, unregisters it.
- The account switch is loaded from the homeserver only after the
  session is ready and reads as off until then. So a ready session is
  only ever registered; unregistering happens only when a setting
  changes. Registrations of a session run one after the other, the last
  request winning. (The first version also unregistered on "ready" with
  the not yet loaded setting; that `Unregister` reached KUnifiedPush
  37 ms after `Register`, so it held no client, opened no connection to
  the ntfy server and deleted the topic. Found on the phone with
  `dbus-monitor`, 2026-11-04.)
- Known limit: if notifications are turned off for the account on
  another device while Nuntius is closed, the old registration stays
  until notifications are turned off and on again here.
- On `NewEndpoint`, the Matrix gateway is looked up on the endpoint's
  server (`GET /_matrix/push/v1/notify` must answer
  `{"unifiedpush":{"gateway":"matrix"}}`; there is no fallback to a
  third-party gateway) and a pusher is set on the homeserver: pushkey =
  endpoint, app id = `de.nexumia.Nuntius`, `format: event_id_only`. The
  endpoint per session is kept in `unifiedpush-endpoints.json` in the
  data directory, to delete the pusher later.
- On `Message`, the application is held for 60 s: started by D-Bus
  activation it has no window, restores the sessions and syncs, and
  Fractal's notification handler shows the new messages; then it quits
  unless a window was opened. `NotificationClient` is not needed for
  this.
- Notifications of the catch-up sync (`src/session/mod.rs`,
  `src/session/notifications/`). matrix-sdk calls the notification
  handlers inside `sync_once`, before the response reaches Fractal's
  sync stream, and Fractal registered its handler only when it handled
  the first response; the new message is in exactly that first sync of
  a restored session, so no notification appeared (seen on the phone,
  2026-11-04: "Run notification handlers" ran 20 ms before "Received
  sync response"). Now `prepare()` awaits the registration before the
  first sync when the store has a sync token; without one (a new
  login) the first sync is a full initial sync with old messages, so
  the handler is still registered after it, as upstream does. The
  handler only spawns onto the GLib main context, so the SDK never
  waits for it. `show_push` also waits (at most 30 s) until the
  account switch was loaded from the homeserver
  (`NotificationsSettings:is-loaded`), which happens only after the
  first sync, instead of dropping the notification because the switch
  still reads as off. Both drop paths log at `debug`. Upstream had no
  fix (searched issues and merge requests on 2026-11-04); upstream
  moved the registration after the first sync in MR 1315 (2023).
- Flatpak: `--talk-name=org.unifiedpush.Distributor.*`; the app owns
  its id, and Flatpak exports its D-Bus service file for activation.

To verify on the phone: the distributor starts Nuntius, notifications
of the catch-up sync appear, 60 s is enough on mobile data.

Each step becomes an issue in this repository.
