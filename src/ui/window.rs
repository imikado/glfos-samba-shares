use crate::config::AppConfig;
use crate::ui::dialogs::{AddShareDialog, ListSharesDialog,RemoteListSharesDialog, WelcomeDialog,AddRemoteShareDialog};
use gettextrs::gettext;
use gtk4::prelude::*;
use gtk4::{gio, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

pub struct SambaShareManagerWindow {
    window: adw::ApplicationWindow,
    hardware_config: Rc<RefCell<String>>,
    config_file: PathBuf,
    must_save: Rc<RefCell<bool>>,
    rebuild_banner: adw::Banner,
    rebuild_error_banner: adw::Banner,
    toast_overlay: adw::ToastOverlay,
}

impl SambaShareManagerWindow {
    pub fn new(
        app: &adw::Application,
        hardware_config: Rc<RefCell<String>>,
        config_file: PathBuf,
        must_save: Rc<RefCell<bool>>,
        skip_welcome: bool,
    ) -> Rc<Self> {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(&format!("samba-share v{}", env!("CARGO_PKG_VERSION")))
            .default_width(800)
            .default_height(600)
            .icon_name("samba-share")
            .resizable(true)
            .build();

        // Ensure window can be maximized properly
        window.set_default_size(800, 600);

        // Create toolbar view for proper adwaita layout
        let toolbar_view = adw::ToolbarView::new();

        // Create header bar
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        // Create banners
        let rebuild_banner = adw::Banner::new(&gettext("Rebuilding NixOS configuration..."));
        rebuild_banner.set_revealed(false);

        let rebuild_error_banner = adw::Banner::new(&gettext("Failed to rebuild NixOS configuration"));
        rebuild_error_banner.set_revealed(false);
        rebuild_error_banner.add_css_class("error");

        toolbar_view.add_top_bar(&rebuild_banner);
        toolbar_view.add_top_bar(&rebuild_error_banner);

        // Create toast overlay for notifications
        let toast_overlay = adw::ToastOverlay::new();

        // Create scrolled window for content
        let scrolled = gtk4::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        // Create clamp for proper content width
        let clamp = adw::Clamp::new();
        clamp.set_maximum_size(600);
        clamp.set_tightening_threshold(400);

        // Main content box
        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 24);
        content_box.set_margin_top(24);
        content_box.set_margin_bottom(24);
        content_box.set_margin_start(12);
        content_box.set_margin_end(12);

        // ============ Header Section ============
        let header_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        header_box.set_halign(gtk4::Align::Center);
        header_box.set_margin_bottom(12);

        // App icon
        let icon = gtk4::Image::from_icon_name("folder-remote-symbolic");
        icon.set_pixel_size(64);
        icon.add_css_class("dim-label");
        header_box.append(&icon);

        // Title
        let title_label = gtk4::Label::new(Some(&gettext("Samba Share Manager")));
        title_label.add_css_class("title-1");
        header_box.append(&title_label);

        // Subtitle
        let subtitle_label = gtk4::Label::new(Some(&gettext("Configure local and remote network shares")));
        subtitle_label.add_css_class("dim-label");
        header_box.append(&subtitle_label);

        content_box.append(&header_box);

        // ============ Local Shares Section ============
        let local_group = adw::PreferencesGroup::new();
        local_group.set_title(&gettext("Local Shares"));
        local_group.set_description(Some(&gettext("Share folders from this computer on the network")));

        // List local shares row
        let list_local_row = adw::ActionRow::new();
        list_local_row.set_title(&gettext("Manage Local Shares"));
        list_local_row.set_subtitle(&gettext("View, edit, and delete existing shares"));
        list_local_row.set_activatable(true);
        list_local_row.add_prefix(&gtk4::Image::from_icon_name("folder-symbolic"));
        list_local_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
        local_group.add(&list_local_row);

        // Add local share row
        let add_local_row = adw::ActionRow::new();
        add_local_row.set_title(&gettext("Add New Share"));
        add_local_row.set_subtitle(&gettext("Create a new folder share"));
        add_local_row.set_activatable(true);
        add_local_row.add_prefix(&gtk4::Image::from_icon_name("list-add-symbolic"));
        add_local_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
        local_group.add(&add_local_row);

        content_box.append(&local_group);

        // ============ Remote Shares Section ============
        let remote_group = adw::PreferencesGroup::new();
        remote_group.set_title(&gettext("Remote Shares"));
        remote_group.set_description(Some(&gettext("Connect to shares on other computers")));

        // List remote shares row
        let list_remote_row = adw::ActionRow::new();
        list_remote_row.set_title(&gettext("Manage Remote Shares"));
        list_remote_row.set_subtitle(&gettext("View, edit, mount, and unmount remote shares"));
        list_remote_row.set_activatable(true);
        list_remote_row.add_prefix(&gtk4::Image::from_icon_name("network-server-symbolic"));
        list_remote_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
        remote_group.add(&list_remote_row);

        // Add remote share row
        let add_remote_row = adw::ActionRow::new();
        add_remote_row.set_title(&gettext("Add New Remote Share"));
        add_remote_row.set_subtitle(&gettext("Connect to a share on another computer"));
        add_remote_row.set_activatable(true);
        add_remote_row.add_prefix(&gtk4::Image::from_icon_name("list-add-symbolic"));
        add_remote_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
        remote_group.add(&add_remote_row);

        content_box.append(&remote_group);

        // ============ Info Section ============
        let info_group = adw::PreferencesGroup::new();

        let info_row = adw::ActionRow::new();
        info_row.set_title(&gettext("About NixOS Integration"));
        info_row.set_subtitle(&gettext("Changes are saved to /etc/nixos/customConfig/default.nix"));
        info_row.add_prefix(&gtk4::Image::from_icon_name("dialog-information-symbolic"));
        info_row.set_activatable(false);
        info_group.add(&info_row);

        content_box.append(&info_group);

        // Assemble the layout
        clamp.set_child(Some(&content_box));
        scrolled.set_child(Some(&clamp));
        toast_overlay.set_child(Some(&scrolled));
        toolbar_view.set_content(Some(&toast_overlay));

        // Create references for button handlers
        let list_shares_button = list_local_row.clone();
        let setup_share_button = add_local_row.clone();
        let remote_list_shares_button = list_remote_row.clone();
        let remote_setup_share_button = add_remote_row.clone();

        // Connect row activated signals
        // Local shares
        let window_clone_for_list = window.clone();
        list_shares_button.connect_activated(move |_| {
            let dialog = ListSharesDialog::new();
            dialog.present(Some(&window_clone_for_list));
        });

        let window_clone_for_setup = window.clone();
        setup_share_button.connect_activated(move |_| {
            let dialog = AddShareDialog::new();
            dialog.present(Some(&window_clone_for_setup));
        });

        // Remote shares
        let window_clone_for_remote_list = window.clone();
        remote_list_shares_button.connect_activated(move |_| {
            let dialog = RemoteListSharesDialog::new();
            dialog.present(Some(&window_clone_for_remote_list));
        });

        let window_clone_for_remote_setup = window.clone();
        remote_setup_share_button.connect_activated(move |_| {
            let dialog = AddRemoteShareDialog::new();
            dialog.present(Some(&window_clone_for_remote_setup));
        });

        window.set_content(Some(&toolbar_view));

        let window_rc = Rc::new(Self {
            window: window.clone(),
            hardware_config: hardware_config.clone(),
            config_file,
            must_save,
            rebuild_banner,
            rebuild_error_banner,
            toast_overlay: toast_overlay.clone(),
        });

        // Fix minimization bug with pkexec: force redraw when window is shown
        let content_box_clone = content_box.clone();
        window.connect_is_active_notify(move |_| {
            // Force queue a resize and redraw when window becomes active
            content_box_clone.queue_resize();
            content_box_clone.queue_draw();
        });

        // Show welcome dialog only if not skipping
        if !skip_welcome {
            let welcome = Rc::new(WelcomeDialog::new());
            let welcome_clone = welcome.clone();

            // Connect to the response signal to save preference if needed
            welcome.dialog().connect_response(None, move |_, _| {
                if welcome_clone.should_hide_next_time() {
                    let app_config = AppConfig::new();
                    app_config.set_hide_welcome(true);
                }
            });

            welcome.present(Some(&window));
        }

        window_rc
    }

    fn do_save_config(
        config_file: &PathBuf,
        hardware_config: &Rc<RefCell<String>>,
        rebuild_banner: &adw::Banner,
        rebuild_error_banner: &adw::Banner,
        must_save: &Rc<RefCell<bool>>,
        on_rebuild_complete: Option<Rc<dyn Fn()>>,
    ) {
        eprintln!("=== Beginning save ===");

        let config = hardware_config.borrow().clone();

        // For now, just write the config as-is
        // TODO: Add Samba share configuration generation
        if let Err(e) = fs::write(config_file, &config) {
            eprintln!("Error writing file: {}", e);
            rebuild_error_banner.set_revealed(true);
            return;
        }

        eprintln!("File written successfully");

        rebuild_error_banner.set_revealed(false);
        rebuild_banner.set_revealed(true);

        // Run nixos-rebuild in background
        let rebuild_banner = rebuild_banner.clone();
        let rebuild_error_banner = rebuild_error_banner.clone();
        let _must_save = must_save.clone();
        let hardware_config_for_reload = hardware_config.clone();
        let config_file_for_reload = config_file.clone();

        glib::spawn_future_local(async move {
            eprintln!("Launching nixos-rebuild switch...");
            let result = gio::spawn_blocking(|| {
                // Create a temporary wrapper script for rebuild
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let wrapper_path = format!("/tmp/samba_share_rebuild_{}.sh", timestamp);
                let status_file = format!("/tmp/samba_share_rebuild_{}.done", timestamp);

                let script_content = format!(
                    r#"#!/usr/bin/env bash

echo "======================================"
echo "  REBUILDING CONFIGURATION"
echo "======================================"
echo ""

# Preserve environment for sudo
sudo -E nixos-rebuild switch
EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "======================================"
    echo "  ✅ REBUILD COMPLETED SUCCESSFULLY"
    echo "======================================"

    # Signal completion
    touch {}
else
    echo ""
    echo "======================================"
    echo "  ❌ REBUILD FAILED"
    echo "======================================"
fi

echo ""
echo "Press Enter or close this window..."
read -t 300 || true
"#,
                    status_file
                );

                if let Err(e) = std::fs::write(&wrapper_path, script_content) {
                    eprintln!("Error: unable to write rebuild script: {}", e);
                    return (false, status_file.clone(), wrapper_path.clone());
                }

                if let Err(e) = Command::new("chmod").arg("+x").arg(&wrapper_path).status() {
                    eprintln!("Error chmod: {}", e);
                    let _ = std::fs::remove_file(&wrapper_path);
                    return (false, status_file.clone(), wrapper_path.clone());
                }

                // Try multiple terminals in order of preference
                let terminals: Vec<(&str, Vec<&str>)> = vec![
                    ("kgx", vec!["--", &wrapper_path]), // GNOME Console
                    ("gnome-terminal", vec!["--", &wrapper_path]),
                    ("konsole", vec!["-e", &wrapper_path]),
                    ("xfce4-terminal", vec!["-e", &wrapper_path]),
                    ("alacritty", vec!["-e", &wrapper_path]),
                    ("kitty", vec![&wrapper_path]),
                    ("xterm", vec!["-e", &wrapper_path]),
                ];

                for (term, args) in terminals {
                    eprintln!("Trying {}...", term);
                    if Command::new(term).args(&args).spawn().is_ok() {
                        eprintln!("Terminal {} opened successfully", term);
                        return (true, status_file, wrapper_path);
                    }
                }

                eprintln!("No terminal found to execute nixos-rebuild");
                let _ = std::fs::remove_file(&wrapper_path);
                (false, status_file, wrapper_path)
            })
            .await
            .unwrap_or((false, String::new(), String::new()));

            let (terminal_opened, status_file_path, script_path) = result;

            if !terminal_opened {
                rebuild_banner.set_revealed(false);
                rebuild_error_banner.set_revealed(true);
            } else {
                // Start watching for completion
                let rebuild_banner_watch = rebuild_banner.clone();
                let rebuild_error_banner_watch = rebuild_error_banner.clone();
                let hardware_config_watch = hardware_config_for_reload.clone();
                let on_rebuild_complete_watch = on_rebuild_complete.clone();
                let config_file_watch = config_file_for_reload.clone();
                let check_count = Rc::new(RefCell::new(0u32));

                glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
                    *check_count.borrow_mut() += 1;
                    let count = *check_count.borrow();

                    // Check if status file exists
                    if std::path::Path::new(&status_file_path).exists() {
                        eprintln!("Rebuild completed detected!");

                        // Reload hardware config from file (it was updated by the rebuild)
                        eprintln!("Reloading config from: {}", config_file_watch.display());
                        let updated_config = std::fs::read_to_string(&config_file_watch)
                            .unwrap_or_else(|e| {
                                eprintln!("Error reading config: {}", e);
                                hardware_config_watch.borrow().clone()
                            });

                        // Update the config in memory
                        *hardware_config_watch.borrow_mut() = updated_config.clone();
                        eprintln!("Config in memory updated");

                        // Call the refresh callback if provided
                        if let Some(ref callback) = on_rebuild_complete_watch {
                            eprintln!("Refreshing interface after rebuild");
                            callback();
                        }

                        // Hide banner
                        rebuild_banner_watch.set_revealed(false);

                        // Clean up
                        let _ = std::fs::remove_file(&status_file_path);
                        let _ = std::fs::remove_file(&script_path);

                        return glib::ControlFlow::Break;
                    }

                    // Stop after 10 minutes (300 checks * 2 seconds)
                    if count > 300 {
                        eprintln!("Rebuild watcher timeout");
                        rebuild_banner_watch.set_revealed(false);
                        let _ = std::fs::remove_file(&script_path);
                        return glib::ControlFlow::Break;
                    }

                    glib::ControlFlow::Continue
                });
            }
        });
    }

    pub fn save_config(&self) {
        let refresh_callback = Rc::new(move || {
            eprintln!("Refresh callback called");
        });

        Self::do_save_config(
            &self.config_file,
            &self.hardware_config,
            &self.rebuild_banner,
            &self.rebuild_error_banner,
            &self.must_save,
            Some(refresh_callback),
        );
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn gtk_window(&self) -> &adw::ApplicationWindow {
        &self.window
    }
}