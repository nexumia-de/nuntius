//! The pusher on the homeserver and the Matrix push gateway it sends to.

use gtk::glib;
use ruma::{
    api::client::push::{PusherIds, PusherInit, PusherKind},
    push::{HttpPusherData, PushFormat},
};
use tracing::warn;

use crate::{APP_ID, application::APP_NAME, prelude::*, session::Session, spawn_tokio};

/// The path of the Matrix push gateway on a UnifiedPush server.
const MATRIX_GATEWAY_PATH: &str = "/_matrix/push/v1/notify";

/// Set a pusher for the given endpoint on the homeserver of the given session.
///
/// Returns whether the pusher was set.
pub(super) async fn set(session: &Session, endpoint: &str) -> bool {
    let client = session.client();
    let gateway_endpoint = endpoint.to_owned();
    let gateway =
        spawn_tokio!(async move { matrix_gateway(client.http_client(), &gateway_endpoint).await })
            .await
            .expect("task was not aborted");

    let Some(gateway) = gateway else {
        warn!(
            session = session.session_id(),
            "The UnifiedPush server has no Matrix push gateway, push notifications are off"
        );
        return false;
    };

    let mut data = HttpPusherData::new(gateway);
    data.format = Some(PushFormat::EventIdOnly);

    let pusher = PusherInit {
        ids: PusherIds::new(endpoint.to_owned(), APP_ID.to_owned()),
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

    if let Err(error) = result {
        warn!(
            session = session.session_id(),
            "Could not set pusher on the homeserver: {error}"
        );
        return false;
    }

    true
}

/// Delete the pusher with the given endpoint from the homeserver of the given
/// session.
pub(super) async fn delete(session: &Session, endpoint: String) {
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
