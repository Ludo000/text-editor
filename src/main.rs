// Module declarations for the application components
mod ui;        // User interface components and layout
mod handlers;  // Event handlers and business logic
mod utils;     // Utility functions used across the application
mod syntax;    // Syntax highlighting functionality
mod settings;  // User settings and preferences

// GTK and standard library imports
use gtk4::prelude::*;   // GTK trait imports for widget functionality
use gtk4::{Application, ApplicationWindow, Label};  // Main GTK application classes
use gtk4::Box as GtkBox;  // Box container for layout
use gtk4::gio;          // GIO for menu and action support
use gtk4::glib;         // GLib for clone macro and other utilities
use std::rc::Rc;        // Reference counting for shared ownership
use std::cell::RefCell; // Interior mutability pattern
use std::collections::HashMap; // For mapping tab indices to file paths
use std::path::PathBuf;        // File system path representation
use std::io::Write;            // File writing capabilities
use clap::Parser;       // Command line argument parsing

/// Command line arguments for the Basado Text Editor
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File to open
    #[arg(help = "Path to the file to open")]
    file: Option<PathBuf>,
}

/// Application entry point - initializes the GTK application and runs the main loop
fn main() {
    // Check for help and version flags before GTK takes over
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
            Args::parse(); // This will show help and exit
            return;
        }
        if args.contains(&"--version".to_string()) || args.contains(&"-V".to_string()) {
            Args::parse(); // This will show version and exit
            return;
        }
    }
    
    // Parse command line arguments (mainly for non-GTK cases)
    let parsed_args = Args::try_parse().unwrap_or_else(|_| Args { file: None });
    
    // Debug output
    println!("Command line args parsed: {:?}", parsed_args);
    if let Some(ref file) = parsed_args.file {
        println!("File to open: {:?}", file);
    } else {
        println!("No file specified");
    }
    
    // Initialize user settings first
    settings::initialize_settings();
    
    // Create the main GTK application with a unique application ID
    // Set flags to handle file opening
    let app = Application::builder()
        .application_id("com.example.BasadoTextEditor")
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    
    // Force GTK to respect system dark mode settings
    app.connect_startup(|_| {
        if let Some(settings) = gtk4::Settings::default() {
            // Use our comprehensive dark mode detection function
            // This is more reliable than ad-hoc checks
            let prefer_dark = syntax::is_dark_mode_enabled();
            
            // Set dark mode preference
            settings.set_gtk_application_prefer_dark_theme(prefer_dark);
                    
            // Double check that the setting took effect
            if settings.is_gtk_application_prefer_dark_theme() != prefer_dark {
                println!("Warning: GTK dark mode setting didn't match our preference! Trying again...");
                settings.set_gtk_application_prefer_dark_theme(prefer_dark);
                settings.notify("gtk-application-prefer-dark-theme");
            }
        }
    });
    
    // Connect the activate signal to the build_ui function with file argument
    let args_for_activate = parsed_args.clone();
    app.connect_activate(move |app| {
        println!("activate signal called!");
        build_ui(app, args_for_activate.file.clone());
    });

    // Connect the open signal to handle file opening from command line
    app.connect_open(move |app, files, _hint| {
        println!("open signal called with {} files", files.len());
        if let Some(file) = files.first() {
            if let Some(path) = file.path() {
                println!("Opening file from command line: {:?}", path);
                build_ui(app, Some(path));
            } else {
                println!("File has no path, opening without file");
                build_ui(app, None);
            }
        } else {
            println!("No files provided to open signal");
            build_ui(app, None);
        }
    });
    
    // Add startup signal for debugging
    app.connect_startup(|_| {
        println!("startup signal called!");
    });
    
    // Start the GTK main loop
    println!("Starting app.run()");
    app.run();
}

