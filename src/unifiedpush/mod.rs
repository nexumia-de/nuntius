//! Push notifications through UnifiedPush (Nuntius).
//!
//! Nuntius registers each session with a UnifiedPush distributor on the
//! session bus ([`distributor`]) and sets a pusher on the homeserver with the
//! endpoint the distributor returns ([`pusher`]). The distributor calls back
//! on the connector exported on the application's own bus name
//! ([`connector`]), which also starts Nuntius by D-Bus activation when it is
//! not running. The endpoint of each session is stored to delete its pusher
//! later ([`endpoints`]).
//!
//! This module keeps the registrations in sync with the sessions and their
//! notification settings.
//!
//! See `docs/push-notifications.md`.

mod connector;
mod distributor;
mod endpoints;
mod pusher;

use gtk::{glib, prelude::*};
use tracing::{debug, info, warn};

pub(crate) use self::connector::export_connector;
use crate::{
    Application,
    application::APP_NAME,
    prelude::*,
    session::{Session, SessionState},
    session_list::SessionList,
    spawn,
};

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
            distributor::unregister(&token).await;
            endpoints::remove(&token);
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
        let token = session.session_id().to_owned();

        if session.notifications().enabled() {
            // The distributor answers with `NewEndpoint` on the connector.
            let description = format!("{APP_NAME} ({})", session.user_id());
            distributor::register(&token, &description).await;
        } else {
            if let Some(endpoint) = endpoints::get(&token) {
                pusher::delete(&session, endpoint).await;
                endpoints::remove(&token);
            }
            distributor::unregister(&token).await;
        }
    });
}

/// The session with the given ID, if it is logged in.
fn session_for_token(token: &str) -> Option<Session> {
    Application::default()
        .session_list()
        .get(token)
        .and_downcast::<Session>()
}

/// Handle a new endpoint for the registration with the given token.
async fn on_new_endpoint(token: String, endpoint: String) {
    let Some(session) = session_for_token(&token) else {
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

    if let Some(old_endpoint) = endpoints::get(&token) {
        if old_endpoint == endpoint {
            debug!(session = token, "UnifiedPush endpoint unchanged");
        } else {
            pusher::delete(&session, old_endpoint).await;
        }
    }

    if pusher::set(&session, &endpoint).await {
        info!(session = token, "Push notifications are on");
        endpoints::store(&token, &endpoint);
    }
}

/// Handle the end of the registration with the given token.
async fn on_unregistered(token: String) {
    let Some(endpoint) = endpoints::get(&token) else {
        return;
    };

    if let Some(session) = session_for_token(&token) {
        pusher::delete(&session, endpoint).await;
    }

    endpoints::remove(&token);
    info!(session = token, "Push notifications are off");
}
