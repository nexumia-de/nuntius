//! What a report says, as a GitHub issue body or as an address of GitHub's
//! new-issue form.
//!
//! Follows the issue forms in `.github/ISSUE_TEMPLATE/` (`bug.yml` and
//! `feature.yml`), so a report reads the same whichever way it was sent. No
//! GTK.

use url::form_urlencoded;

/// The device option of the bug form under Phosh.
pub(crate) const PHONE: &str = "Phone (Phosh)";
/// The device option of the bug form everywhere else.
pub(crate) const DESKTOP: &str = "Desktop";

/// GitHub serves addresses up to about 8 KiB; stay well below.
pub(crate) const MAX_URL: usize = 7000;
/// The note at the end of a text shortened to fit in an address.
const CUT: &str = "\n\n[…shortened, please paste the rest]";

/// The kind of a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Something does not work: the bug form.
    Problem,
    /// Something is missing: the feature form.
    Idea,
}

impl Kind {
    /// All kinds, in the order of the dialog's combo row.
    pub(crate) const ALL: [Self; 2] = [Self::Problem, Self::Idea];

    /// The label in the dialog.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Problem => "Problem",
            Self::Idea => "Idea",
        }
    }

    /// The file name of the issue form.
    fn template(self) -> &'static str {
        match self {
            Self::Problem => "bug.yml",
            Self::Idea => "feature.yml",
        }
    }

    /// The label of the issue.
    pub(crate) fn issue_label(self) -> &'static str {
        match self {
            Self::Problem => "bug",
            Self::Idea => "enhancement",
        }
    }

    /// The id of the form field that takes the user's text.
    fn field(self) -> &'static str {
        match self {
            Self::Problem => "what",
            Self::Idea => "problem",
        }
    }

    /// The heading of that field in the rendered issue (its label in the
    /// form).
    pub(crate) fn heading(self) -> &'static str {
        match self {
            Self::Problem => "What happened",
            Self::Idea => "What do you want to do",
        }
    }

    /// Whether the form has the `version` and `device` fields.
    fn has_details(self) -> bool {
        matches!(self, Self::Problem)
    }
}

/// A report: what the user typed, the app version and the kind of device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Report {
    pub(crate) kind: Kind,
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) version: String,
    pub(crate) device: &'static str,
}

impl Report {
    /// Construct a report; title and text are trimmed.
    pub(crate) fn new(
        kind: Kind,
        title: &str,
        text: &str,
        version: &str,
        device: &'static str,
    ) -> Self {
        Self {
            kind,
            title: title.trim().to_owned(),
            text: text.trim().to_owned(),
            version: version.to_owned(),
            device,
        }
    }

    /// Whether the report has a title and a text, as sending requires.
    pub(crate) fn is_complete(&self) -> bool {
        !self.title.is_empty() && !self.text.is_empty()
    }

