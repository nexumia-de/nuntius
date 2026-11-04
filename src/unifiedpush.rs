//! Push notifications through UnifiedPush (Nuntius).
//!
//! Nuntius registers each session with a UnifiedPush distributor on the
//! session bus (D-Bus interface `org.unifiedpush.Distributor1`) and sets a
//! pusher on the homeserver with the endpoint the distributor returns. The
//! distributor calls back on `org.unifiedpush.Connector1`, exported on the
//! application's own bus name, which also starts Nuntius by D-Bus activation
//! when it is not running.
//!
//! When a push message arrives, the application is kept alive for a while so
//! that the restored sessions sync and the usual notification handler shows
//! the new messages.
//!
//! See `docs/push-notifications.md`.

use std::{cell::RefCell, collections::HashMap, time::Duration};

use gtk::{gio, glib, prelude::*};
use ruma::{
    api::client::push::{PusherIds, PusherInit, PusherKind},
    push::{HttpPusherData, PushFormat},
};
use tracing::{debug, info, warn};

use crate::{
    APP_ID, Application,
    application::APP_NAME,
    prelude::*,
    session::{Session, SessionState},
    session_list::SessionList,
    spawn, spawn_tokio,
    utils::DataType,
};

/// The prefix of the bus names of UnifiedPush distributors.
const DISTRIBUTOR_NAME_PREFIX: &str = "org.unifiedpush.Distributor.";
/// The object path of a UnifiedPush distributor.
const DISTRIBUTOR_PATH: &str = "/org/unifiedpush/Distributor";
/// The D-Bus interface of a UnifiedPush distributor.
const DISTRIBUTOR_INTERFACE: &str = "org.unifiedpush.Distributor1";
/// The object path of the UnifiedPush connector of an application.
const CONNECTOR_PATH: &str = "/org/unifiedpush/Connector";
/// The D-Bus interface of the UnifiedPush connector.
const CONNECTOR_INTERFACE: &str = "org.unifiedpush.Connector1";
/// The introspection data of the UnifiedPush connector.
const CONNECTOR_XML: &str = r#"<node>
  <interface name="org.unifiedpush.Connector1">
    <method name="Message">
      <arg direction="in" type="s" name="token"/>
      <arg direction="in" type="ay" name="message"/>
      <arg direction="in" type="s" name="messageIdentifier"/>
      <annotation name="org.freedesktop.DBus.Method.NoReply" value="true"/>
    </method>
    <method name="NewEndpoint">
      <arg direction="in" type="s" name="token"/>
      <arg direction="in" type="s" name="endpoint"/>
      <annotation name="org.freedesktop.DBus.Method.NoReply" value="true"/>
    </method>
    <method name="Unregistered">
      <arg direction="in" type="s" name="token"/>
      <annotation name="org.freedesktop.DBus.Method.NoReply" value="true"/>
    </method>
  </interface>
</node>"#;
/// The path of the Matrix push gateway on a UnifiedPush server.
const MATRIX_GATEWAY_PATH: &str = "/_matrix/push/v1/notify";
/// How long the application is kept running after a push message, so that
/// the sessions can sync and show the notifications.
const MESSAGE_HOLD: Duration = Duration::from_secs(60);
/// The name of the file that stores the endpoint of each session.
const ENDPOINTS_FILE: &str = "unifiedpush-endpoints.json";

thread_local! {
    /// The endpoint of each session, by session ID.
    static ENDPOINTS: RefCell<Option<HashMap<String, String>>> = const { RefCell::new(None) };
}

/// Export the UnifiedPush connector on the given connection.
///
/// Must be called while the application registers on the bus, so the
/// connector exists before the distributor calls it after a D-Bus activation.
pub(crate) fn export_connector(connection: &gio::DBusConnection) -> Result<(), glib::Error> {
    let node = gio::DBusNodeInfo::for_xml(CONNECTOR_XML)?;
    let interface = node
        .lookup_interface(CONNECTOR_INTERFACE)
        .expect("connector XML should contain the connector interface");

    connection
        .register_object(CONNECTOR_PATH, &interface)
        .method_call(
            |_connection, _sender, _path, _interface, method, parameters, invocation| {
                handle_connector_call(method, &parameters);
                invocation.return_value(None);
            },
        )
        .build()?;

    Ok(())
}

