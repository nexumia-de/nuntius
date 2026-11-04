//! The stored UnifiedPush endpoint of each session.
//!
//! The endpoint is the pushkey of the session's pusher, so it is needed to
//! delete the pusher after a restart.

use std::{cell::RefCell, collections::HashMap, path::PathBuf};

use tracing::warn;

use crate::utils::DataType;

/// The name of the file that stores the endpoint of each session.
const ENDPOINTS_FILE: &str = "unifiedpush-endpoints.json";

thread_local! {
    /// The endpoint of each session, by session ID, loaded on first use.
    static ENDPOINTS: RefCell<Option<HashMap<String, String>>> = const { RefCell::new(None) };
}

/// The stored endpoint of the session with the given ID.
pub(super) fn get(session_id: &str) -> Option<String> {
    with_endpoints(|endpoints| endpoints.get(session_id).cloned())
}

/// Store the endpoint of the session with the given ID.
pub(super) fn store(session_id: &str, endpoint: &str) {
    with_endpoints(|endpoints| {
        endpoints.insert(session_id.to_owned(), endpoint.to_owned());
        save(endpoints);
    });
}

/// Remove the endpoint of the session with the given ID.
pub(super) fn remove(session_id: &str) {
    with_endpoints(|endpoints| {
        if endpoints.remove(session_id).is_some() {
            save(endpoints);
        }
    });
}

/// Run the given function with the stored endpoints, loading them first if
/// needed.
fn with_endpoints<T>(f: impl FnOnce(&mut HashMap<String, String>) -> T) -> T {
    ENDPOINTS.with_borrow_mut(|endpoints| {
        let endpoints = endpoints.get_or_insert_with(|| {
            std::fs::read(path())
                .ok()
                .and_then(|data| serde_json::from_slice(&data).ok())
                .unwrap_or_default()
        });
        f(endpoints)
    })
}

/// Write the given endpoints to disk.
fn save(endpoints: &HashMap<String, String>) {
    let path = path();
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
fn path() -> PathBuf {
    DataType::Persistent.dir_path().join(ENDPOINTS_FILE)
}
