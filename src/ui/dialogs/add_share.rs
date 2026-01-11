use crate::samba::share_config::{get_system_groups, get_system_users, SambaShareConfig};
use gettextrs::gettext;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

pub struct AddShareDialog {
    window: adw::Window,
    name_entry: adw::EntryRow,
    path_entry: adw::EntryRow,
    browse_button: gtk4::Button,
    browsable_switch: adw::SwitchRow,
    read_only_switch: adw::SwitchRow,
    guest_ok_switch: adw::SwitchRow,
    force_user_combo: adw::ComboRow,
    force_group_combo: adw::ComboRow,
    toast_overlay: adw::ToastOverlay,
}

impl AddShareDialog {
    pub fn new() -> Self {
        let window = adw::Window::new();
        window.set_title(Some(&gettext("Setup New Samba Share")));
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

        // Share Name
        let name_entry = adw::EntryRow::new();
        name_entry.set_title(&gettext("Share Name"));
        basic_group.add(&name_entry);

        // Path with browse button
        let path_entry = adw::EntryRow::new();
        path_entry.set_title(&gettext("Path"));

        let browse_button = gtk4::Button::with_label(&gettext("Browse..."));
        browse_button.set_valign(gtk4::Align::Center);
        path_entry.add_suffix(&browse_button);
        basic_group.add(&path_entry);

        preferences_page.add(&basic_group);

        // Permissions Group
        let permissions_group = adw::PreferencesGroup::new();
        permissions_group.set_title(&gettext("Permissions"));

        // Browsable switch
        let browsable_switch = adw::SwitchRow::new();
        browsable_switch.set_title(&gettext("Browsable"));
        browsable_switch.set_subtitle(&gettext("Share is visible in network browsing"));
        browsable_switch.set_active(true);
        permissions_group.add(&browsable_switch);

        // Read Only switch
        let read_only_switch = adw::SwitchRow::new();
        read_only_switch.set_title(&gettext("Read Only"));
        read_only_switch.set_subtitle(&gettext("Users can only read files"));
        read_only_switch.set_active(false);
        permissions_group.add(&read_only_switch);

        // Guest OK switch
        let guest_ok_switch = adw::SwitchRow::new();
        guest_ok_switch.set_title(&gettext("Guest OK"));
        guest_ok_switch.set_subtitle(&gettext("Allow guest access without password"));
        guest_ok_switch.set_active(false);
        permissions_group.add(&guest_ok_switch);

        preferences_page.add(&permissions_group);

        // User/Group Settings Group
        let user_group_group = adw::PreferencesGroup::new();
        user_group_group.set_title(&gettext("User &amp; Group Settings"));

        // Force User dropdown
        let force_user_combo = adw::ComboRow::new();
        force_user_combo.set_title(&gettext("Force User"));
        force_user_combo.set_subtitle(&gettext("Force all file operations as this user"));

        // Get system users
        let users = get_system_users();
        let user_list = gtk4::StringList::new(&users.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        force_user_combo.set_model(Some(&user_list));
        force_user_combo.set_selected(0);
        user_group_group.add(&force_user_combo);

        // Force Group dropdown
        let force_group_combo = adw::ComboRow::new();
        force_group_combo.set_title(&gettext("Force Group"));
        force_group_combo.set_subtitle(&gettext("Force all file operations as this group"));

        // Get system groups
        let groups = get_system_groups();
        let group_list = gtk4::StringList::new(&groups.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        force_group_combo.set_model(Some(&group_list));
        force_group_combo.set_selected(0);
        user_group_group.add(&force_group_combo);

        preferences_page.add(&user_group_group);

        toolbar_view.set_content(Some(&preferences_page));

        // Add action buttons in header
        let cancel_button = gtk4::Button::with_label(&gettext("Cancel"));
        header_bar.pack_start(&cancel_button);

        let add_button = gtk4::Button::with_label(&gettext("Add Share"));
        add_button.add_css_class("suggested-action");
        header_bar.pack_end(&add_button);

        // Wrap toolbar in toast overlay for error messages
        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&toolbar_view));

        window.set_content(Some(&toast_overlay));

        // Handle browse button
        let window_clone_for_browse = window.clone();
        let path_entry_clone = path_entry.clone();
        browse_button.connect_clicked(move |_| {
            let dialog = gtk4::FileDialog::new();
            dialog.set_title(&gettext("Select Folder"));

            let path_entry_clone2 = path_entry_clone.clone();
            dialog.select_folder(Some(&window_clone_for_browse), None::<&gtk4::gio::Cancellable>, move |result| {
                if let Ok(folder) = result {
                    if let Some(path) = folder.path() {
                        path_entry_clone2.set_text(&path.to_string_lossy());
                    }
                }
            });
        });

        // Handle cancel button
        let window_clone = window.clone();
        cancel_button.connect_clicked(move |_| {
            window_clone.close();
        });

        // Handle add button
        let window_clone2 = window.clone();
        let name_entry_clone = name_entry.clone();
        let path_entry_clone2 = path_entry.clone();
        let browsable_switch_clone = browsable_switch.clone();
        let read_only_switch_clone = read_only_switch.clone();
        let guest_ok_switch_clone = guest_ok_switch.clone();
        let force_user_combo_clone = force_user_combo.clone();
        let force_group_combo_clone = force_group_combo.clone();
        let toast_overlay_clone = toast_overlay.clone();

        add_button.connect_clicked(move |_| {
            let name = name_entry_clone.text();
            let path = path_entry_clone2.text();

            // Validate required fields
            if name.is_empty() {
                let toast = adw::Toast::new(&gettext("Share name is required"));
                toast_overlay_clone.add_toast(toast);
                return;
            }

            if path.is_empty() {
                let toast = adw::Toast::new(&gettext("Path is required"));
                toast_overlay_clone.add_toast(toast);
                return;
            }

            let browsable = browsable_switch_clone.is_active();
            let read_only = read_only_switch_clone.is_active();
            let guest_ok = guest_ok_switch_clone.is_active();

            let force_user = if let Some(model) = force_user_combo_clone.model() {
                if let Some(string_list) = model.dynamic_cast_ref::<gtk4::StringList>() {
                    string_list.string(force_user_combo_clone.selected())
                        .map(|s| s.to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let force_group = if let Some(model) = force_group_combo_clone.model() {
                if let Some(string_list) = model.dynamic_cast_ref::<gtk4::StringList>() {
                    string_list.string(force_group_combo_clone.selected())
                        .map(|s| s.to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            // Write configuration to NixOS
            let share_config = SambaShareConfig::new(
                name.to_string(),
                path.to_string(),
                browsable,
                read_only,
                guest_ok,
                force_user,
                force_group,
            );

            match share_config.write() {
                Ok(_) => {
                    eprintln!(
                        "Share added: name={}, path={}, browsable={}, read_only={}, guest_ok={}, force_user={}, force_group={}",
                        name, path, browsable, read_only, guest_ok, share_config.force_user, share_config.force_group
                    );
                    let toast = adw::Toast::new(&gettext("Share added successfully. Please rebuild NixOS to apply changes."));
                    toast_overlay_clone.add_toast(toast);
                    window_clone2.close();
                }
                Err(e) => {
                    eprintln!("Failed to add share: {}", e);
                    let error_msg = format!("{}: {}", gettext("Failed to add share"), e);
                    let toast = adw::Toast::new(&error_msg);
                    toast_overlay_clone.add_toast(toast);
                }
            }
        });

        Self {
            window,
            name_entry,
            path_entry,
            browse_button,
            browsable_switch,
            read_only_switch,
            guest_ok_switch,
            force_user_combo,
            force_group_combo,
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
