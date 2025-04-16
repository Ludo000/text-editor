use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, TextView, FileChooserAction, FileChooserDialog, Button, HeaderBar};
use gio::SimpleAction;
use glib::clone;
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Write};
use std::rc::Rc;

fn build_ui(app: &Application) {
    let window = ApplicationWindow::new(app);
    window.set_title("Basado Text Editor");
    window.set_default_size(800, 600);

    let header_bar = HeaderBar::new();
    header_bar.set_title(Some("Basado Text Editor"));
    header_bar.set_show_close_button(true);
    window.set_titlebar(Some(&header_bar));

    let open_button = Button::with_label("Open");
    header_bar.pack_start(&open_button);

    let save_button = Button::with_label("Save");
    header_bar.pack_start(&save_button);

    let save_as_button = Button::with_label("Save As...");
    header_bar.pack_start(&save_as_button);

    let text_view = TextView::new();
    let text_buffer = text_view.buffer().expect("Could not get TextBuffer");

    // Shared state to store the file path
    let file_path = Rc::new(RefCell::new(None));

    window.add(&text_view);
    window.show_all();

    // Connect the save action to a callback
    let save_action = SimpleAction::new("save", None);
    let file_path_clone = Rc::clone(&file_path);
    save_action.connect_activate(clone!(@strong text_buffer, @strong window, @strong file_path_clone => move |_, _| {
        println!("Save action activated"); // Debugging line
        save_file(&window, &text_buffer, &file_path_clone);
    }));
    app.add_action(&save_action);

    // Connect the save as action to a callback
    let save_as_action = SimpleAction::new("save_as", None);
    let file_path_clone = Rc::clone(&file_path);
    save_as_action.connect_activate(clone!(@strong text_buffer, @strong window, @strong file_path_clone => move |_, _| {
        println!("Save As action activated"); // Debugging line
        save_file_as(&window, &text_buffer, &file_path_clone);
    }));
    app.add_action(&save_as_action);

    // Connect the open action to a callback
    let open_action = SimpleAction::new("open", None);
    let file_path_clone = Rc::clone(&file_path);
    open_action.connect_activate(clone!(@strong text_buffer, @strong window, @strong file_path_clone => move |_, _| {
        println!("Open action activated"); // Debugging line
        open_file(&window, &text_buffer, &file_path_clone);
    }));
    app.add_action(&open_action);

    // Connect the save button to the save action
    let file_path_clone = Rc::clone(&file_path);
    save_button.connect_clicked(clone!(@strong window, @strong text_buffer, @strong file_path_clone => move |_| {
        println!("Save button clicked"); // Debugging line
        save_file(&window, &text_buffer, &file_path_clone);
    }));

    // Connect the save as button to the save as action
    let file_path_clone = Rc::clone(&file_path);
    save_as_button.connect_clicked(clone!(@strong window, @strong text_buffer, @strong file_path_clone => move |_| {
        println!("Save As button clicked"); // Debugging line
        save_file_as(&window, &text_buffer, &file_path_clone);
    }));

    // Connect the open button to the open action
    let file_path_clone = Rc::clone(&file_path);
    open_button.connect_clicked(clone!(@strong window, @strong text_buffer, @strong file_path_clone => move |_| {
        println!("Open button clicked"); // Debugging line
        open_file(&window, &text_buffer, &file_path_clone);
    }));

    // Add keyboard shortcuts for saving (Ctrl+S), saving as (Ctrl+Shift+S), and opening (Ctrl+O)
    app.set_accels_for_action("app.save", &["<Primary>s"]);
    app.set_accels_for_action("app.save_as", &["<Primary><Shift>s"]);
    app.set_accels_for_action("app.open", &["<Primary>o"]);
}

fn save_file(window: &ApplicationWindow, text_buffer: &gtk::TextBuffer, file_path: &Rc<RefCell<Option<String>>>) {
    let mut file_path_borrow = file_path.borrow_mut();
    let filename = if let Some(ref path) = *file_path_borrow {
        path.clone()
    } else {
        let dialog = FileChooserDialog::new(
            Some("Save File"),
            Some(window),
            FileChooserAction::Save,
        );

        dialog.add_button("Cancel", gtk::ResponseType::Cancel);
        dialog.add_button("Save", gtk::ResponseType::Accept);

        let filename = if dialog.run() == gtk::ResponseType::Accept {
            dialog.filename()
        } else {
            None
        };

        dialog.close();

        let filename = if let Some(filename) = filename {
            filename
        } else {
            eprintln!("Save operation cancelled.");
            return; // Return early if the operation is cancelled
        };

        *file_path_borrow = Some(filename.to_string_lossy().into_owned());
        filename.to_string_lossy().into_owned()
    };

    let start = text_buffer.start_iter();
    let end = text_buffer.end_iter();
    let text = text_buffer.text(&start, &end, false).expect("Could not get text");

    if let Err(e) = File::create(&filename).and_then(|mut file| file.write_all(text.as_bytes())) {
        eprintln!("Failed to save file: {}", e);
    }
}

fn save_file_as(window: &ApplicationWindow, text_buffer: &gtk::TextBuffer, file_path: &Rc<RefCell<Option<String>>>) {
    let dialog = FileChooserDialog::new(
        Some("Save File As"),
        Some(window),
        FileChooserAction::Save,
    );

    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button("Save", gtk::ResponseType::Accept);

    let filename = if dialog.run() == gtk::ResponseType::Accept {
        dialog.filename()
    } else {
        None
    };

    dialog.close();

    let filename = if let Some(filename) = filename {
        filename
    } else {
        eprintln!("Save As operation cancelled.");
        return; // Return early if the operation is cancelled
    };

    let mut file_path_borrow = file_path.borrow_mut();
    *file_path_borrow = Some(filename.to_string_lossy().into_owned());

    let start = text_buffer.start_iter();
    let end = text_buffer.end_iter();
    let text = text_buffer.text(&start, &end, false).expect("Could not get text");

    if let Err(e) = File::create(&filename).and_then(|mut file| file.write_all(text.as_bytes())) {
        eprintln!("Failed to save file: {}", e);
    }
}

fn open_file(window: &ApplicationWindow, text_buffer: &gtk::TextBuffer, file_path: &Rc<RefCell<Option<String>>>) {
    let dialog = FileChooserDialog::new(
        Some("Open File"),
        Some(window),
        FileChooserAction::Open,
    );

    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button("Open", gtk::ResponseType::Accept);

    if dialog.run() == gtk::ResponseType::Accept {
        if let Some(filename) = dialog.filename() {
            let mut file = match File::open(&filename) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Failed to open file: {}", e);
                    return;
                }
            };

            let mut contents = String::new();
            if let Err(e) = file.read_to_string(&mut contents) {
                eprintln!("Failed to read file: {}", e);
                return;
            }

            text_buffer.set_text(&contents);

            // Update the file path
            let mut file_path_borrow = file_path.borrow_mut();
            *file_path_borrow = Some(filename.to_string_lossy().into_owned());
        }
    }

    dialog.close();
}

fn main() {
    let app = Application::new(
        Some("com.example.BasadoTextEditor"),
        Default::default(),
    );

    app.connect_activate(|app| {
        build_ui(app);
    });

    app.run();
}
