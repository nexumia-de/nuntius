//! Calls to the UnifiedPush distributor on the session bus.

use gtk::{gio, glib, prelude::*};
use tracing::{debug, info, warn};

use crate::{APP_ID, Application};

/// The prefix of the bus names of UnifiedPush distributors.
const DISTRIBUTOR_NAME_PREFIX: &str = "org.unifiedpush.Distributor.";
/// The object path of a UnifiedPush distributor.
const DISTRIBUTOR_PATH: &str = "/org/unifiedpush/Distributor";
/// The D-Bus interface of a UnifiedPush distributor.
const DISTRIBUTOR_INTERFACE: &str = "org.unifiedpush.Distributor1";

/// Register with the distributor under the given token.
///
/// The distributor answers with `NewEndpoint` on the connector.
pub(super) async fn register(token: &str, description: &str) {
    let Some(connection) = Application::default().dbus_connection() else {
        warn!("Cannot register for push notifications without a D-Bus connection");
        return;
    };
    let Some(distributor) = find(&connection).await else {
        info!("No UnifiedPush distributor found, push notifications are off");
        return;
    };

    let parameters = (APP_ID, token, description).to_variant();
    let result = connection
        .call_future(
            Some(distributor.as_str()),
            DISTRIBUTOR_PATH,
            DISTRIBUTOR_INTERFACE,
            "Register",
            Some(&parameters),
            Some(glib::VariantTy::new("(ss)").expect("type string should be valid")),
            gio::DBusCallFlags::NONE,
            -1,
        )
        .await;

    match result.map(|reply| reply.get::<(String, String)>()) {
        Ok(Some((status, _))) if status == "REGISTRATION_SUCCEEDED" => {
            info!(
                session = token,
                "Registered with UnifiedPush distributor {distributor}"
            );
        }
        Ok(Some((status, reason))) => {
            warn!(
                session = token,
                "UnifiedPush registration with {distributor} failed: {status} {reason}"
            );
        }
        Ok(None) => {
            warn!(session = token, "Invalid UnifiedPush registration reply");
        }
        Err(error) => {
            warn!(
                session = token,
                "Could not register with UnifiedPush distributor: {error}"
            );
        }
    }
}

/// Tell the distributor to forget the registration with the given token.
pub(super) async fn unregister(token: &str) {
    let Some(connection) = Application::default().dbus_connection() else {
        return;
    };
    let Some(distributor) = find(&connection).await else {
        return;
    };

    let result = connection
        .call_future(
            Some(distributor.as_str()),
            DISTRIBUTOR_PATH,
            DISTRIBUTOR_INTERFACE,
            "Unregister",
            Some(&(token,).to_variant()),
            None,
            gio::DBusCallFlags::NONE,
            -1,
        )
        .await;

    if let Err(error) = result {
        debug!(
            session = token,
            "Could not unregister from UnifiedPush distributor: {error}"
        );
    }
}

/// Find the bus name of a UnifiedPush distributor, running or activatable.
async fn find(connection: &gio::DBusConnection) -> Option<String> {
    for method in ["ListNames", "ListActivatableNames"] {
        let result = connection
            .call_future(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                method,
                None,
                Some(glib::VariantTy::new("(as)").expect("type string should be valid")),
                gio::DBusCallFlags::NONE,
                -1,
            )
            .await;

        let names = match result.map(|reply| reply.get::<(Vec<String>,)>()) {
            Ok(Some((names,))) => names,
            Ok(None) => continue,
            Err(error) => {
                debug!("Could not list D-Bus names: {error}");
                continue;
            }
        };

        if let Some(distributor) = first_distributor(names) {
            return Some(distributor);
        }
    }

    None
}

/// The first UnifiedPush distributor, in alphabetical order, among the given
/// bus names.
fn first_distributor(names: impl IntoIterator<Item = String>) -> Option<String> {
    names
        .into_iter()
        .filter(|name| name.starts_with(DISTRIBUTOR_NAME_PREFIX))
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn first_distributor_among_bus_names() {
        // KUnifiedPush's bus name is from its D-Bus introspection data
        // (`docs/push-notifications.md`); the other names are examples.
        assert_eq!(
            first_distributor(names(&[
                ":1.42",
                "org.freedesktop.Notifications",
                "org.unifiedpush.Distributor.kde",
                "de.nexumia.Nuntius",
            ]))
            .as_deref(),
            Some("org.unifiedpush.Distributor.kde")
        );
        assert_eq!(
            first_distributor(names(&[
                "org.unifiedpush.Distributor.kde",
                "org.unifiedpush.Distributor.gotify",
            ]))
            .as_deref(),
            Some("org.unifiedpush.Distributor.gotify")
        );
        assert_eq!(
            first_distributor(names(&[
                "org.freedesktop.DBus",
                "org.unifiedpush.Connector"
            ])),
            None
        );
        assert_eq!(first_distributor(names(&[])), None);
    }
}
