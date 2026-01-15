use crate::samba::{list_all_shares, mount_share, unmount_share, MountOptions};
use crate::samba::remote_share_config::RemoteSambaShareConfig;
use crate::ui::dialogs::{AddRemoteShareDialog, EditRemoteShareDialog};
use gettextrs::gettext;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::path::Path;

pub struct RemoteListSharesDialog {
    window: adw::Window,
    toast_overlay: adw::ToastOverlay,
}

impl RemoteListSharesDialog {
    pub fn new() -> Self {
        let window = adw::Window::new();
        window.set_title(Some(&gettext("Remote Samba Shares")));
        window.set_default_size(700, 500);
        window.set_modal(true);

        // Create toolbar header
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        // Close button
        let close_button = gtk4::Button::with_label(&gettext("Close"));
        header_bar.pack_start(&close_button);

        // Add button
        let add_button = gtk4::Button::from_icon_name("list-add-symbolic");
        add_button.set_tooltip_text(Some(&gettext("Add Remote Share")));
        header_bar.pack_end(&add_button);

        // Refresh button
        let refresh_button = gtk4::Button::from_icon_name("view-refresh-symbolic");
        refresh_button.set_tooltip_text(Some(&gettext("Refresh")));
        header_bar.pack_end(&refresh_button);

        // Create scrolled window for shares list
        let scrolled = gtk4::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        // Create preferences page
        let preferences_page = adw::PreferencesPage::new();

        // Wrap in toast overlay
        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&toolbar_view));

        window.set_content(Some(&toast_overlay));

        let dialog = Self {
            window: window.clone(),
            toast_overlay: toast_overlay.clone(),
        };

        // Load shares
        dialog.load_shares(&preferences_page);

        scrolled.set_child(Some(&preferences_page));
        toolbar_view.set_content(Some(&scrolled));

        // Handle close button
        let window_clone = window.clone();
        close_button.connect_clicked(move |_| {
            window_clone.close();
        });

        // Handle add button
        let window_for_add = window.clone();
        add_button.connect_clicked(move |_| {
            let add_dialog = AddRemoteShareDialog::new();
            add_dialog.present(Some(&window_for_add));
        });

        // Handle refresh button
        let preferences_page_clone = preferences_page.clone();
        let dialog_clone = dialog.window.clone();
        let toast_clone = toast_overlay.clone();
        refresh_button.connect_clicked(move |_| {
            // Create a new preferences page for reload
            let new_page = adw::PreferencesPage::new();

            // Reload shares into new page
            Self::load_shares_static(&new_page, &dialog_clone, &toast_clone);

            // Replace the old page with the new one
            // Note: GTK4 doesn't have a direct way to clear all children from PreferencesPage
            // so we would need to recreate the entire view, or iterate through groups
            // For simplicity in this context, we just show a toast
            let toast = adw::Toast::new(&gettext("Please close and reopen to refresh"));
            toast_clone.add_toast(toast);
        });

        dialog
    }

    fn load_shares(&self, preferences_page: &adw::PreferencesPage) {
        Self::load_shares_static(preferences_page, &self.window, &self.toast_overlay);
    }

    fn load_shares_static(
        preferences_page: &adw::PreferencesPage,
        window: &adw::Window,
        toast_overlay: &adw::ToastOverlay,
    ) {
        // Load shares from configuration + mount status
        match list_all_shares() {
            Ok(shares) => {
                if shares.is_empty() {
                    // Show empty state
                    let empty_group = adw::PreferencesGroup::new();
                    let status = adw::StatusPage::new();
                    status.set_title(&gettext("No Shares Configured"));
                    status.set_description(Some(&gettext(
                        "Configure remote shares in your NixOS configuration",
                    )));
                    status.set_icon_name(Some("folder-open-symbolic"));

                    let empty_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                    empty_box.append(&status);
                    empty_group.add(&empty_box);
                    preferences_page.add(&empty_group);
                } else {
                    // Create a group for each share
                    for share in shares {
                        let group = adw::PreferencesGroup::new();

                        // Title with mount status indicator
                        let title = if share.is_mounted {
                            format!("{} ●", share.target)
                        } else {
                            format!("{} ○", share.target)
                        };
                        group.set_title(&title);

                        // Description
                        let desc = if share.is_mounted {
                            gettext("Mounted")
                        } else {
                            gettext("Not mounted")
                        };
                        group.set_description(Some(&desc));

                        // Remote path row
                        let path_row = adw::ActionRow::new();
                        path_row.set_title(&gettext("Remote Path"));
                        path_row.set_subtitle(&share.source);
                        group.add(&path_row);

                        // Mount point row
                        let mount_row = adw::ActionRow::new();
                        mount_row.set_title(&gettext("Mount Point"));
                        mount_row.set_subtitle(&share.target);
                        group.add(&mount_row);

                        // Filesystem type row
                        let fs_type_row = adw::ActionRow::new();
                        fs_type_row.set_title(&gettext("Type"));
                        fs_type_row.set_subtitle(&share.fstype);
                        group.add(&fs_type_row);

                        // Options row (truncated if too long)
                        let options_text = if share.options.len() > 60 {
                            format!("{}...", &share.options[..60])
                        } else {
                            share.options.clone()
                        };
                        let options_row = adw::ActionRow::new();
                        options_row.set_title(&gettext("Options"));
                        options_row.set_subtitle(&options_text);
                        group.add(&options_row);

                        // Buttons row
                        let button_row = adw::ActionRow::new();
                        let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);

                        // Edit button (always visible)
                        let edit_button = gtk4::Button::with_label(&gettext("Edit"));
                        edit_button.set_valign(gtk4::Align::Center);

                        // Create RemoteSambaShareConfig from share data for editing
                        let remote_config = RemoteSambaShareConfig::new(
                            share.target.clone(),
                            share.source.clone(),
                            share.fstype.clone(),
                            // Extract credentials from options
                            share.options
                                .split(',')
                                .find(|opt| opt.contains("credentials="))
                                .and_then(|opt| opt.split('=').nth(1))
                                .unwrap_or("")
                                .to_string(),
                            // Extract uid from options
                            share.options
                                .split(',')
                                .find(|opt| opt.contains("uid="))
                                .and_then(|opt| opt.split('=').nth(1))
                                .unwrap_or("1000")
                                .to_string(),
                            // Extract gid from options
                            share.options
                                .split(',')
                                .find(|opt| opt.contains("gid="))
                                .and_then(|opt| opt.split('=').nth(1))
                                .unwrap_or("100")
                                .to_string(),
                        );

                        let window_for_edit = window.clone();
                        edit_button.connect_clicked(move |_| {
                            let edit_dialog = EditRemoteShareDialog::new(&remote_config);
                            edit_dialog.present(Some(&window_for_edit));
                        });

                        button_box.append(&edit_button);

                        if share.is_mounted {
                            // Unmount button
                            let unmount_button = gtk4::Button::with_label(&gettext("Unmount"));
                            unmount_button.set_valign(gtk4::Align::Center);
                            unmount_button.add_css_class("destructive-action");

                            let target = share.target.clone();
                            let toast_clone = toast_overlay.clone();
                            let window_clone = window.clone();
                            unmount_button.connect_clicked(move |button| {
                                button.set_sensitive(false);

                                let target_path = Path::new(&target).to_path_buf();
                                let toast = toast_clone.clone();
                                let btn = button.clone();

                                glib::spawn_future_local(async move {
                                    let result = gio::spawn_blocking(move || {
                                        unmount_share(&target_path)
                                    })
                                    .await;

                                    btn.set_sensitive(true);

                                    match result {
                                        Ok(Ok(())) => {
                                            let toast_msg =
                                                adw::Toast::new(&gettext("Share unmounted successfully"));
                                            toast.add_toast(toast_msg);
                                            // Note: Should refresh the list here
                                        }
                                        Ok(Err(e)) => {
                                            let toast_msg = adw::Toast::new(&format!(
                                                "{}: {}",
                                                gettext("Unmount failed"),
                                                e
                                            ));
                                            toast.add_toast(toast_msg);
                                        }
                                        Err(e) => {
                                            let toast_msg = adw::Toast::new(&format!(
                                                "{}: {:?}",
                                                gettext("Error"),
                                                e
                                            ));
                                            toast.add_toast(toast_msg);
                                        }
                                    }
                                });
                            });

                            button_box.append(&unmount_button);
                        } else {
                            // Mount button
                            let mount_button = gtk4::Button::with_label(&gettext("Mount"));
                            mount_button.set_valign(gtk4::Align::Center);
                            mount_button.add_css_class("suggested-action");

                            let source = share.source.clone();
                            let target = share.target.clone();
                            let toast_clone = toast_overlay.clone();
                            mount_button.connect_clicked(move |button| {
                                button.set_sensitive(false);

                                // TODO: Get credentials from user input dialog
                                // For now, show a message that manual mount via CLI is needed
                                let toast = adw::Toast::new(&gettext(
                                    "Mount requires credentials. Use 'sudo mount -t cifs ...' or nixos-rebuild.",
                                ));
                                toast_clone.add_toast(toast);

                                button.set_sensitive(true);

                                // Future implementation:
                                // 1. Show credentials dialog
                                // 2. Get username/password
                                // 3. Call mount_share()
                            });

                            button_box.append(&mount_button);
                        }

                        button_row.add_suffix(&button_box);
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