/// Updates the style scheme of all editor buffers when the system theme changes
pub fn update_all_buffer_themes(window: &gtk4::ApplicationWindow) {
    println!("Beginning comprehensive theme update for all buffers...");

    // First, let's try a more comprehensive search for notebooks
    fn find_all_notebooks(widget: &gtk4::Widget) -> Vec<gtk4::Notebook> {
        let mut notebooks = Vec::new();
        
        // Check if this widget is a notebook
        if let Some(notebook) = widget.downcast_ref::<gtk4::Notebook>() {
            notebooks.push(notebook.clone());
        }
        
        // Recursively search children
        let mut child = widget.first_child();
        while let Some(current_child) = child {
            notebooks.extend(find_all_notebooks(&current_child));
            child = current_child.next_sibling();
        }
        
        notebooks
    }

    let notebooks = find_all_notebooks(window.upcast_ref::<gtk4::Widget>());
    println!("Found {} notebooks in the window", notebooks.len());

    for (notebook_idx, notebook) in notebooks.iter().enumerate() {
        let n_pages = notebook.n_pages();
        println!("Notebook {}: Updating {} pages...", notebook_idx, n_pages);
        
        // Iterate through all notebook pages
        for page_num in 0..n_pages {
            if let Some(page) = notebook.nth_page(Some(page_num)) {
                println!("Processing notebook {} page {}", notebook_idx, page_num);
                
                // Try to find any SourceView in this page (could be nested)
                fn find_source_views(widget: &gtk4::Widget) -> Vec<sourceview5::View> {
                    let mut views = Vec::new();
                    
                    if let Some(source_view) = widget.downcast_ref::<sourceview5::View>() {
                        views.push(source_view.clone());
                    }
                    
                    let mut child = widget.first_child();
                    while let Some(current_child) = child {
                        views.extend(find_source_views(&current_child));
                        child = current_child.next_sibling();
                    }
                    
                    views
                }
                
                let source_views = find_source_views(&page);
                println!("Found {} source views in page {}", source_views.len(), page_num);
                
                for (view_idx, source_view) in source_views.iter().enumerate() {
                    let buffer = source_view.buffer();
                    if let Some(source_buffer) = buffer.dynamic_cast_ref::<sourceview5::Buffer>() {
                        println!("Updating source buffer {} in page {}", view_idx, page_num);
                        syntax::update_buffer_style_scheme(source_buffer);
                        source_view.queue_draw();
                    }
                }
                
                // Force the page to redraw
                page.queue_draw();
            }
        }
        
        // Force the notebook to redraw
        notebook.queue_draw();
    }

    // Let's also print the current dark mode setting to help with debugging
    if let Some(settings) = gtk4::Settings::default() {
        let is_dark = settings.is_gtk_application_prefer_dark_theme();
        println!("Dark mode is now: {}", if is_dark { "enabled" } else { "disabled" });
        
        // If dark mode setting doesn't match our detection, try to fix it
        let detected_dark_mode = syntax::is_dark_mode_enabled();
        if detected_dark_mode != is_dark {
            println!("Warning: Dark mode setting ({}) doesn't match detected preference ({}), fixing...",
                     if is_dark { "enabled" } else { "disabled" },
                     if detected_dark_mode { "enabled" } else { "disabled" });
            settings.set_gtk_application_prefer_dark_theme(detected_dark_mode);
        }
    }

    // Force UI to update after a short delay
    let window_clone = window.clone();
    glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
        window_clone.queue_draw();
    });
}



