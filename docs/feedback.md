---
type: Concept
title: Report a problem
description: How Nuntius lets users report a problem or an idea as a GitHub issue from inside the app - in Rust, along the toolkit's feedback guideline.
tags: [feedback, issues, github, keyring]
generated: { by: claude-code/2.1.317, at: 2026-11-22T23:30:00Z }
sources:
  - id: guideline
    resource: ../../phosh-toolkit/docs/feedback.md
    title: phosh-toolkit, Report a problem from the app
  - id: reference
    resource: ../../phosh-toolkit/lib/phosh_toolkit/feedback_dialog.py
    title: phosh-toolkit, Python reference dialog (feedback.py, feedback_report.py, feedback_github.py, feedback_dialog.py)
  - id: gh-issues-api
    resource: https://docs.github.com/en/rest/issues/issues#create-an-issue
    title: GitHub REST API, create an issue
---

# Report a problem

The main menu (sidebar, next to About) has **Report a Problem…**
(`app.report-problem`). The dialog follows the toolkit's guideline[^guideline]
with the same layout, texts and privacy rules as the Python
reference[^reference]: Kind (Problem or Idea), Title, the text, a
"Send Directly" group for the user's GitHub token, and a bottom bar with
**Open on GitHub** and **Send**. Nuntius is Rust, so it cannot use
`phosh_toolkit.feedback`; the code is in `src/feedback/`, one job per
file:

| File | Job |
|---|---|
| `report.rs` | The report as issue body and as filled-in form address; device kind from `XDG_CURRENT_DESKTOP` |
| `github.rs` | `POST /repos/nexumia-de/nuntius/issues`[^gh-issues-api] with matrix-sdk's re-exported `reqwest`; maps the answer to no access (401/403/404), offline (no connection, timeout) or failed |
| `token.rs` | The token in the keyring through `oo7`, like the Matrix sessions, under its own schema `de.nexumia.Nuntius.GitHubToken` so the session search never sees it |
| `dialog.rs` | The `AdwDialog`, built in code |
| `mod.rs` | `present()` and `issue_url()` for the About dialog |

- "Create a Token" opens GitHub's form for a classic token with the
  `public_repo` scope ticked: a fine-grained token can write only to
  repositories of the owner it is made for, so it would work only for
  members of nexumia-de.
- The token is stored only after GitHub created the issue with it;
  **Forget** deletes the keyring item. Without a Secret Service (other
  platforms) no token is kept.
- The About dialog's **Report an Issue** opens the bug form with version
  and device filled in (`feedback::issue_url()`).
- Only the title, the text, the version (`config::VERSION`, the fork's
  own) and `Phone (Phosh)` or `Desktop` are sent. Log lines name the
  repository and the outcome (`no-access`, `offline`, `failed`), never
  title, text or token.
- A text too long for an address (over 7000 bytes) is shortened in the
  form with a note, and the full text goes to the clipboard.
- The texts are English only, like the fork's other own texts; they are
  not in upstream's translations.

## Issue forms

`.github/ISSUE_TEMPLATE/bug.yml` (fields `what`, `steps`, `version`,
`device`, `logs`; label `bug`) and `feature.yml` (fields `problem`,
`idea`; label `enhancement`), modelled on Pensum's. The dialog fills
`what`, `version` and `device`, or `problem`; through the API the body has
the same `### What happened` / `### Version` / `### Device` sections.
