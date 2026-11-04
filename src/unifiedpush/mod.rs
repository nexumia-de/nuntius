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

use std::{cell::RefCell, collections::HashMap};

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

thread_local! {
    /// The registration state of each session, by session ID.
    static STATES: RefCell<HashMap<String, RegistrationState>> = RefCell::default();
}

/// The UnifiedPush registration state of a session.
#[derive(Debug, Default)]
struct RegistrationState {
    /// Whether a task is registering or unregistering the session.
    busy: bool,
    /// Whether the session should be registered, if this was requested while
    /// the task was busy.
    wanted: Option<bool>,
    /// Whether the session was last registered or unregistered by this
    /// process.
    applied: Option<bool>,
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
    // Whether notifications are enabled for the account is only known once
    // the notification settings are loaded from the homeserver, which happens
    // after the session is ready. Until then it reads as disabled, so a ready
    // session is only ever registered here, never unregistered; the settings
    // notifications below handle the rest.
    session.connect_ready(register_if_enabled);

    let settings = session.notifications().settings();
    settings.connect_account_enabled_notify(glib::clone!(
        #[weak]
        session,
        move |_| on_settings_changed(&session)
    ));
    settings.connect_session_enabled_notify(glib::clone!(
        #[weak]
        session,
        move |_| on_settings_changed(&session)
    ));

    session.connect_logged_out(|session| {
        let token = session.session_id().to_owned();
        STATES.with_borrow_mut(|states| states.remove(&token));
        spawn!(async move {
            distributor::unregister(&token).await;
            endpoints::remove(&token);
        });
    });

    if session.state() == SessionState::Ready {
        register_if_enabled(session);
    }
}

/// Register the given session if notifications are enabled for it.
fn register_if_enabled(session: &Session) {
    if session.notifications().enabled() {
        request_registration(session, true);
    }
}

/// Handle a change of the notification settings of the given session.
fn on_settings_changed(session: &Session) {
    if session.state() == SessionState::Ready {
        request_registration(session, session.notifications().enabled());
    }
}

/// Request that the given session is registered or unregistered.
///
/// The requests of a session are applied one after the other, and only the
/// last one requested while a previous one is running is applied, so a
/// registration and an unregistration never overlap.
fn request_registration(session: &Session, registered: bool) {
    let token = session.session_id().to_owned();
    let start = STATES.with_borrow_mut(|states| {
        let state = states.entry(token).or_default();
        state.wanted = Some(registered);
        !std::mem::replace(&mut state.busy, true)
    });

    if start {
        let session = session.clone();
        spawn!(async move {
            apply_requests(&session).await;
        });
    }
}

/// Apply the requested registration state of the given session until no
/// request is left.
async fn apply_requests(session: &Session) {
    let token = session.session_id().to_owned();

    loop {
        let next = STATES.with_borrow_mut(|states| {
            let state = states.entry(token.clone()).or_default();
            match state.wanted.take() {
                Some(wanted) if state.applied != Some(wanted) => {
                    state.applied = Some(wanted);
                    Some(wanted)
                }
                _ => {
                    state.busy = false;
                    None
                }
            }
        });

        match next {
            Some(true) => {
                // The distributor answers with `NewEndpoint` on the connector.
                let description = format!("{APP_NAME} ({})", session.user_id());
                distributor::register(&token, &description).await;
            }
            Some(false) => {
                if let Some(endpoint) = endpoints::get(&token) {
                    pusher::delete(session, endpoint).await;
                    endpoints::remove(&token);
                }
                distributor::unregister(&token).await;
            }
            None => break,
        }
    }
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
