use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Button, FileChooserAction, FileChooserDialog,
    HeaderBar, ResponseType, ScrolledWindow, TextView,
};
use gtk4::glib::{clone};
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

fn main() {
    let app = Application::builder()
        .application_id("com.example.BasadoTextEditor")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(800)
        .default_height(600)
        .title("Basado Text Editor")
        .build();

    let header = HeaderBar::builder()
        .show_title_buttons(true)
        .build();

    let open_button = Button::with_label("Open");
    let save_as_button = Button::with_label("Save As");
    let save_button = Button::with_label("Save");

    header.pack_start(&open_button);
    header.pack_end(&save_as_button);
    header.pack_end(&save_button);

    window.set_titlebar(Some(&header));

    let text_view = TextView::new();
    let text_buffer = text_view.buffer().clone();
    let file_path = Rc::new(RefCell::new(None));

    let scrolled_window = ScrolledWindow::builder()
        .child(&text_view)
        .vexpand(true)
        .hexpand(true)
        .build();

    window.set_child(Some(&scrolled_window));

    // Open button logic
    open_button.connect_clicked(clone!(@strong window, @strong text_buffer, @strong file_path => move |_| {
        let dialog = FileChooserDialog::new(
            Some("Open File"),
            Some(&window),
            FileChooserAction::Open,
            &[("Open", ResponseType::Accept), ("Cancel", ResponseType::Cancel)],
        );

        dialog.connect_response(clone!(@strong text_buffer, @strong file_path => move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file().and_then(|f| f.path()) {
                    if let Ok(content) = std::fs::read_to_string(&file) {
                        text_buffer.set_text(&content);
                        *file_path.borrow_mut() = Some(file);
                    }
                }
            }
            dialog.close();
        }));

        dialog.show();
    }));

    // Save button logic
    save_button.connect_clicked(clone!(@strong window, @strong text_buffer, @strong file_path => move |_| {
        if let Some(ref path) = *file_path.borrow() {
            if let Ok(mut file) = File::create(path) {
                let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                let _ = file.write_all(text.as_bytes());
            }
        } else {
            let dialog = FileChooserDialog::new(
                Some("Save File"),
                Some(&window),
                FileChooserAction::Save,
                &[("Save", ResponseType::Accept), ("Cancel", ResponseType::Cancel)],
            );

            dialog.connect_response(clone!(@strong text_buffer, @strong file_path => move |dialog, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = dialog.file().and_then(|f| f.path()) {
                        if let Ok(mut f) = File::create(&file) {
                            let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                            let _ = f.write_all(text.as_bytes());
                            *file_path.borrow_mut() = Some(file);
                        }
                    }
                }
                dialog.close();
            }));

            dialog.show();
        }
    }));

    // Save As button logic
    save_as_button.connect_clicked(clone!(@strong window, @strong text_buffer, @strong file_path => move |_| {
        let dialog = FileChooserDialog::new(
            Some("Save File As"),
            Some(&window),
            FileChooserAction::Save,
            &[("Save As", ResponseType::Accept), ("Cancel", ResponseType::Cancel)],
        );

        dialog.connect_response(clone!(@strong text_buffer, @strong file_path => move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file().and_then(|f| f.path()) {
                    if let Ok(mut f) = File::create(&file) {
                        let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                        let _ = f.write_all(text.as_bytes());
                        *file_path.borrow_mut() = Some(file);
                    }
                }
            }
            dialog.close();
        }));

        dialog.show();
    }));

    window.show();
}
