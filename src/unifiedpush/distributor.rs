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

        let mut distributors = names
            .into_iter()
            .filter(|name| name.starts_with(DISTRIBUTOR_NAME_PREFIX))
            .collect::<Vec<_>>();
        distributors.sort();

        if let Some(distributor) = distributors.into_iter().next() {
            return Some(distributor);
        }
    }

    None
}
