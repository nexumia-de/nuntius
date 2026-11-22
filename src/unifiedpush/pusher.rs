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
async fn matrix_gateway(
    http_client: &matrix_sdk::reqwest::Client,
    endpoint: &str,
) -> Option<String> {
    let url = gateway_url(endpoint)?;

    let response = http_client.get(url.as_str()).send().await.ok()?;
    let bytes = response.bytes().await.ok()?;

    is_matrix_gateway(&bytes).then(|| url.to_string())
}

/// The URL of the Matrix push gateway on the server of the given endpoint.
fn gateway_url(endpoint: &str) -> Option<url::Url> {
    let mut url = url::Url::parse(endpoint).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }

    url.set_path(MATRIX_GATEWAY_PATH);
    url.set_query(None);
    url.set_fragment(None);

    Some(url)
}

/// Whether the given answer to a `GET` on the gateway URL is the one of a
/// Matrix push gateway.
///
/// UnifiedPush servers that embed a Matrix gateway answer with
/// `{"unifiedpush":{"gateway":"matrix"}}`.
fn is_matrix_gateway(body: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(body)
        .is_ok_and(|body| body["unifiedpush"]["gateway"] == "matrix")
}

/// The preferred language as a language tag, like `en` or `de-DE`.
fn language_tag() -> String {
    language_tag_from(glib::language_names().iter().map(glib::GString::as_str))
}

/// The first language tag in the given locale names, like `de_DE.UTF-8`, or
/// `en`.
fn language_tag_from<'a>(names: impl IntoIterator<Item = &'a str>) -> String {
    names
        .into_iter()
        .map(|name| {
            name.split(['.', '@'])
                .next()
                .unwrap_or_default()
                .replace('_', "-")
        })
        .find(|tag| !tag.is_empty() && tag != "C" && tag != "POSIX")
        .unwrap_or_else(|| "en".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The answer of `https://unifiedpush.kde.org/_matrix/push/v1/notify`,
    /// recorded on 2026-11-04 (`docs/push-notifications.md`).
    const KDE_GATEWAY_ANSWER: &[u8] = br#"{"unifiedpush":{"gateway":"matrix"}}"#;

    #[test]
    fn gateway_url_of_endpoint() {
        // The shape of an ntfy endpoint: the topic as path, `up=1` as query.
        let url = gateway_url("https://unifiedpush.kde.org/upTopic123?up=1").unwrap();
        assert_eq!(
            url.as_str(),
            "https://unifiedpush.kde.org/_matrix/push/v1/notify"
        );

        let url = gateway_url("http://localhost:8080/some/path#fragment").unwrap();
        assert_eq!(url.as_str(), "http://localhost:8080/_matrix/push/v1/notify");

        assert_eq!(gateway_url("not a url"), None);
        assert_eq!(gateway_url("file:///etc/passwd"), None);
    }

    #[test]
    fn matrix_gateway_answer() {
        assert!(is_matrix_gateway(KDE_GATEWAY_ANSWER));

        // Answers of servers without a Matrix gateway, made up for this test.
        assert!(!is_matrix_gateway(br#"{"unifiedpush":{"version":1}}"#));
        assert!(!is_matrix_gateway(br#"{"unifiedpush":{"gateway":"other"}}"#));
        assert!(!is_matrix_gateway(b"{}"));
        assert!(!is_matrix_gateway(b"<!DOCTYPE html><html></html>"));
        assert!(!is_matrix_gateway(b""));
    }

    #[test]
    fn language_tag_of_locale_names() {
        assert_eq!(
            language_tag_from(["de_DE.UTF-8", "de_DE", "de", "C"]),
            "de-DE"
        );
        assert_eq!(language_tag_from(["sr_RS@latin", "sr"]), "sr-RS");
        assert_eq!(language_tag_from(["C.UTF-8", "C", "POSIX"]), "en");
        assert_eq!(language_tag_from(["", "fr"]), "fr");
        assert_eq!(language_tag_from([]), "en");
    }
}