/// Builds the user interface and sets up event handlers
fn build_ui(app: &Application, file_to_open: Option<PathBuf>) {
    // Debug output
    println!("build_ui called with file_to_open: {:?}", file_to_open);
    
    // Create the main application window
    let window = ui::create_window(app);
    
    // Create the header bar with action buttons
    let (header, new_button, open_button, save_main_button, save_menu_button, save_as_button, save_button, settings_button) = ui::create_header();

    // Create terminal notebook with tabs instead of single terminal
    let (terminal_notebook, add_terminal_button) = ui::create_terminal_notebook();
    let terminal_notebook_box = ui::create_terminal_notebook_box(&terminal_notebook, &add_terminal_button);
    
    // Set up theme settings based on system preferences
    if let Some(settings) = gtk4::Settings::default() {
        // Don't override the system preference - let GTK handle it naturally
        // This allows the app to respond to system theme changes automatically
        
        // Clone references to update editor views when theme changes
        let window_clone = window.clone();
        let terminal_notebook_clone = terminal_notebook.clone();
        
        // Connect to multiple theme-related signals to catch all possible theme changes
        let window_clone_2 = window_clone.clone();
        let terminal_notebook_clone_2 = terminal_notebook_clone.clone();
        let window_clone_3 = window_clone.clone();
        let terminal_notebook_clone_3 = terminal_notebook_clone.clone();
        
        // Primary signal for dark theme preference changes
        settings.connect_notify_local(
            Some("gtk-application-prefer-dark-theme"),
            move |_, _| {
                println!("Theme changed via gtk-application-prefer-dark-theme signal");
                syntax::sync_gtk_with_system_theme();
                update_all_buffer_themes(&window_clone);
                ui::update_all_terminal_themes(&terminal_notebook_clone);
            }
        );
        
        // Secondary signal for general theme name changes (catches more theme switches)
        settings.connect_notify_local(
            Some("gtk-theme-name"),
            move |_, _| {
                println!("Theme changed via gtk-theme-name signal");
                syntax::sync_gtk_with_system_theme();
                update_all_buffer_themes(&window_clone_2);
                ui::update_all_terminal_themes(&terminal_notebook_clone_2);
            }
        );
        
        // Monitor icon theme changes which often accompany theme switches
        settings.connect_notify_local(
            Some("gtk-icon-theme-name"),
            move |_, _| {
                println!("Icon theme changed - may indicate system theme change");
                syntax::sync_gtk_with_system_theme();
                update_all_buffer_themes(&window_clone_3);
                ui::update_all_terminal_themes(&terminal_notebook_clone_3);
            }
        );
        
        // Set up a GSettings monitor for GNOME/Ubuntu theme changes
        setup_gsettings_monitor(&window, &terminal_notebook);
    }

    // Initialize the text editor components
    // Returns multiple widgets and associated state for the editor UI
    let (
        _initial_scrolled_window, // Container for the first tab's TextView with scrolling capability
        _initial_text_view,       // The editable text view widget for the first tab
        initial_text_buffer,      // Buffer holding the text content for the first tab
        _initial_tab_file_path_rc,// Reference-counted path for the first tab's file
        error_label,              // Label for displaying error messages to the user
        picture,                  // Widget for displaying images when opening image files
        current_dir,              // Current working directory for file operations
        editor_notebook,          // Tabbed container for managing multiple open files
        _initial_tab_widget,      // Container for custom tab label components
        initial_tab_actual_label, // Text label showing the file name in the tab
        initial_tab_close_button  // Button for closing the tab
    ) = ui::create_text_view();
    
    // Debug theme detection at startup
    println!("=== Theme Detection at Startup ===");
    syntax::debug_theme_detection();
    
    // Ensure the initial buffer gets the correct theme based on dark mode setting
    if let Some(source_buffer) = initial_text_buffer.dynamic_cast_ref::<sourceview5::Buffer>() {
        syntax::update_buffer_style_scheme(source_buffer);
        println!("Applied initial theme to first tab buffer");
    }

    // Create a mapping between notebook tab indexes and their corresponding file paths
    // This allows tracking which file is open in each tab
    let file_path_manager = Rc::new(RefCell::new(HashMap::<u32, PathBuf>::new()));
    
    // Track the file path of the currently active tab
    let active_tab_path = Rc::new(RefCell::new(None::<PathBuf>));

    // Set up window close event handler to check for unsaved changes
    let window_clone_for_close = window.clone();
    let editor_notebook_clone_for_close = editor_notebook.clone();
    let file_path_manager_clone_for_close = file_path_manager.clone();
    
    window.connect_close_request(move |_| {
        // Check if any tabs have unsaved changes (indicated by '*' in tab labels)
        let notebook = &editor_notebook_clone_for_close;
        let mut unsaved_files = Vec::new();
        
        // Iterate through all tabs to check for unsaved changes
        let num_pages = notebook.n_pages();
        for page_num in 0..num_pages {
            if let Some(page_widget) = notebook.nth_page(Some(page_num)) {
                if let Some(tab_label_widget) = notebook.tab_label(&page_widget) {
                    if let Some(tab_box) = tab_label_widget.downcast_ref::<gtk4::Box>() {
                        if let Some(label) = tab_box.first_child().and_then(|w| w.downcast::<Label>().ok()) {
                            if label.text().starts_with('*') {
                                // Found an unsaved file - get its name
                                let filename = file_path_manager_clone_for_close.borrow()
                                    .get(&page_num)
                                    .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
                                    .unwrap_or_else(|| "Untitled".to_string());
                                unsaved_files.push(filename);
                            }
                        }
                    }
                }
            }
        }
        
        // If there are unsaved files, show confirmation dialog
        if !unsaved_files.is_empty() {
            let message = if unsaved_files.len() == 1 {
                format!("You have unsaved changes in {}.\n\nAre you sure you want to close the application without saving?", unsaved_files[0])
            } else {
                format!("You have unsaved changes in {} files:\n• {}\n\nAre you sure you want to close the application without saving?", 
                        unsaved_files.len(), 
                        unsaved_files.join("\n• "))
            };
            
            let dialog = gtk4::MessageDialog::new(
                Some(&window_clone_for_close),
                gtk4::DialogFlags::MODAL | gtk4::DialogFlags::DESTROY_WITH_PARENT,
                gtk4::MessageType::Warning,
                gtk4::ButtonsType::None,
                &message
            );
            
            dialog.add_buttons(&[
                ("Cancel", gtk4::ResponseType::Cancel),
                ("Close Anyway", gtk4::ResponseType::Yes),
            ]);
            
            dialog.set_default_response(gtk4::ResponseType::Cancel);
            
            let window_clone_for_dialog = window_clone_for_close.clone();
            
            dialog.connect_response(move |d, response| {
                d.close();
                match response {
                    gtk4::ResponseType::Yes => {
                        // User chose "Close Anyway" - allow the close to proceed
                        // We need to temporarily disconnect the close handler to avoid recursion
                        window_clone_for_dialog.destroy();
                    }
                    _ => {
                        // User chose "Cancel" or closed dialog - close was already stopped
                        // Do nothing, the close request was already stopped
                    }
                }
            });
            
            dialog.present();
            return glib::Propagation::Stop; // Prevent window from closing until user decides
        }
        
        // No unsaved changes, allow normal close
        glib::Propagation::Proceed
    });

    // Initialize the file manager panel components
    let (file_list_box, file_list_scrolled_window) =
        ui::create_file_manager_panel();
        
    // Assemble the file manager panel from its components
    let file_manager_panel =
        ui::create_file_manager_panel_container(file_list_scrolled_window);

    // Create the path bar with navigation buttons and path segments
    let (path_bar, path_box, up_button, _refresh_button, terminal_button) = ui::create_path_bar();
    
    // Set up keyboard shortcuts for common operations (including Ctrl+L for path editing)
    utils::setup_keyboard_shortcuts(
        &window, 
        &save_button, 
        &open_button, 
        &new_button, 
        &save_as_button, 
        Some(&editor_notebook),
        Some(&path_box),
        Some(&current_dir),
        Some(&file_list_box),
        Some(&active_tab_path)
    );

    // Create the main container that will hold the path bar and paned content
    let main_container = GtkBox::new(gtk4::Orientation::Vertical, 0);
    main_container.append(&path_bar);

    // Define GIO actions for save operations to be used by the menu
    let save_action = gio::SimpleAction::new("save", None);
    let save_as_action = gio::SimpleAction::new("save-as", None);
    
    // Prepare button references for the action handlers
    let save_button_clone = save_button.clone();
    let save_as_button_clone = save_as_button.clone();
    
    // Connect the save action to trigger the save button's click event
    // This allows menu items to reuse existing save functionality
    let save_button_clone_for_action = save_button_clone.clone();
    save_action.connect_activate(move |_, _| {
        save_button_clone_for_action.emit_clicked();
    });
    
    // Connect the save-as action to trigger the save-as button's click event
    let save_as_button_clone_for_action = save_as_button_clone.clone();
    save_as_action.connect_activate(move |_, _| {
        save_as_button_clone_for_action.emit_clicked();
    });
    
    // Register the actions with the application window
    // This makes them available to be triggered by menu items
    window.add_action(&save_action);
    window.add_action(&save_as_action);
    
    // Set up direct save functionality for the main save button
    // Instead of circular references between buttons, implement the save logic directly here
    
    // Clone references needed for the save operation
    let editor_notebook_clone = editor_notebook.clone();
    let _active_tab_path_clone = active_tab_path.clone(); // Unused but kept for potential future use
    let file_path_manager_clone = file_path_manager.clone();
    let _window_clone = window.clone(); // Unused but kept for potential future use
    let _file_list_box_clone = file_list_box.clone(); // Unused but kept for potential future use
    let _current_dir_clone = current_dir.clone(); // Unused but kept for potential future use
    let save_as_button_clone = save_as_button.clone();
    
    save_main_button.connect_clicked(move |_| {
        // Implementation of the save functionality
        if let Some((_active_text_view, active_buffer)) = handlers::get_active_text_view_and_buffer(&editor_notebook_clone) {
            // Get the current tab index
            let current_page_num_opt = editor_notebook_clone.current_page();
            if current_page_num_opt.is_none() { return; }
            let current_page_num = current_page_num_opt.unwrap();

            // Look up the file path associated with this tab
            let path_to_save_opt = file_path_manager_clone.borrow().get(&current_page_num).cloned();

            if let Some(path_to_save) = path_to_save_opt {
                // Check if this is a supported file type for saving
                let mime_type = mime_guess::from_path(&path_to_save).first_or_octet_stream();
                if utils::is_allowed_mime_type(&mime_type) {
                    // Attempt to save the file
                    if let Ok(mut file) = std::fs::File::create(&path_to_save) {
                        // Extract the text content from the buffer
                        let text = active_buffer.text(&active_buffer.start_iter(), &active_buffer.end_iter(), false);
                        
                        // Write the content to the file and update UI if successful
                        if file.write_all(text.as_bytes()).is_ok() {
                            // Update tab label to remove the modified indicator (*)
                            handlers::update_tab_label_after_save(&editor_notebook_clone, current_page_num, Some(&path_to_save.file_name().unwrap_or_default().to_string_lossy()), false);
                        }
                    }
                }
            } else {
                // If no path is associated with this tab (new unsaved file),
                // redirect to the Save As functionality
                save_as_button_clone.emit_clicked();
            }
        }
    });

    // Set up modification tracking for the initial tab
    // This adds a "*" indicator to the tab label when content has been modified
    let initial_tab_actual_label_clone = initial_tab_actual_label.clone();
    let initial_buffer_clone_for_dirty_track = initial_text_buffer.clone();
    
    // Connect to the buffer's changed signal to detect modifications
    initial_text_buffer.connect_changed(move |_buffer| {
        // Get the current text content from the buffer
        let text_content = initial_buffer_clone_for_dirty_track.text(
            &initial_buffer_clone_for_dirty_track.start_iter(),
            &initial_buffer_clone_for_dirty_track.end_iter(),
            false
        );
        
        // Get the current tab label text
        let label_text = initial_tab_actual_label_clone.text();
        
        // If the file was previously unmodified and now has content, mark as modified
        if label_text == "Untitled" && !text_content.is_empty() {
            initial_tab_actual_label_clone.set_text("*Untitled");
        } 
        // If the file was previously modified but now is empty, remove the modified indicator
        else if label_text.starts_with('*') && text_content.is_empty() && label_text == "*Untitled" {
            initial_tab_actual_label_clone.set_text("Untitled");
        }
    });

    // Prepare dependencies needed for creating a new tab
    // This structure holds references to all components needed when creating or managing tabs
    // It's particularly used when closing tabs to ensure a new one is created if the last tab is closed
    let deps_for_new_tab_creation = handlers::NewTabDependencies {
        editor_notebook: editor_notebook.clone(),      // The main tabbed container
        active_tab_path: active_tab_path.clone(),      // Currently active file path
        file_path_manager: file_path_manager.clone(),  // Tab-to-path mapping
        window: window.clone(),                        // Main application window
        file_list_box: file_list_box.clone(),          // File browser list
        current_dir: current_dir.clone(),              // Current directory for file operations
        save_button: save_button.clone(),              // Save button reference
        save_as_button: save_as_button.clone(),        // Save As button reference
        _save_menu_button: Some(save_menu_button.clone()), // Split button menu component (currently unused)
    };

    // Set up the close button handler for the initial tab
    // Clone all necessary references for the closure
    let initial_tab_close_button_clone = initial_tab_close_button.clone();
    let editor_notebook_clone_for_initial_close = editor_notebook.clone();
    let window_clone_for_initial_close = window.clone();
    let file_path_manager_clone_for_initial_close = file_path_manager.clone();
    let active_tab_path_clone_for_initial_close = active_tab_path.clone();
    let current_dir_clone_for_initial_close = current_dir.clone();
    let file_list_box_clone_for_initial_close = file_list_box.clone();

    // Connect to the close button's clicked signal
    initial_tab_close_button_clone.connect_clicked(move |_| {
        // Verify the notebook still has pages before attempting to close one
        if editor_notebook_clone_for_initial_close.n_pages() > 0 { 
            // Check if the first tab (usually the initial one) exists
            if let Some(_page_widget) = editor_notebook_clone_for_initial_close.nth_page(Some(0)) {
                // Handle the tab close request with proper cleanup and potential new tab creation
                handlers::handle_close_tab_request(
                    &editor_notebook_clone_for_initial_close,
                    0, // Tab index 0 (first tab)
                    &window_clone_for_initial_close,
                    &file_path_manager_clone_for_initial_close,
                    &active_tab_path_clone_for_initial_close,
                    &current_dir_clone_for_initial_close,
                    &file_list_box_clone_for_initial_close,
                    Some(deps_for_new_tab_creation.clone()) // Dependencies for creating a new tab if needed
                );
            }
        }
    });

    // Track the current file selection source for click-outside detection
    let current_selection_source = Rc::new(RefCell::new(utils::FileSelectionSource::TabSwitch));
    
    // Initialize the path box with clickable buttons for each directory segment
    utils::update_path_buttons(&path_box, &current_dir, &file_list_box, &active_tab_path);
    
    // Create the main paned layout that contains:
    // - The file manager sidebar on the left
    // - The editor notebook and terminal in a vertical split on the right
    let paned_content = ui::create_paned(&file_manager_panel, &editor_notebook, &terminal_notebook_box);
    
    // Add click-outside detection for file manager to switch from DirectClick to TabSwitch styling
    // This allows the file manager to revert to subtle highlighting when focus is lost
    let click_controller = gtk4::GestureClick::new();
    let file_list_box_clone_for_click_outside = file_list_box.clone();
    let file_manager_panel_clone = file_manager_panel.clone();
    let current_dir_clone_for_click_outside = current_dir.clone();
    let active_tab_path_clone_for_click_outside = active_tab_path.clone();
    let current_selection_source_clone = current_selection_source.clone();
    
    click_controller.connect_pressed(move |_gesture, _n_press, x, y| {
        println!("Click detected at coordinates: ({}, {})", x, y);
        
        // Check if the current selection is DirectClick (only then do we need to switch)
        if *current_selection_source_clone.borrow() == utils::FileSelectionSource::DirectClick {
            println!("Current selection is DirectClick, checking if click is outside file manager");
            
            // Check if the click was outside the file manager panel bounds
            let file_manager_allocation = file_manager_panel_clone.allocation();
            let fm_x = file_manager_allocation.x() as f64;
            let fm_y = file_manager_allocation.y() as f64;
            let fm_width = file_manager_allocation.width() as f64;
            let fm_height = file_manager_allocation.height() as f64;
            
            println!("File manager bounds: x={}, y={}, width={}, height={}", fm_x, fm_y, fm_width, fm_height);
            
            let clicked_outside_file_manager = x < fm_x || 
                y < fm_y ||
                x > (fm_x + fm_width) || 
                y > (fm_y + fm_height);
            
            if clicked_outside_file_manager {
                println!("Click outside file manager detected! Switching from DirectClick to TabSwitch styling");
                
                // Update selection source to TabSwitch
                *current_selection_source_clone.borrow_mut() = utils::FileSelectionSource::TabSwitch;
                
                // Update file list to use TabSwitch styling instead of DirectClick
                utils::update_file_list(
                    &file_list_box_clone_for_click_outside,
                    &current_dir_clone_for_click_outside.borrow(),
                    &active_tab_path_clone_for_click_outside.borrow(),
                    utils::FileSelectionSource::TabSwitch
                );
            } else {
                println!("Click was inside file manager bounds");
            }
        } else {
            println!("Current selection is not DirectClick, ignoring click");
        }
    });
    
    // Add the click controller to the main window to capture all clicks
    window.add_controller(click_controller);

    // Add the main paned content
    main_container.append(&paned_content);

    // Set the custom header bar as the window's titlebar
    window.set_titlebar(Some(&header));

    // Initialize the file browser panel with the current directory contents
    // Initially there's no active file selection since we start with an empty "Untitled" tab
    utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path.borrow(), utils::FileSelectionSource::TabSwitch);
    
    // Set up the save menu button visibility for the default text plain content type
    // This is appropriate for the initial empty "Untitled" document
    utils::update_save_menu_button_visibility(&save_menu_button, Some(mime_guess::mime::TEXT_PLAIN_UTF_8));
    
    // Set the main container (with paned layout and status bar) as the window's content
    window.set_child(Some(&main_container));

    // Set up the tab switching handler to update UI state when changing tabs
    // Clone all required references for use in the closure
    let file_path_manager_clone_for_switch = file_path_manager.clone();
    let active_tab_path_clone_for_switch = active_tab_path.clone();
    let file_list_box_clone_for_switch = file_list_box.clone();
    let current_dir_clone_for_switch = current_dir.clone();
    let save_button_clone_for_switch = save_button.clone();
    let save_as_button_clone_for_switch = save_as_button.clone();
    let save_menu_button_clone_for_switch = save_menu_button.clone();
    let path_box_clone_for_switch = path_box.clone();

    // Connect to the notebook's switch-page signal
    editor_notebook.connect_switch_page(move |notebook, _page, page_num| {
        // Retrieve the file path associated with the newly selected tab
        let new_active_path = { 
            // Use a separate scope to limit the borrow duration
            file_path_manager_clone_for_switch.borrow().get(&page_num).cloned()
        };

        // Update the active tab path reference
        *active_tab_path_clone_for_switch.borrow_mut() = new_active_path.clone();

        // If the focused tab has a file, update current directory to match the file's directory
        if let Some(file_path) = &new_active_path {
            if let Some(parent_dir) = file_path.parent() {
                let parent_path = parent_dir.to_path_buf();
                // Only update if the directory is different from current
                if *current_dir_clone_for_switch.borrow() != parent_path {
                    *current_dir_clone_for_switch.borrow_mut() = parent_path;
                    
                    // Update the file list to show the new directory
                    utils::update_file_list(&file_list_box_clone_for_switch, &current_dir_clone_for_switch.borrow(), &new_active_path, utils::FileSelectionSource::TabSwitch);
                    
                    // Update the path buttons to reflect the new current directory
                    utils::update_path_buttons(&path_box_clone_for_switch, &current_dir_clone_for_switch, &file_list_box_clone_for_switch, &active_tab_path_clone_for_switch);
                    
                    return; // Exit early since we've already updated the file list
                }
            }
        }

        // Update file list highlighting to show the current file (only if directory didn't change)
        let current_dir_path_clone = current_dir_clone_for_switch.borrow().clone(); 
        utils::update_file_list(&file_list_box_clone_for_switch, &current_dir_path_clone, &new_active_path, utils::FileSelectionSource::TabSwitch);

        // Determine the MIME type from the file path
        let mime_type = new_active_path.as_ref()
            .map(|p| mime_guess::from_path(p).first_or_octet_stream())
            .unwrap_or(mime_guess::mime::TEXT_PLAIN_UTF_8); // Default to plain text for unsaved files
        
        // Check if the current tab has a text view (editable content) or is an image tab
        if let Some((_, _)) = handlers::get_text_view_and_buffer_for_page(notebook, page_num) {
            // This is a text tab - enable save functionality
            utils::update_save_buttons_visibility(
                &save_button_clone_for_switch, 
                &save_as_button_clone_for_switch, 
                Some(mime_type.clone())
            );
            
            utils::update_save_menu_button_visibility(
                &save_menu_button_clone_for_switch, 
                Some(mime_type)
            );
        } else if let Some(page) = notebook.nth_page(Some(page_num)) {
            // Handle cases where the tab contains non-text content (e.g., image)
            if let Some(scrolled_window) = page.downcast_ref::<gtk4::ScrolledWindow>() {
                if let Some(child) = scrolled_window.child() {
                    // Check if the child is a Picture widget (image content)
                    if child.is::<gtk4::Picture>() || mime_type.type_() == "image" {
                        // This is an image tab - disable save functionality
                        utils::update_save_buttons_visibility(
                            &save_button_clone_for_switch, 
                            &save_as_button_clone_for_switch, 
                            Some(mime_guess::mime::IMAGE_PNG) // Use any image MIME type to trigger hiding
                        );
                        
                        utils::update_save_menu_button_visibility(
                            &save_menu_button_clone_for_switch, 
                            Some(mime_guess::mime::IMAGE_PNG)
                        );
                    } else {
                        // Other non-text content, use default behavior based on MIME type
                        utils::update_save_buttons_visibility(
                            &save_button_clone_for_switch, 
                            &save_as_button_clone_for_switch, 
                            Some(mime_type.clone())
                        );
                        
                        utils::update_save_menu_button_visibility(
                            &save_menu_button_clone_for_switch, 
                            Some(mime_type)
                        );
                    }
                }
            }
        } else {
            // Fallback: disable save functionality if we can't determine content type
            utils::update_save_buttons_visibility(
                &save_button_clone_for_switch, 
                &save_as_button_clone_for_switch, 
                None
            );
            utils::update_save_menu_button_visibility(
                &save_menu_button_clone_for_switch, 
                None
            );
        }
    });

    // Set up all button event handlers and their associated functionality
    handlers::setup_button_handlers(
        &new_button,           // New file button
        &open_button,          // Open file button
        &save_button,          // Save button (hidden, used programmatically)
        &save_as_button,       // Save As button
        &initial_text_buffer,  // Text buffer for the initial tab
        &file_path_manager,    // Mapping of tabs to file paths
        &active_tab_path,      // Currently active file path
        &window,               // Main application window
        &current_dir,          // Current working directory
        &file_list_box,        // File browser list box
        &editor_notebook,      // Tabbed notebook for editor
        &error_label,          // Label for displaying errors
        &picture,              // Widget for displaying images
        &up_button,            // Navigation button for parent directory
        &file_list_box,        // File list box (duplicate param for historical reasons)
        Some(&save_menu_button), // Split button menu component
        Some(&path_box),        // Path box for the status bar with clickable segments
        &current_selection_source, // Track selection source for click-outside detection
    );

    // Set up the terminal button handler to open a new terminal in the current directory
    let terminal_notebook_clone_for_terminal_button = terminal_notebook.clone();
    let current_dir_clone_for_terminal_button = current_dir.clone();
    terminal_button.connect_clicked(move |_| {
        // Add a new terminal tab in the current directory
        ui::add_terminal_tab(&terminal_notebook_clone_for_terminal_button, Some(current_dir_clone_for_terminal_button.borrow().clone()));
    });

    // Handle file opening from command line arguments
    println!("Checking file_to_open: {:?}", file_to_open);
    if let Some(ref file_path) = file_to_open {
        println!("Processing file argument: {:?}", file_path);
        // Check if the file exists and is readable
        if file_path.exists() {
            if file_path.is_file() {
                // Close any empty untitled tabs before opening the file
                handlers::close_empty_untitled_tabs(&editor_notebook, &file_path_manager);
                
                let mime_type = mime_guess::from_path(&file_path).first_or_octet_stream();
                
                if utils::is_allowed_mime_type(&mime_type) {
                    // Try to read the file content
                    match std::fs::read_to_string(&file_path) {
                        Ok(content) => {
                            // Open the file in a new tab
                            handlers::open_or_focus_tab(
                                &editor_notebook,
                                &file_path,
                                &content,
                                &active_tab_path,
                                &file_path_manager,
                                &save_button,
                                &save_as_button,
                                &mime_type,
                                &window,
                                &file_list_box,
                                &current_dir,
                                Some(&save_menu_button),
                            );
                            
                            // Update current directory to the file's parent directory
                            if let Some(parent) = file_path.parent() {
                                *current_dir.borrow_mut() = parent.to_path_buf();
                                utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path.borrow(), utils::FileSelectionSource::TabSwitch);
                                utils::update_path_buttons(&path_box, &current_dir, &file_list_box, &active_tab_path);
                            }
                            
                            println!("Successfully opened file: {:?}", file_path);
                        }
                        Err(e) => {
                            eprintln!("Error reading file {:?}: {}", file_path, e);
                            // Could show an error dialog here in the future
                        }
                    }
                } else if mime_type.type_() == "image" {
                    // Handle image files
                    handlers::open_or_focus_tab(
                        &editor_notebook,
                        &file_path,
                        "", // Empty content for images
                        &active_tab_path,
                        &file_path_manager,
                        &save_button,
                        &save_as_button,
                        &mime_type,
                        &window,
                        &file_list_box,
                        &current_dir,
                        Some(&save_menu_button),
                    );
                    
                    // Update current directory to the file's parent directory
                    if let Some(parent) = file_path.parent() {
                        *current_dir.borrow_mut() = parent.to_path_buf();
                        utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path.borrow(), utils::FileSelectionSource::TabSwitch);
                        utils::update_path_buttons(&path_box, &current_dir, &file_list_box, &active_tab_path);
                    }
                    
                    println!("Successfully opened image file: {:?}", file_path);
                } else {
                    // Handle unsupported file types by opening them with empty content
                    handlers::open_or_focus_tab(
                        &editor_notebook,
                        &file_path,
                        "", // Empty content for unsupported files
                        &active_tab_path,
                        &file_path_manager,
                        &save_button,
                        &save_as_button,
                        &mime_type,
                        &window,
                        &file_list_box,
                        &current_dir,
                        Some(&save_menu_button),
                    );
                    
                    // Update current directory to the file's parent directory
                    if let Some(parent) = file_path.parent() {
                        *current_dir.borrow_mut() = parent.to_path_buf();
                        utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path.borrow(), utils::FileSelectionSource::TabSwitch);
                        utils::update_path_buttons(&path_box, &current_dir, &file_list_box, &active_tab_path);
                    }
                    
                    println!("Opened unsupported file type: {:?}", file_path);
                }
            } else {
                eprintln!("Error: {:?} is not a file", file_path);
            }
        } else {
            eprintln!("Error: File {:?} does not exist", file_path);
        }
    }

    // Show the main window to display the application
    window.show();

    // Set up the settings button handler
    let window_clone_for_settings = window.clone();
    settings_button.connect_clicked(move |_| {
        // Create and show the settings dialog
        let dialog = ui::create_settings_dialog(&window_clone_for_settings);
        
        // When the dialog is closed, update all buffer themes
        let window_ref = window_clone_for_settings.clone();
        dialog.connect_close(move |_| {
            // Apply the new theme settings to all buffers
            update_all_buffer_themes(&window_ref);
        });
        
        dialog.show();
    });
}

