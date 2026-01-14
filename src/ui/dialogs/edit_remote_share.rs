use crate::samba::remote_share_config::RemoteSambaShareConfig;
use gettextrs::gettext;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

pub struct EditRemoteShareDialog {
    window: adw::Window,
    original_name: String,
}

impl EditRemoteShareDialog {
    pub fn new(share: &RemoteSambaShareConfig) -> Self {
        let window = adw::Window::new();
        window.set_title(Some(&gettext("Edit Remote Samba Share")));
        window.set_default_size(500, 600);
        window.set_modal(true);

        // Create toolbar header
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        // Create preferences page for the form
        let preferences_page = adw::PreferencesPage::new();

        // Basic Information Group
        let basic_group = adw::PreferencesGroup::new();
        basic_group.set_title(&gettext("Basic Information"));

        // Mount Point (path where it will be mounted locally)
        let mount_point_entry = adw::EntryRow::new();
        mount_point_entry.set_title(&gettext("Mount Point"));
        mount_point_entry.set_text(&share.name);
        mount_point_entry.set_tooltip_text(Some(&gettext("Local directory where the remote share will be mounted (e.g., /media/share)")));
        basic_group.add(&mount_point_entry);

        // Remote Path (SMB share path)
        let remote_path_entry = adw::EntryRow::new();
        remote_path_entry.set_title(&gettext("Remote Path"));
        remote_path_entry.set_text(&share.remote_path);
        remote_path_entry.set_tooltip_text(Some(&gettext("SMB share path (e.g., //server/share)")));
        basic_group.add(&remote_path_entry);

        // Credentials File Path
        let credentials_entry = adw::EntryRow::new();
        credentials_entry.set_title(&gettext("Credentials File"));
        credentials_entry.set_text(&share.option_credentials);
        credentials_entry.set_tooltip_text(Some(&gettext("Path to file containing username and password")));

        let browse_button = gtk4::Button::with_label(&gettext("Browse..."));
        browse_button.set_valign(gtk4::Align::Center);
        credentials_entry.add_suffix(&browse_button);
        basic_group.add(&credentials_entry);

        preferences_page.add(&basic_group);

        // Mount Options Group
        let options_group = adw::PreferencesGroup::new();
        options_group.set_title(&gettext("Mount Options"));

        // UID Entry
        let uid_entry = adw::EntryRow::new();
        uid_entry.set_title(&gettext("User ID (uid)"));
        uid_entry.set_text(&share.force_user);
        uid_entry.set_tooltip_text(Some(&gettext("The user ID that will own the mounted files")));
        options_group.add(&uid_entry);

        // GID Entry
        let gid_entry = adw::EntryRow::new();
        gid_entry.set_title(&gettext("Group ID (gid)"));
        gid_entry.set_text(&share.force_group);
        gid_entry.set_tooltip_text(Some(&gettext("The group ID that will own the mounted files")));
        options_group.add(&gid_entry);

        preferences_page.add(&options_group);

        // Additional Options Group
        let advanced_group = adw::PreferencesGroup::new();
        advanced_group.set_title(&gettext("Additional Options"));
        advanced_group.set_description(Some(&gettext(
            "These options are automatically included in the configuration"
        )));

        // Auto-mount switch
        let automount_switch = adw::SwitchRow::new();
        automount_switch.set_title(&gettext("Auto-mount"));
        automount_switch.set_subtitle(&gettext("Automatically mount on system startup"));
        automount_switch.set_active(true); // Default enabled
        advanced_group.add(&automount_switch);

        // No auto switch (mount on access)
        let noauto_switch = adw::SwitchRow::new();
        noauto_switch.set_title(&gettext("Mount on access"));
        noauto_switch.set_subtitle(&gettext("Only mount when accessed (noauto)"));
        noauto_switch.set_active(true); // Default enabled
        advanced_group.add(&noauto_switch);

        preferences_page.add(&advanced_group);

        // Information banner
        let info_group = adw::PreferencesGroup::new();
        let info_banner = adw::Banner::new(&gettext(
            "Changes will be written to your NixOS configuration. Run 'sudo nixos-rebuild switch' to apply them."
        ));
        info_banner.set_revealed(true);

        let banner_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        banner_box.append(&info_banner);
        info_group.add(&banner_box);
        preferences_page.add(&info_group);

        toolbar_view.set_content(Some(&preferences_page));

        // Add action buttons in header
        let cancel_button = gtk4::Button::with_label(&gettext("Cancel"));
        header_bar.pack_start(&cancel_button);

        let save_button = gtk4::Button::with_label(&gettext("Save Changes"));
        save_button.add_css_class("suggested-action");
        header_bar.pack_end(&save_button);

        // Wrap toolbar in toast overlay for error messages
        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&toolbar_view));

        window.set_content(Some(&toast_overlay));

        // Store original name for updating
        let original_name = share.name.clone();

        // Handle browse button for credentials file
        let window_clone_for_browse = window.clone();
        let credentials_entry_clone = credentials_entry.clone();
        browse_button.connect_clicked(move |_| {
            let dialog = gtk4::FileDialog::new();
            dialog.set_title(&gettext("Select Credentials File"));

            let credentials_entry_clone2 = credentials_entry_clone.clone();
            dialog.open(Some(&window_clone_for_browse), None::<&gtk4::gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        credentials_entry_clone2.set_text(&path.to_string_lossy());
                    }
                }
            });
        });

        // Handle cancel button
        let window_clone = window.clone();
        cancel_button.connect_clicked(move |_| {
            window_clone.close();
        });

        // Handle save button
        let window_clone2 = window.clone();
        let mount_point_entry_clone = mount_point_entry.clone();
        let remote_path_entry_clone = remote_path_entry.clone();
        let credentials_entry_clone = credentials_entry.clone();
        let uid_entry_clone = uid_entry.clone();
        let gid_entry_clone = gid_entry.clone();
        let toast_overlay_clone = toast_overlay.clone();
        let original_name_clone = original_name.clone();

        save_button.connect_clicked(move |_| {
            let mount_point = mount_point_entry_clone.text();
            let remote_path = remote_path_entry_clone.text();
            let credentials = credentials_entry_clone.text();
            let uid = uid_entry_clone.text();
            let gid = gid_entry_clone.text();

            // Validate required fields
            if mount_point.is_empty() {
                let toast = adw::Toast::new(&gettext("Mount point is required"));
                toast_overlay_clone.add_toast(toast);
                return;
            }

            if remote_path.is_empty() {
                let toast = adw::Toast::new(&gettext("Remote path is required"));
                toast_overlay_clone.add_toast(toast);
                return;
            }

            // Validate mount point format (should start with /)
            if !mount_point.starts_with('/') {
                let toast = adw::Toast::new(&gettext("Mount point must be an absolute path (start with /)"));
                toast_overlay_clone.add_toast(toast);
                return;
            }

            // Validate remote path format (should be //server/share)
            if !remote_path.starts_with("//") {
                let toast = adw::Toast::new(&gettext("Remote path must start with // (e.g., //server/share)"));
                toast_overlay_clone.add_toast(toast);
                return;
            }

            // Validate UID is numeric
            if !uid.is_empty() && uid.parse::<u32>().is_err() {
                let toast = adw::Toast::new(&gettext("User ID must be a number"));
                toast_overlay_clone.add_toast(toast);
                return;
            }

            // Validate GID is numeric
            if !gid.is_empty() && gid.parse::<u32>().is_err() {
                let toast = adw::Toast::new(&gettext("Group ID must be a number"));
                toast_overlay_clone.add_toast(toast);
                return;
            }

            // Update configuration in NixOS
            let updated_share = RemoteSambaShareConfig::new(
                mount_point.to_string(),
                remote_path.to_string(),
                "cifs".to_string(),
                credentials.to_string(),
                uid.to_string(),
                gid.to_string(),
            );

            match updated_share.update(&original_name_clone) {
                Ok(_) => {
                    eprintln!(
                        "Remote share updated: mount_point={}, remote_path={}, credentials={}, uid={}, gid={}",
                        mount_point, remote_path, credentials, uid, gid
                    );
                    let toast = adw::Toast::new(&gettext("Share updated successfully. Run 'sudo nixos-rebuild switch' to apply changes."));
                    toast_overlay_clone.add_toast(toast);
                    window_clone2.close();
                }
                Err(e) => {
                    eprintln!("Failed to update remote share: {}", e);
                    let error_msg = format!("{}: {}", gettext("Failed to update share"), e);
                    let toast = adw::Toast::new(&error_msg);
                    toast_overlay_clone.add_toast(toast);
                }
            }
        });

        Self {
            window,
            original_name,
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
}
