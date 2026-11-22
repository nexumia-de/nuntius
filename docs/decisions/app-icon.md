---
type: Decision Record
title: Own app icon, a speech bubble with a wing
description: Nuntius installs its own icon instead of Fractal's - a purple speech bubble with a white wing, drawn in the GNOME app icon style, with a development and a symbolic variant, under CC0-1.0.
tags: [decision, design, icon]
generated: { by: claude-code/2.1.330, at: 2026-11-22T00:00:00Z }
sources:
  - id: hig-app-icons
    resource: https://developer.gnome.org/hig/guidelines/app-icons.html
    title: GNOME HIG, app icons
---

# Status

Accepted (2026-11-22). Issue #5 asked for a few designs for the owner
to choose from; the owner then asked the agent to decide by itself and
record why. The owner can still replace the files; nothing else depends
on the drawing.

# Context

Nuntius installed Fractal's icon (Tobias Bernard, CC-BY-SA-4.0) under
its own app id. On the phone's app grid the two apps could not be told
apart, and the icon said "Fractal" for an app that must not present
itself as Fractal ([fork and name](fork-and-name.md)).

# Decision

- **Motif**: a speech bubble (a chat app) carrying a wing. Nuntius is
  Latin for messenger; the wing is the attribute of Mercury, the
  messenger of the gods. One simple shape inside one simple shape stays
  readable at 16 px.
- **Colour**: GNOME palette purples (`#dc8add` to `#9141ac`, edge
  `#613583`), a white wing. Fractal's icon is cyan and blue, so the two
  apps are told apart on the app grid when both are installed.
- **Style**: GNOME app icon style[^hig-app-icons]: 128 px canvas, simple
  geometry, a lower edge for depth, no outline, no text.
- **Files** (`data/icons/`, CC0-1.0, drawn for Nuntius):
  `de.nexumia.Nuntius.svg`, `de.nexumia.Nuntius.Devel.svg` (the same
  with hazard stripes on the edge, for the nightly app) and
  `de.nexumia.Nuntius-symbolic.svg` (the bubble with the wing cut out).
  `data/icons/meson.build` installs them through `icon_base_id`;
  upstream's icon files stay in the repository unused, so upstream
  changes to them never conflict.

# Considered

- A paper plane: the usual "send" sign, but it is the logo of Telegram.
- An envelope with wings: reads as e-mail.
- A winged helmet or the caduceus: too detailed for 16 px; the caduceus
  is read as a medical sign.
- Recolouring Fractal's bubble and spiral: still Fractal's logo, and it
  would stay CC-BY-SA-4.0 with attribution.

# Consequences

- The app grid, the About dialog, notifications and GNOME Software show
  the new icon; no code changed.
- Fractal's icon is still compiled into the resources
  (`data/resources/icons/scalable/apps/org.gnome.Fractal.svg`, upstream's
  file list) but no code looks it up under that name.
- The accent colour inside the app is unchanged (upstream's UI follows
  the system accent).
