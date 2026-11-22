//! The user's GitHub token in the keyring, through the same Secret Service
//! or Secret portal as the Matrix sessions.
//!
//! The token has its own schema, so the search for stored sessions never
//! finds it. Errors are logged without the token and treated as "no token".

use std::cfg_select;

cfg_select! {
    target_os = "linux" => {
        pub(crate) use self::linux::{clear, lookup, store};
    }
    _ => {
        /// No keyring on this platform: there is never a saved token.
        pub(crate) async fn lookup() -> Option<zeroize::Zeroizing<String>> {
            None
        }

        /// No keyring on this platform: the token is not kept.
        pub(crate) async fn store(_token: zeroize::Zeroizing<String>) {}

        /// No keyring on this platform: nothing to remove.
        pub(crate) async fn clear() {}
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::collections::HashMap;

    use oo7::{Keyring, Secret};
    use tracing::warn;
    use zeroize::Zeroizing;

    use crate::{APP_ID, application::APP_NAME, spawn_tokio};

    /// The attribute for the schema in the Secret Service.
    const XDG_SCHEMA: &str = "xdg:schema";

    /// The schema of the token's item.
    fn schema() -> String {
        format!("{APP_ID}.GitHubToken")
    }

    /// The attributes of the token's item.
    fn attributes(schema: &str) -> HashMap<&str, &str> {
        HashMap::from([(XDG_SCHEMA, schema)])
    }

    /// The saved token, if there is one.
    pub(crate) async fn lookup() -> Option<Zeroizing<String>> {
        let result = spawn_tokio!(async move {
            let schema = schema();
            let keyring = Keyring::new().await?;
            keyring.unlock().await?;
            let items = keyring.search_items(&attributes(&schema)).await?;
            let Some(item) = items.into_iter().next() else {
                return Ok(None);
            };
            item.unlock().await?;
            let secret = item.secret().await?;
            Ok::<_, oo7::Error>(
                String::from_utf8(secret.as_bytes().to_vec())
                    .ok()
                    .map(Zeroizing::new),
            )
        })
        .await
        .expect("task was not aborted");

        result.unwrap_or_else(|error| {
            warn!("Could not read the GitHub token from the keyring: {error}");
            None
        })
    }

    /// Keep the token, replacing a saved one.
    pub(crate) async fn store(token: Zeroizing<String>) {
        let result = spawn_tokio!(async move {
            let schema = schema();
            let keyring = Keyring::new().await?;
            keyring
                .create_item(
                    &format!("{APP_NAME} GitHub token"),
                    &attributes(&schema),
                    Secret::text(token.as_str()),
                    true,
                )
                .await
        })
        .await
        .expect("task was not aborted");

        if let Err(error) = result {
            warn!("Could not store the GitHub token in the keyring: {error}");
        }
    }

    /// Remove the saved token.
    pub(crate) async fn clear() {
        let result = spawn_tokio!(async move {
            let schema = schema();
            let keyring = Keyring::new().await?;
            keyring.delete(&attributes(&schema)).await
        })
        .await
        .expect("task was not aborted");

        if let Err(error) = result {
            warn!("Could not remove the GitHub token from the keyring: {error}");
        }
    }
}
