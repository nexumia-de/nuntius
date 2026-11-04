---
type: Concept
title: Push notifications
description: The fork's goal: Matrix notifications through UnifiedPush and ntfy while Nuntius is closed. Plan and open questions; nothing is implemented yet.
tags: [notifications, unifiedpush, ntfy, matrix]
generated: { by: claude-code/2.1.317, at: 2026-11-04T18:00:00Z }
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
  Phosh is documented with it[^honk]. Whether postmarketOS/Alpine
  packages it is not checked yet.

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

## Design (proposal)

Use the **single-process** setup with D-Bus activation of Nuntius
itself, not a second helper binary:

- Fractal already installs a D-Bus service file
  (`data/org.gnome.Fractal.service.in`: `Exec=fractal
  --gapplication-service`), so the session bus can start Nuntius
  without a window when the distributor delivers a message to its bus
  name.
- In that mode Nuntius restores the sessions, lets `NotificationClient`
  (single process) fetch and decrypt the event, shows the notification
  and quits after an idle timeout unless a window was opened.
- No cross-process lock and no change to how Fractal opens its stores,
  which keeps the diff against upstream small.

Steps:

1. Register with the distributor (UnifiedPush D-Bus interface) and set
   a pusher on the homeserver with the endpoint, `format:
   event_id_only`.
2. Implement the connector interface the distributor calls; on a
   message, resolve and show the notification.
3. Background mode: no window on D-Bus activation, quit when idle.
4. Flatpak: `--own-name` is implied for the app id; talk access to the
   distributor's bus name.

Each step becomes an issue in this repository.