/// Handle a call to the UnifiedPush connector.
fn handle_connector_call(method: &str, parameters: &glib::Variant) {
    match method {
        "NewEndpoint" => {
            let Some((token, endpoint)) = parameters.get::<(String, String)>() else {
                warn!("Invalid parameters for UnifiedPush NewEndpoint");
                return;
            };
            spawn!(async move {
                on_new_endpoint(token, endpoint).await;
            });
        }
        "Unregistered" => {
            let Some((token,)) = parameters.get::<(String,)>() else {
                warn!("Invalid parameters for UnifiedPush Unregistered");
                return;
            };
            spawn!(async move {
                on_unregistered(token).await;
            });
        }
        "Message" => {
            debug!("Received UnifiedPush message");
            // Keep the application running while the sessions sync, then let
            // it quit again if no window was opened.
            let guard = Application::default().hold();
            glib::timeout_add_local_once(MESSAGE_HOLD, move || drop(guard));
        }
        _ => {
            warn!("Unknown UnifiedPush connector method {method}");
        }
    }
}

/// Watch the sessions of the given list and keep their UnifiedPush
/// registration in sync with their notification settings.
pub(crate) fn watch_sessions(session_list: &SessionList) {
    session_list.connect_items_changed(|list, position, _removed, added| {
        for index in position..position + added {
            if let Some(session) = list.item(index).and_downcast::<Session>() {
                watch_session(&session);
            }
        }
    });

    for index in 0..session_list.n_items() {
        if let Some(session) = session_list.item(index).and_downcast::<Session>() {
            watch_session(&session);
        }
    }
}

/// Keep the UnifiedPush registration of the given session in sync with its
/// state and notification settings.
fn watch_session(session: &Session) {
    session.connect_ready(update_registration);

    let settings = session.notifications().settings();
    settings.connect_account_enabled_notify(glib::clone!(
        #[weak]
        session,
        move |_| update_registration(&session)
    ));
    settings.connect_session_enabled_notify(glib::clone!(
        #[weak]
        session,
        move |_| update_registration(&session)
    ));

    session.connect_logged_out(|session| {
        let token = session.session_id().to_owned();
        spawn!(async move {
            unregister_from_distributor(&token).await;
            remove_endpoint(&token);
        });
    });

    if session.state() == SessionState::Ready {
        update_registration(session);
    }
}

/// Register or unregister the given session, depending on its state and
/// notification settings.
fn update_registration(session: &Session) {
    if session.state() != SessionState::Ready {
        return;
    }

    let session = session.clone();
    spawn!(async move {
        if session.notifications().enabled() {
            register(&session).await;
        } else {
            unregister(&session).await;
        }
    });
}

