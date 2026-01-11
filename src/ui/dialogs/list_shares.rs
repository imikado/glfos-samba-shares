use crate::samba::SambaShareConfig;
use crate::ui::dialogs::EditShareDialog;
use gettextrs::gettext;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

pub struct ListSharesDialog {
    window: adw::Window,
    toast_overlay: adw::ToastOverlay,
}

impl ListSharesDialog {
    pub fn new() -> Self {
        let window = adw::Window::new();
        window.set_title(Some(&gettext("Samba Shares")));
        window.set_default_size(700, 500);
        window.set_modal(true);

        // Create toolbar header
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        // Close button
        let close_button = gtk4::Button::with_label(&gettext("Close"));
        header_bar.pack_start(&close_button);

        // Create scrolled window for shares list
        let scrolled = gtk4::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        // Create preferences page
        let preferences_page = adw::PreferencesPage::new();

        // Load shares from configuration
        match SambaShareConfig::load_all() {
            Ok(shares) => {
                if shares.is_empty() {
                    // Show empty state
                    let empty_group = adw::PreferencesGroup::new();
                    let status = adw::StatusPage::new();
                    status.set_title(&gettext("No Shares Configured"));
                    status.set_description(Some(&gettext("Click 'Setup New Share' to add your first share")));
                    status.set_icon_name(Some("folder-open-symbolic"));

                    let empty_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                    empty_box.append(&status);
                    empty_group.add(&empty_box);
                    preferences_page.add(&empty_group);
                } else {
                    // Create a group for each share
                    for share in shares {
                        let group = adw::PreferencesGroup::new();
                        group.set_title(&share.name);

                        // Path row
                        let path_row = adw::ActionRow::new();
                        path_row.set_title(&gettext("Path"));
                        path_row.set_subtitle(&share.path);
                        group.add(&path_row);

                        // Settings summary
                        let settings = format!(
                            "Browsable: {} • Read Only: {} • Guest OK: {}",
                            if share.browsable { "Yes" } else { "No" },
                            if share.read_only { "Yes" } else { "No" },
                            if share.guest_ok { "Yes" } else { "No" }
                        );
                        let settings_row = adw::ActionRow::new();
                        settings_row.set_title(&gettext("Settings"));
                        settings_row.set_subtitle(&settings);
                        group.add(&settings_row);

                        // User/Group row
                        let user_group_text = format!("User: {} • Group: {}", share.force_user, share.force_group);
                        let user_group_row = adw::ActionRow::new();
                        user_group_row.set_title(&gettext("User &amp; Group"));
                        user_group_row.set_subtitle(&user_group_text);
                        group.add(&user_group_row);

                        // Edit button
                        let edit_button = gtk4::Button::with_label(&gettext("Edit"));
                        edit_button.set_valign(gtk4::Align::Center);
                        edit_button.add_css_class("flat");

                        let share_clone = share.clone();
                        let window_clone_for_edit = window.clone();
                        edit_button.connect_clicked(move |_| {
                            let edit_dialog = EditShareDialog::new(&share_clone);
                            edit_dialog.present(Some(&window_clone_for_edit));
                        });

                        let button_row = adw::ActionRow::new();
                        button_row.add_suffix(&edit_button);
                        group.add(&button_row);

                        preferences_page.add(&group);
                    }
                }
            }
            Err(e) => {
                // Show error state
                let error_group = adw::PreferencesGroup::new();
                let status = adw::StatusPage::new();
                status.set_title(&gettext("Error Loading Shares"));
                status.set_description(Some(&e));
                status.set_icon_name(Some("dialog-error-symbolic"));

                let error_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                error_box.append(&status);
                error_group.add(&error_box);
                preferences_page.add(&error_group);
            }
        }

        scrolled.set_child(Some(&preferences_page));
        toolbar_view.set_content(Some(&scrolled));

        // Wrap in toast overlay
        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&toolbar_view));

        window.set_content(Some(&toast_overlay));

        // Handle close button
        let window_clone = window.clone();
        close_button.connect_clicked(move |_| {
            window_clone.close();
        });

        Self {
            window,
            toast_overlay,
        }
    }

    pub fn present(&self, parent: Option<&impl IsA<gtk4::Widget>>) {
        if let Some(p) = parent {
            if let Some(parent_window) = p.dynamic_cast_ref::<gtk4::Window>() {
                self.window.set_transient_for(Some(parent_window));
            }
        }
        self.window.present();
    }

    pub fn window(&self) -> &adw::Window {
        &self.window
    }
}
