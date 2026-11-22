//! The "Report a Problem" dialog: the user writes a problem or an idea; with a
//! GitHub token it becomes an issue right away, without one GitHub's issue
//! form opens in the browser, filled in.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use adw::prelude::*;
use gtk::gio;
use tracing::{info, warn};
use zeroize::Zeroizing;

use super::{
    REPO, github,
    report::{self, Kind, Report},
    token,
};
use crate::{application::APP_NAME, config::VERSION, spawn, spawn_tokio};

/// GitHub's form for a classic token with the `public_repo` scope, which may
/// open issues in any public repository. A fine-grained token can write only
/// to repositories of the owner it is made for, so it works only for members
/// of that owner.
const TOKEN_PAGE: &str =
    "https://github.com/settings/tokens/new?scopes=public_repo&description=Report%20a%20Problem";

/// The widgets and state of an open dialog.
struct ReportDialog {
    dialog: adw::Dialog,
    /// The window the dialog is presented over, for opening web pages.
    parent: gtk::Window,
    toolbar: adw::ToolbarView,
    banner: adw::Banner,
    kind_row: adw::ComboRow,
    title_row: adw::EntryRow,
    text_group: adw::PreferencesGroup,
    text: gtk::TextView,
    token_row: adw::PasswordEntryRow,
    help_row: adw::ActionRow,
    saved_row: adw::ActionRow,
    form_button: gtk::Button,
    send_button: gtk::Button,
    device: &'static str,
    /// The token in the keyring, once looked up.
    saved: RefCell<Option<Zeroizing<String>>>,
    sending: Cell<bool>,
}

/// Present the dialog over `parent`.
pub(crate) fn present(parent: &gtk::Window) {
    let this = ReportDialog::new(parent);
    this.connect_signals();
    this.update();
    this.dialog.present(Some(parent));

    // The dialog keeps its state until it is closed; the signal handlers only
    // hold it weakly.
    let keep = RefCell::new(Some(this.clone()));
    this.dialog.connect_closed(move |_| {
        keep.take();
    });

    let weak = Rc::downgrade(&this);
    spawn!(async move {
        let saved = token::lookup().await;
        if let Some(this) = weak.upgrade() {
            this.saved.replace(saved);
            this.update();
        }
    });
}

impl ReportDialog {
    fn new(parent: &gtk::Window) -> Rc<Self> {
        let dialog = adw::Dialog::builder()
            .title("Report a Problem")
            .content_width(480)
            .content_height(680)
            .build();

        let page = adw::PreferencesPage::new();

        let about = adw::PreferencesGroup::builder()
            .description(format!(
                "Goes to the public issue list of {APP_NAME} on GitHub. Only what you write \
                 here, the app version and the kind of device are sent."
            ))
            .build();
        let kinds = Kind::ALL.map(Kind::label);
        let kind_row = adw::ComboRow::builder()
            .title("Kind")
            .model(&gtk::StringList::new(&kinds))
            .build();
        let title_row = adw::EntryRow::builder().title("Title").build();
        about.add(&kind_row);
        about.add(&title_row);
        page.add(&about);

        let text_group = adw::PreferencesGroup::new();
        let text = gtk::TextView::builder()
            .wrap_mode(gtk::WrapMode::WordChar)
            .accepts_tab(false)
            .top_margin(12)
            .bottom_margin(12)
            .left_margin(12)
            .right_margin(12)
            .height_request(160)
            .build();
        text_group.add(&gtk::Frame::builder().child(&text).build());
        page.add(&text_group);

        let direct = adw::PreferencesGroup::builder()
            .title("Send Directly")
            .description(
                "With a GitHub token the issue is created here. Without one, GitHub opens in \
                 the browser with everything filled in.",
            )
            .build();
        let token_row = adw::PasswordEntryRow::builder()
            .title("GitHub Token")
            .build();
        let help_row = adw::ActionRow::builder()
            .title("Create a Token")
            .subtitle("Classic, with the “public_repo” scope")
            .activatable(true)
            .build();
        help_row.add_suffix(&gtk::Image::from_icon_name("adw-external-link-symbolic"));
        let saved_row = adw::ActionRow::builder()
            .title("Token Saved")
            .subtitle("Kept in the keyring")
            .build();
        direct.add(&token_row);
        direct.add(&help_row);
        direct.add(&saved_row);
        page.add(&direct);

        // Bottom bar, in thumb reach: the main action on the right.
        let bar = gtk::Box::builder()
            .spacing(12)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        let form_button = gtk::Button::with_label("Open on GitHub");
        let send_button = gtk::Button::with_label("Send");
        for button in [&form_button, &send_button] {
            button.add_css_class("pill");
            button.set_hexpand(true);
            bar.append(button);
        }

        let banner = adw::Banner::builder()
            .button_label("Open on GitHub")
            .build();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.add_top_bar(&banner);
        toolbar.set_content(Some(&page));
        toolbar.add_bottom_bar(&bar);
        dialog.set_child(Some(&toolbar));

        Rc::new(Self {
            dialog,
            parent: parent.clone(),
            toolbar,
            banner,
            kind_row,
            title_row,
            text_group,
            text,
            token_row,
            help_row,
            saved_row,
            form_button,
            send_button,
            device: report::device(),
            saved: RefCell::default(),
            sending: Cell::default(),
        })
    }