    /// Markdown like GitHub renders the issue form: one `###` section per
    /// field.
    pub(crate) fn body(&self) -> String {
        let mut sections = vec![(self.kind.heading(), self.text.as_str())];
        if self.kind.has_details() {
            sections.push(("Version", &self.version));
            sections.push(("Device", self.device));
        }

        sections
            .into_iter()
            .map(|(heading, value)| {
                let value = if value.is_empty() {
                    "_No response_"
                } else {
                    value
                };
                format!("### {heading}\n\n{value}")
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// The address of the repository's issue form for this report's kind,
    /// filled in, and whether the text had to be shortened to fit.
    ///
    /// A shortened text ends with a note asking to paste the rest.
    pub(crate) fn form_url(&self, repo: &str) -> (String, bool) {
        let url = self.form_url_with_text(repo, &self.text);
        if url.len() <= MAX_URL {
            return (url, false);
        }

        // The longest start of the text, in characters, whose address fits.
        let shortened = |keep: usize| {
            let text = if keep == 0 {
                String::new()
            } else {
                let kept = self.text.chars().take(keep).collect::<String>();
                format!("{}{CUT}", kept.trim_end())
            };
            self.form_url_with_text(repo, &text)
        };
        let (mut fits, mut too_long) = (0, self.text.chars().count());
        while too_long - fits > 1 {
            let middle = fits + (too_long - fits) / 2;
            if shortened(middle).len() <= MAX_URL {
                fits = middle;
            } else {
                too_long = middle;
            }
        }
        (shortened(fits), true)
    }

    fn form_url_with_text(&self, repo: &str, text: &str) -> String {
        let mut query = form_urlencoded::Serializer::new(String::new());
        query
            .append_pair("template", self.kind.template())
            .append_pair("title", &self.title)
            .append_pair(self.kind.field(), text);
        if self.kind.has_details() {
            query
                .append_pair("version", &self.version)
                .append_pair("device", self.device);
        }
        format!("https://github.com/{repo}/issues/new?{}", query.finish())
    }
}

/// The kind of device for the bug form: [`PHONE`] when
/// `XDG_CURRENT_DESKTOP` names Phosh, else [`DESKTOP`].
pub(crate) fn device() -> &'static str {
    device_of(std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref())
}

fn device_of(current_desktop: Option<&str>) -> &'static str {
    let is_phosh = current_desktop
        .unwrap_or_default()
        .split(':')
        .any(|desktop| desktop.eq_ignore_ascii_case("phosh"));
    if is_phosh { PHONE } else { DESKTOP }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPO: &str = "nexumia-de/nuntius";

    fn query(url: &str) -> Vec<(String, String)> {
        let (_, query) = url.split_once('?').unwrap();
        form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect()
    }

    #[test]
    fn device_from_current_desktop() {
        assert_eq!(device_of(Some("Phosh:GNOME")), PHONE);
        assert_eq!(device_of(Some("phosh")), PHONE);
        assert_eq!(device_of(Some("GNOME")), DESKTOP);
        assert_eq!(device_of(Some("")), DESKTOP);
        assert_eq!(device_of(None), DESKTOP);
    }

    #[test]
    fn new_trims_title_and_text() {
        let report = Report::new(Kind::Idea, "  Title ", "\n text \n", "0.1.0", DESKTOP);
        assert_eq!(report.title, "Title");
        assert_eq!(report.text, "text");
        assert!(report.is_complete());
        assert!(!Report::new(Kind::Idea, "Title", " ", "0.1.0", DESKTOP).is_complete());
    }

    #[test]
    fn problem_body_has_version_and_device() {
        let report = Report::new(Kind::Problem, "Crash", "It crashed.", "0.1.0", PHONE);
        assert_eq!(
            report.body(),
            "### What happened\n\nIt crashed.\n\n### Version\n\n0.1.0\n\n\
             ### Device\n\nPhone (Phosh)"
        );
        assert_eq!(report.kind.issue_label(), "bug");
    }

    #[test]
    fn idea_body_has_only_the_text() {
        let report = Report::new(Kind::Idea, "Polls", "Vote in rooms.", "0.1.0", PHONE);
        assert_eq!(
            report.body(),
            "### What do you want to do\n\nVote in rooms."
        );
        assert_eq!(report.kind.issue_label(), "enhancement");
    }

    #[test]
    fn empty_text_is_no_response() {
        let report = Report::new(Kind::Problem, "", "", "0.1.0", DESKTOP);
        assert!(
            report
                .body()
                .starts_with("### What happened\n\n_No response_")
        );
    }

    #[test]
    fn problem_form_url() {
        let report = Report::new(Kind::Problem, "A b", "x & y", "0.1.0", PHONE);
        let (url, shortened) = report.form_url(REPO);
        assert!(!shortened);
        assert!(url.starts_with("https://github.com/nexumia-de/nuntius/issues/new?"));
        let pairs = [
            ("template", "bug.yml"),
            ("title", "A b"),
            ("what", "x & y"),
            ("version", "0.1.0"),
            ("device", "Phone (Phosh)"),
        ]
        .map(|(key, value)| (key.to_owned(), value.to_owned()));
        assert_eq!(query(&url), pairs);
    }

    #[test]
    fn idea_form_url() {
        let report = Report::new(Kind::Idea, "Polls", "Vote", "0.1.0", DESKTOP);
        let (url, _) = report.form_url(REPO);
        let pairs = [
            ("template", "feature.yml"),
            ("title", "Polls"),
            ("problem", "Vote"),
        ]
        .map(|(key, value)| (key.to_owned(), value.to_owned()));
        assert_eq!(query(&url), pairs);
    }

    #[test]
    fn long_text_is_shortened() {
        for text in ["ä ".repeat(5000), "x".repeat(20000), "€".repeat(3000)] {
            let report = Report::new(Kind::Problem, "Long", &text, "0.1.0", DESKTOP);
            let (url, shortened) = report.form_url(REPO);
            assert!(shortened);
            assert!(url.len() <= MAX_URL);
            let what = query(&url)
                .into_iter()
                .find(|(key, _)| key == "what")
                .unwrap()
                .1;
            assert!(what.ends_with("please paste the rest]"));
            assert!(what.len() > 1000);
        }
    }
}