/// Register the given session with the UnifiedPush distributor.
///
/// The distributor answers with `NewEndpoint` on the connector.
async fn register(session: &Session) {
    let Some(connection) = Application::default().dbus_connection() else {
        warn!("Cannot register for push notifications without a D-Bus connection");
        return;
    };
    let Some(distributor) = find_distributor(&connection).await else {
        info!("No UnifiedPush distributor found, push notifications are off");
        return;
    };

    let token = session.session_id().to_owned();
    let description = format!("{APP_NAME} ({})", session.user_id());
    let parameters = (APP_ID, token.as_str(), description.as_str()).to_variant();

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

/// Unregister the given session from the distributor and delete its pusher.
async fn unregister(session: &Session) {
    let token = session.session_id().to_owned();

    if let Some(endpoint) = stored_endpoint(&token) {
        delete_pusher(session, endpoint).await;
        remove_endpoint(&token);
    }

    unregister_from_distributor(&token).await;
}

/// Tell the distributor to forget the registration with the given token.
async fn unregister_from_distributor(token: &str) {
    let Some(connection) = Application::default().dbus_connection() else {
        return;
    };
    let Some(distributor) = find_distributor(&connection).await else {
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
async fn find_distributor(connection: &gio::DBusConnection) -> Option<String> {
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

/// Handle a new endpoint for the registration with the given token.
async fn on_new_endpoint(token: String, endpoint: String) {
    let Some(session) = Application::default()
        .session_list()
        .get(&token)
        .and_downcast::<Session>()
    else {
        warn!("Received UnifiedPush endpoint for unknown session");
        return;
    };

    if !session.notifications().enabled() {
        debug!(
            session = token,
            "Ignoring UnifiedPush endpoint, notifications are off"
        );
        return;
    }

    if let Some(old_endpoint) = stored_endpoint(&token) {
        if old_endpoint == endpoint {
            debug!(session = token, "UnifiedPush endpoint unchanged");
        } else {
            delete_pusher(&session, old_endpoint).await;
        }
    }

    let client = session.client();
    let gateway_endpoint = endpoint.clone();
    let gateway =
        spawn_tokio!(async move { matrix_gateway(client.http_client(), &gateway_endpoint).await })
            .await
            .expect("task was not aborted");

    let Some(gateway) = gateway else {
        warn!(
            session = token,
            "The UnifiedPush server has no Matrix push gateway, push notifications are off"
        );
        return;
    };

    let mut data = HttpPusherData::new(gateway);
    data.format = Some(PushFormat::EventIdOnly);

    let pusher = PusherInit {
        ids: PusherIds::new(endpoint.clone(), APP_ID.to_owned()),
        kind: PusherKind::Http(data),
        app_display_name: APP_NAME.to_owned(),
        device_display_name: session.device_id().to_string(),
        profile_tag: None,
        lang: language_tag(),
    };

    let client = session.client();
    let result = spawn_tokio!(async move { client.pusher().set(pusher.into(), false).await })
        .await
        .expect("task was not aborted");

    match result {
        Ok(()) => {
            info!(session = token, "Push notifications are on");
            store_endpoint(&token, &endpoint);
        }
        Err(error) => {
            warn!(
                session = token,
                "Could not set pusher on the homeserver: {error}"
            );
        }
    }
}

/// Handle the end of the registration with the given token.
async fn on_unregistered(token: String) {
    let Some(endpoint) = stored_endpoint(&token) else {
        return;
    };

    if let Some(session) = Application::default()
        .session_list()
        .get(&token)
        .and_downcast::<Session>()
    {
        delete_pusher(&session, endpoint).await;
    }

    remove_endpoint(&token);
    info!(session = token, "Push notifications are off");
}

/// Delete the pusher with the given endpoint from the homeserver.
async fn delete_pusher(session: &Session, endpoint: String) {
    let client = session.client();
    let ids = PusherIds::new(endpoint, APP_ID.to_owned());
    let result = spawn_tokio!(async move { client.pusher().delete(ids).await })
        .await
        .expect("task was not aborted");

    if let Err(error) = result {
        warn!(
            session = session.session_id(),
            "Could not delete pusher on the homeserver: {error}"
        );
    }
}

/// The Matrix push gateway of the UnifiedPush server of the given endpoint, if
/// it has one.
///
/// UnifiedPush servers that embed a Matrix gateway answer a `GET` on
/// `/_matrix/push/v1/notify` with `{"unifiedpush":{"gateway":"matrix"}}`.
async fn matrix_gateway(
    http_client: &matrix_sdk::reqwest::Client,
    endpoint: &str,
) -> Option<String> {
    let mut url = url::Url::parse(endpoint).ok()?;
    url.set_path(MATRIX_GATEWAY_PATH);
    url.set_query(None);
    url.set_fragment(None);

    let response = http_client.get(url.as_str()).send().await.ok()?;
    let bytes = response.bytes().await.ok()?;
    let body = serde_json::from_slice::<serde_json::Value>(&bytes).ok()?;

    (body["unifiedpush"]["gateway"] == "matrix").then(|| url.to_string())
}

/// The preferred language as a language tag, like `en` or `de-DE`.
fn language_tag() -> String {
    glib::language_names()
        .iter()
        .map(|name| {
            name.split(['.', '@'])
                .next()
                .unwrap_or_default()
                .replace('_', "-")
        })
        .find(|tag| !tag.is_empty() && tag != "C" && tag != "POSIX")
        .unwrap_or_else(|| "en".to_owned())
}

/// Run the given function with the stored endpoints, loading them first if
/// needed.
fn with_endpoints<T>(f: impl FnOnce(&mut HashMap<String, String>) -> T) -> T {
    ENDPOINTS.with_borrow_mut(|endpoints| {
        let endpoints = endpoints.get_or_insert_with(|| {
            std::fs::read(endpoints_path())
                .ok()
                .and_then(|data| serde_json::from_slice(&data).ok())
                .unwrap_or_default()
        });
        f(endpoints)
    })
}

/// The stored endpoint of the session with the given ID.
fn stored_endpoint(session_id: &str) -> Option<String> {
    with_endpoints(|endpoints| endpoints.get(session_id).cloned())
}

/// Store the endpoint of the session with the given ID.
fn store_endpoint(session_id: &str, endpoint: &str) {
    with_endpoints(|endpoints| {
        endpoints.insert(session_id.to_owned(), endpoint.to_owned());
        save_endpoints(endpoints);
    });
}

/// Remove the endpoint of the session with the given ID.
fn remove_endpoint(session_id: &str) {
    with_endpoints(|endpoints| {
        if endpoints.remove(session_id).is_some() {
            save_endpoints(endpoints);
        }
    });
}

/// Write the given endpoints to disk.
fn save_endpoints(endpoints: &HashMap<String, String>) {
    let path = endpoints_path();
    let result = serde_json::to_vec(endpoints)
        .map_err(std::io::Error::other)
        .and_then(|data| {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, data)
        });

    if let Err(error) = result {
        warn!("Could not store UnifiedPush endpoints: {error}");
    }
}

/// The path of the file that stores the endpoints.
fn endpoints_path() -> std::path::PathBuf {
    DataType::Persistent.dir_path().join(ENDPOINTS_FILE)
}
