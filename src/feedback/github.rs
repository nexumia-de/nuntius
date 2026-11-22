//! Create a GitHub issue with the user's own token, through the REST API.

use std::time::Duration;

use matrix_sdk::reqwest::{self, StatusCode, header};
use serde::Deserialize;

use super::report::Report;
use crate::{application::APP_HOMEPAGE_URL, config::VERSION};

/// The GitHub REST API.
const API: &str = "https://api.github.com";
/// How long to wait for GitHub.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Why sending failed, for the dialog's message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SendError {
    /// The token was rejected, or it may not write issues.
    NoAccess,
    /// No connection to GitHub.
    Offline,
    /// Anything else.
    Failed,
}

impl SendError {
    /// The message of the dialog's banner.
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::NoAccess => {
                "GitHub did not accept the token. Check that it may write issues in this \
                 repository."
            }
            Self::Offline => "GitHub cannot be reached right now.",
            Self::Failed => "GitHub did not create the issue.",
        }
    }

    /// The reason for log lines.
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::NoAccess => "no-access",
            Self::Offline => "offline",
            Self::Failed => "failed",
        }
    }

    /// The error for an answer of GitHub that is not a success.
    fn from_status(status: StatusCode) -> Self {
        match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND => {
                Self::NoAccess
            }
            _ => Self::Failed,
        }
    }

    /// The error for a request that got no answer.
    fn from_request_error(error: &reqwest::Error) -> Self {
        if error.is_connect() || error.is_timeout() {
            Self::Offline
        } else {
            Self::Failed
        }
    }
}

/// The part of GitHub's answer we need.
#[derive(Deserialize)]
struct CreatedIssue {
    html_url: String,
}

/// The JSON body of the request.
fn payload(report: &Report) -> serde_json::Value {
    serde_json::json!({
        "title": report.title,
        "body": report.body(),
        "labels": [report.kind.issue_label()],
    })
}

/// The web address of the created issue in GitHub's answer.
fn issue_url(answer: &[u8]) -> Result<String, SendError> {
    serde_json::from_slice::<CreatedIssue>(answer)
        .map(|issue| issue.html_url)
        .map_err(|_| SendError::Failed)
}

/// Open an issue for the report in `repo` (`owner/name`).
///
/// Returns the web address of the issue. GitHub ignores the labels when the
/// token's user may not set them. Must run on the tokio runtime.
pub(crate) async fn create_issue(
    token: &str,
    repo: &str,
    report: &Report,
) -> Result<String, SendError> {
    let client = reqwest::Client::builder()
        .user_agent(format!("Nuntius/{VERSION} (+{APP_HOMEPAGE_URL})"))
        .timeout(TIMEOUT)
        .build()
        .map_err(|_| SendError::Failed)?;

    let response = client
        .post(format!("{API}/repos/{repo}/issues"))
        .bearer_auth(token)
        .header(header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header(header::CONTENT_TYPE, "application/json")
        .body(payload(report).to_string())
        .send()
        .await
        .map_err(|error| SendError::from_request_error(&error))?;

    let status = response.status();
    if !status.is_success() {
        return Err(SendError::from_status(status));
    }

    let answer = response
        .bytes()
        .await
        .map_err(|error| SendError::from_request_error(&error))?;
    issue_url(&answer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feedback::report::{Kind, PHONE};

    #[test]
    fn status_errors() {
        for status in [
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ] {
            assert_eq!(SendError::from_status(status), SendError::NoAccess);
        }
        for status in [
            StatusCode::GONE,
            StatusCode::UNPROCESSABLE_ENTITY,
            StatusCode::INTERNAL_SERVER_ERROR,
        ] {
            assert_eq!(SendError::from_status(status), SendError::Failed);
        }
    }

    #[test]
    fn payload_has_title_body_and_label() {
        let report = Report::new(Kind::Problem, "Crash", "It crashed.", "0.1.0", PHONE);
        let payload = payload(&report);
        assert_eq!(payload["title"], "Crash");
        assert_eq!(payload["body"], report.body());
        assert_eq!(payload["labels"], serde_json::json!(["bug"]));
    }

    #[test]
    fn issue_url_from_answer() {
        let answer = br#"{"number":7,"html_url":"https://github.com/o/r/issues/7"}"#;
        assert_eq!(
            issue_url(answer).as_deref(),
            Ok("https://github.com/o/r/issues/7")
        );
        assert_eq!(issue_url(b"{}"), Err(SendError::Failed));
        assert_eq!(issue_url(b"<html>"), Err(SendError::Failed));
    }
}