    /// Connect the signals of the widgets; the handlers hold the dialog
    /// weakly.
    fn connect_signals(self: &Rc<Self>) {
        fn with(weak: &Weak<ReportDialog>, f: impl Fn(&Rc<ReportDialog>)) {
            if let Some(this) = weak.upgrade() {
                f(&this);
            }
        }

        let weak = Rc::downgrade(self);
        self.kind_row
            .connect_selected_notify(move |_| with(&weak, |this| this.update()));
        let weak = Rc::downgrade(self);
        self.title_row
            .connect_changed(move |_| with(&weak, |this| this.update()));
        let weak = Rc::downgrade(self);
        self.text
            .buffer()
            .connect_changed(move |_| with(&weak, |this| this.update()));
        let weak = Rc::downgrade(self);
        self.token_row
            .connect_changed(move |_| with(&weak, |this| this.update()));
        let weak = Rc::downgrade(self);
        self.help_row
            .connect_activated(move |_| with(&weak, |this| this.open(TOKEN_PAGE)));
        let forget = gtk::Button::builder()
            .label("Forget")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        let weak = Rc::downgrade(self);
        forget.connect_clicked(move |_| with(&weak, ReportDialog::forget_token));
        self.saved_row.add_suffix(&forget);
        let weak = Rc::downgrade(self);
        self.banner
            .connect_button_clicked(move |_| with(&weak, |this| this.open_form()));
        let weak = Rc::downgrade(self);
        self.form_button
            .connect_clicked(move |_| with(&weak, |this| this.open_form()));
        let weak = Rc::downgrade(self);
        self.send_button
            .connect_clicked(move |_| with(&weak, ReportDialog::send));
    }

    // State

    fn kind(&self) -> Kind {
        Kind::ALL
            .get(self.kind_row.selected() as usize)
            .copied()
            .unwrap_or(Kind::Problem)
    }

    fn report(&self) -> Report {
        let buffer = self.text.buffer();
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        Report::new(
            self.kind(),
            &self.title_row.text(),
            &text,
            VERSION,
            self.device,
        )
    }

    fn has_saved_token(&self) -> bool {
        self.saved.borrow().is_some()
    }

    /// The saved token, else the typed one.
    fn token(&self) -> Option<Zeroizing<String>> {
        if let Some(saved) = &*self.saved.borrow() {
            return Some(saved.clone());
        }
        let typed = self.token_row.text();
        let typed = typed.trim();
        (!typed.is_empty()).then(|| Zeroizing::new(typed.to_owned()))
    }

    fn update(&self) {
        let report = self.report();
        let heading = report.kind.heading();
        self.text_group.set_title(heading);
        self.text
            .update_property(&[gtk::accessible::Property::Label(heading)]);

        let saved = self.has_saved_token();
        self.token_row.set_visible(!saved);
        self.help_row.set_visible(!saved);
        self.saved_row.set_visible(saved);

        let direct = self.token().is_some();
        self.send_button.set_visible(direct);
        self.send_button
            .set_sensitive(report.is_complete() && !self.sending.get());
        for (button, main) in [(&self.send_button, direct), (&self.form_button, !direct)] {
            if main {
                button.add_css_class("suggested-action");
            } else {
                button.remove_css_class("suggested-action");
            }
        }
    }

    fn forget_token(self: &Rc<Self>) {
        self.saved.replace(None);
        self.token_row.set_text("");
        self.update();
        spawn!(token::clear());
    }

    // Actions

    /// Open a web page in the browser.
    fn open(&self, url: &str) {
        gtk::UriLauncher::new(url).launch(Some(&self.parent), gio::Cancellable::NONE, |result| {
            // The error can name the address, which holds what the user typed.
            if result.is_err() {
                warn!("Could not open a web page");
            }
        });
    }

    fn send(self: &Rc<Self>) {
        let report = self.report();
        let Some(token) = self.token() else {
            return;
        };
        if !report.is_complete() || self.sending.get() {
            return;
        }
        let typed = !self.has_saved_token();

        self.sending.set(true);
        self.banner.set_revealed(false);
        self.send_button.set_label("Sending…");
        self.update();

        let weak = Rc::downgrade(self);
        spawn!(async move {
            let request_token = token.clone();
            let result =
                spawn_tokio!(
                    async move { github::create_issue(&request_token, REPO, &report).await }
                )
                .await
                .expect("task was not aborted");

            match &result {
                Ok(_) => {
                    info!("Issue created in {REPO}");
                    if typed {
                        token::store(token).await;
                    }
                }
                Err(error) => warn!("Issue not created in {REPO}: {}", error.reason()),
            }

            let Some(this) = weak.upgrade() else {
                return;
            };
            this.sending.set(false);
            this.send_button.set_label("Send");
            match result {
                Ok(url) => {
                    if typed {
                        this.saved.replace(this.token());
                    }
                    this.show_sent(url);
                }
                Err(error) => {
                    this.banner.set_title(error.message());
                    this.banner.set_revealed(true);
                    this.update();
                }
            }
        });
    }

    /// GitHub's issue form in the browser. When the text had to be
    /// shortened, the full text is on the clipboard to paste.
    fn open_form(&self) {
        let report = self.report();
        let (url, shortened) = report.form_url(REPO);
        if shortened {
            self.dialog.clipboard().set_text(&report.text);
        }
        self.open(&url);
        self.dialog.close();
    }

    fn show_sent(self: &Rc<Self>, url: String) {
        let button = gtk::Button::builder()
            .label("View Issue")
            .halign(gtk::Align::Center)
            .css_classes(["pill"])
            .build();
        let weak = Rc::downgrade(self);
        button.connect_clicked(move |_| {
            if let Some(this) = weak.upgrade() {
                this.open(&url);
            }
        });
        let page = adw::StatusPage::builder()
            .icon_name("emblem-ok-symbolic")
            .title("Thank You")
            .description("The issue is on GitHub.")
            .child(&button)
            .build();
        self.banner.set_revealed(false);
        self.toolbar.set_content(Some(&page));
        self.toolbar.set_reveal_bottom_bars(false);
    }
}
