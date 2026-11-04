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

## Plan for Nuntius (not started)

1. A setting that registers Nuntius with the distributor over D-Bus
   (UnifiedPush D-Bus spec) and sets a pusher on the homeserver.
2. A small process started by D-Bus activation on a push message. It
   opens the session store, fetches and decrypts the one event and sends
   a desktop notification; activating it opens the room in Nuntius.
   `matrix-rust-sdk` has a notification client meant for this
   (separate process next to the app, as on iOS); its API and the
   cross-process store lock are to be read in the SDK version Fractal
   pins before the design is fixed.
3. Flatpak: D-Bus activation of the helper and access to the
   distributor's bus name.

Each step becomes an issue in this repository.
