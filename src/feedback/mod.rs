//! Report a problem or an idea from inside the app, as a GitHub issue: with
//! the user's own token directly, otherwise through GitHub's issue form in
//! the browser (`docs/feedback.md`).

mod dialog;
mod github;
mod report;
mod token;

pub(crate) use self::dialog::present;
use self::report::{Kind, Report};
use crate::config::VERSION;

/// The repository that takes the reports.
const REPO: &str = "nexumia-de/nuntius";

/// The repository's bug form with the version and the device filled in, for
/// the About dialog.
pub(crate) fn issue_url() -> String {
    Report::new(Kind::Problem, "", "", VERSION, report::device())
        .form_url(REPO)
        .0
}
