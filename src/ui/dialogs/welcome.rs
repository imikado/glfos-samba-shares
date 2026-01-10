use gettextrs::gettext;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct WelcomeDialog {
    dialog: adw::MessageDialog,
    dont_show_again: Rc<RefCell<bool>>,
}

impl WelcomeDialog {
    pub fn new() -> Self {
        let dialog = adw::MessageDialog::new(
            None::<&gtk4::Window>,
            Some(&gettext("Welcome to Samba Share Manager")),
            Some(&gettext("This application helps you manage your Samba shares on NixOS")),
        );

        // Create checkbox for "Don't show again"
        let checkbox = gtk4::CheckButton::with_label(&gettext("Don't show this dialog again"));
        checkbox.set_margin_top(12);
        checkbox.set_margin_bottom(12);
        checkbox.set_margin_start(12);
        checkbox.set_margin_end(12);

        // Add checkbox to dialog's extra child
        dialog.set_extra_child(Some(&checkbox));

        dialog.add_response("continue", &gettext("Continue to Application"));
        dialog.set_response_appearance("continue", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("continue"));
        dialog.set_close_response("continue");

        let dont_show_again = Rc::new(RefCell::new(false));
        let dont_show_clone = dont_show_again.clone();

        // Update the preference when checkbox is toggled
        checkbox.connect_toggled(move |cb| {
            *dont_show_clone.borrow_mut() = cb.is_active();
        });

        Self {
            dialog,
            dont_show_again,
        }
    }

    pub fn present(&self, parent: Option<&impl IsA<gtk4::Widget>>) {
        if let Some(p) = parent {
            if let Some(window) = p.dynamic_cast_ref::<gtk4::Window>() {
                self.dialog.set_transient_for(Some(window));
            }
        }
        self.dialog.present();
    }

    pub fn should_hide_next_time(&self) -> bool {
        *self.dont_show_again.borrow()
    }

    pub fn dialog(&self) -> &adw::MessageDialog {
        &self.dialog
    }
}