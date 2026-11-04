# Directory Update Log

## 2026-11-04
* **Update**: [Push notifications](push-notifications.md): KUnifiedPush is in Alpine edge `community` (26.08.2); its autostart file and default configuration (KDE's ntfy server) read from the package, dependency size estimated from the APKINDEX.
* **Update**: [Push notifications](push-notifications.md): `matrix-sdk-ui` 0.19.1's `NotificationClient` read (process setups, `get_notification`, cross-process lock needed in both processes); design proposal: single process, Nuntius started by D-Bus activation through upstream's existing service file.
* **Update**: [Push notifications](push-notifications.md): the owner chose KDE's ntfy server `unifiedpush.kde.org`; its health endpoint, Matrix gateway and anonymous-only configuration checked. [Development environment](dev-environment.md): the host is reachable.
* **Creation**: Fork of Fractal at upstream `9fbc7283` as Nuntius ([fork and name](decisions/fork-and-name.md)): `main` mirrors GNOME's GitLab, `phosh` carries the fork ([fork workflow](fork-workflow.md)); app id `de.nexumia.Nuntius` with upstream's file names kept; own Flatpak manifest on GNOME 51 and a build workflow for x86_64 and aarch64 ([development environment](dev-environment.md)); the push notification goal ([push notifications](push-notifications.md)).
