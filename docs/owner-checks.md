---
type: Playbook
title: Owner checks
description: What only the owner can do or confirm for Nuntius - checks on the phone, secrets, and the first stable release; nothing here was run by an agent.
tags: [checklist, owner, device, release]
generated: { by: claude-code/2.1.330, at: 2026-11-22T00:00:00Z }
---

# Owner checks

Open points that need the phone, the owner's accounts or repository
settings. Agents add points here instead of opening issues that wait
for the owner, and remove a point only when the owner reported its
result (write the result into the concept it belongs to).

## Push notifications on the phone

From #2; background in [push notifications](push-notifications.md).
The code is on `phosh` and its parts that need no device have unit
tests; none of the following ran after the last fix (pull request #12).

- [ ] With Nuntius closed and KUnifiedPush running, a message from
      another account shows a notification within about a minute.
- [ ] Tapping the notification opens Nuntius in that room.
- [ ] The same on mobile data: the 60 s that Nuntius stays alive after a
      push message are enough for the catch-up sync.
- [ ] After a reboot of the phone, without opening Nuntius, a message
      still shows a notification (autostart of the distributor, D-Bus
      activation of Nuntius).
- [ ] Turning notifications off in Nuntius' settings stops them, turning
      them on again brings them back.
- [ ] `unifiedpush.kde.org` may be used by a non-KDE app: KDE publishes
      no terms on the server itself (`config.js` shows anonymous use
      only, 2026-11-04). If KDE says no, set another ntfy server in
      `~/.config/KDE/kunifiedpush-distributor.conf`; Nuntius needs no
      change, it uses whatever endpoint the distributor hands out.

If a check fails, the log helps:

```sh
flatpak run --env=RUST_LOG=fractal=debug,warn de.nexumia.Nuntius
```

## Releases

From #16; how releases work: [Flatpak build and releases](flatpak.md#releases).

- [ ] Add the repository secret `RELEASE_TOKEN`: a fine-grained token
      with **Contents** and **Pull requests** read/write on
      `nexumia-de/nuntius`. Without it the release pull request and the
      tag start no builds.
- [ ] After merging the pull request that added this file: *Actions →
      Release → Run workflow*, review and merge "Release v0.2.0", then
      follow steps 3 and 4 of
      [the first stable release](flatpak.md#first-stable-release).
- [ ] If `phosh` has a ruleset with required checks, add *Changelog*
      and *Build scripts*.

## App icon

From #5; [decision](decisions/app-icon.md).

- [ ] The icon looks right on the phone's app grid, in the app switcher
      and in a notification. To change it, replace the three files
      `data/icons/de.nexumia.Nuntius*.svg`.
