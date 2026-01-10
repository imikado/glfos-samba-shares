use crate::config::AppConfig;
use crate::ui::window::SambaShareManagerWindow;
use gtk4::prelude::*;
use gtk4::{glib, gio};
use libadwaita as adw;
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

pub struct SambaShareManagerApp {
    app: adw::Application,
    #[allow(dead_code)]
    hardware_config_file: PathBuf,
    #[allow(dead_code)]
    hardware_config: Rc<RefCell<String>>,
    #[allow(dead_code)]
    must_save: Rc<RefCell<bool>>,
    #[allow(dead_code)]
    windows: Rc<RefCell<Vec<adw::ApplicationWindow>>>,
}

impl SambaShareManagerApp {
    pub fn new() -> Self {
        let app = adw::Application::builder()
            .application_id("org.dupot.sambasharemanager")
            .build();

        glib::set_application_name("samba-share");
        glib::set_prgname(Some("samba-share"));

        let hardware_config_file = PathBuf::from("/etc/nixos/customConfig/default.nix");
        let hardware_config = Rc::new(RefCell::new(String::new()));
        let must_save = Rc::new(RefCell::new(false));
        let windows: Rc<RefCell<Vec<adw::ApplicationWindow>>> = Rc::new(RefCell::new(Vec::new()));

        // Configure theme to follow system (simple approach)
        let style_manager = adw::StyleManager::default();
        style_manager.set_color_scheme(adw::ColorScheme::Default);

        let app_instance = Self {
            app: app.clone(),
            hardware_config_file: hardware_config_file.clone(),
            hardware_config: hardware_config.clone(),
            must_save: must_save.clone(),
            windows: windows.clone(),
        };

        // Setup activation
        let hardware_config_clone = hardware_config.clone();
        let config_file_clone = hardware_config_file.clone();
        let must_save_clone = must_save.clone();
        let windows_clone = windows.clone();

        app.connect_activate(move |app| {
            Self::on_activate(
                app,
                &config_file_clone,
                &hardware_config_clone,
                &must_save_clone,
                &windows_clone,
            );
        });

        app_instance
    }

    fn on_activate(
        app: &adw::Application,
        config_file: &PathBuf,
        hardware_config: &Rc<RefCell<String>>,
        must_save: &Rc<RefCell<bool>>,
        windows: &Rc<RefCell<Vec<adw::ApplicationWindow>>>,
    ) {
        // Load hardware configuration
        if let Ok(config) = fs::read_to_string(config_file) {
            *hardware_config.borrow_mut() = config;
        } else {
            eprintln!("Failed to read hardware configuration file");
            return;
        }

        // Check if we should show welcome dialog
        let app_config = AppConfig::new();
        let skip_welcome = !app_config.should_show_welcome();

        let window = SambaShareManagerWindow::new(
            app,
            hardware_config.clone(),
            config_file.clone(),
            must_save.clone(),
            skip_welcome,
        );

        // Store window reference for theme updates
        windows.borrow_mut().push(window.gtk_window().clone());

        window.present();
    }

    pub fn run(&self) -> i32 {
        self.app.run().into()
    }
}