/// Sets up a GSettings monitor to detect Ubuntu/GNOME theme changes
/// This provides better integration with system theme switching on Ubuntu
fn setup_gsettings_monitor(window: &ApplicationWindow, terminal_notebook: &gtk4::Notebook) {
    use gio::prelude::*;
    
    let window_clone = window.clone();
    let terminal_notebook_clone = terminal_notebook.clone();
    
    // Monitor the GNOME color-scheme setting which is the primary way Ubuntu switches themes
    match std::panic::catch_unwind(|| gio::Settings::new("org.gnome.desktop.interface")) {
        Ok(settings) => {
        let window_clone_2 = window_clone.clone();
        let terminal_notebook_clone_2 = terminal_notebook_clone.clone();
        
        // Monitor color-scheme changes (prefer-dark, prefer-light, default)
        settings.connect_changed(Some("color-scheme"), move |_, _| {
            println!("System color-scheme changed via GSettings");
            // Small delay to ensure the change has propagated
            let window_clone_inner = window_clone.clone();
            let terminal_notebook_clone_inner = terminal_notebook_clone.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                update_all_buffer_themes(&window_clone_inner);
                ui::update_all_terminal_themes(&terminal_notebook_clone_inner);
            });
        });
        
        // Also monitor gtk-theme changes for additional coverage
        settings.connect_changed(Some("gtk-theme"), move |_, _| {
            println!("GTK theme changed via GSettings");
            let window_clone_inner = window_clone_2.clone();
            let terminal_notebook_clone_inner = terminal_notebook_clone_2.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                update_all_buffer_themes(&window_clone_inner);
                ui::update_all_terminal_themes(&terminal_notebook_clone_inner);
            });
        });
        
        println!("GSettings monitor set up for org.gnome.desktop.interface");
        },
        Err(_) => {
            println!("Could not set up GSettings monitor - org.gnome.desktop.interface not available");
        }
    }
}
