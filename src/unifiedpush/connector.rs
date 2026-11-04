//! The UnifiedPush connector: the D-Bus interface the distributor calls.

use std::time::Duration;

use gtk::{gio, glib, prelude::*};
use tracing::{debug, warn};

use crate::{Application, spawn};

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
/// How long the application is kept running after a push message, so that
/// the sessions can sync and show the notifications.
const MESSAGE_HOLD: Duration = Duration::from_secs(60);

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
                handle_call(method, &parameters);
                invocation.return_value(None);
            },
        )
        .build()?;

    Ok(())
}

/// Handle a call to the UnifiedPush connector.
fn handle_call(method: &str, parameters: &glib::Variant) {
    match method {
        "NewEndpoint" => {
            let Some((token, endpoint)) = parameters.get::<(String, String)>() else {
                warn!("Invalid parameters for UnifiedPush NewEndpoint");
                return;
            };
            spawn!(async move {
                super::on_new_endpoint(token, endpoint).await;
            });
        }
        "Unregistered" => {
            let Some((token,)) = parameters.get::<(String,)>() else {
                warn!("Invalid parameters for UnifiedPush Unregistered");
                return;
            };
            spawn!(async move {
                super::on_unregistered(token).await;
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
