use std::error::Error;
use std::fs;
use std::io;
use std::io::prelude::*;

use gtk::prelude::*;
use magic_crypt::MagicCryptTrait;
use passwords::analyzer;
use passwords::scorer;

use crate::item_actions;
use crate::models;

pub fn launch(application: &gtk::Application, builder: &gtk::Builder) -> Result<(), Box<dyn Error>> {
    let main_window: gtk::Window = builder.object("main_window").unwrap();
    let main_window_item_title_tree_view: gtk::TreeView = builder.object("main_window_item_title_tree_view").unwrap();

    let item_store = create_item_store()?;

    connect_items(builder, &item_store, &main_window_item_title_tree_view)?;
    connect_menu_items(builder, &main_window, &item_store, &main_window_item_title_tree_view)?;
    connect_about_dialog(builder)?;
    connect_change_master_key_dialog(builder)?;
    connect_generate_password_dialog(&builder)?;

    main_window.set_application(Some(application));

    main_window.connect_delete_event(glib::clone!(@weak main_window => @default-return Inhibit(false), move |_, _| {
        main_window.close();
        Inhibit(false)
    }));

    main_window.show_all();
    Ok(())
}

fn create_item_store() -> Result<gtk::ListStore, Box<dyn Error>> {
    let store = gtk::ListStore::new(&[glib::Type::STRING]);
    let items = item_actions::find_all(None).expect("failed to get Items");
    for item in items.iter() {
        debug!("item: {:?}", item);
        let value = glib::value::Value::from(&item.title);
        store.set_value(&store.append(), 0u32, &value);
    }
    Ok(store)
}

fn connect_items(builder: &gtk::Builder, store: &gtk::ListStore, item_title_tree_view: &gtk::TreeView) -> Result<(), Box<dyn Error>> {
    let item_content_text_view: gtk::TextView = builder.object("main_window_item_content_text_view").unwrap();
    let item_title_search_entry: gtk::SearchEntry = builder.object("main_window_item_title_search_entry").unwrap();

    item_title_tree_view.set_model(Some(store));
    item_title_tree_view.set_search_entry(Some(&item_title_search_entry));
    let item_title_tree_view_renderer = gtk::CellRendererTextBuilder::new().editable(true).build();
    item_title_tree_view_renderer.connect_edited(glib::clone!(@strong item_title_tree_view, @strong store => move |_renderer, _path, new_title| {
        tree_view_cell_renderer_edited(new_title, &item_title_tree_view, &store);
    }));
    let column = gtk::TreeViewColumnBuilder::new().title("Title").sort_column_id(0i32).build();
    column.pack_start(&item_title_tree_view_renderer, true);
    column.add_attribute(&item_title_tree_view_renderer, "text", 0i32);
    item_title_tree_view.append_column(&column);

    // remove popup for item title treeview
    let remove_menu_item: gtk::MenuItem = gtk::MenuItemBuilder::new().label("Remove").build();
    remove_menu_item.connect_activate(
        glib::clone!(@strong store, @weak item_title_tree_view, @strong item_content_text_view => move |_menu_item| {
            remove_menu_item_action(&store, &item_title_tree_view, &item_content_text_view);
        }),
    );
    let popup_menu: gtk::Menu = gtk::MenuBuilder::new().child(&remove_menu_item).build();
    item_title_tree_view.connect_button_press_event(move |_tree_view, event| {
        if event.event_type() == gdk::EventType::ButtonPress && event.button() == 3 {
            debug!("event: {:?}", event);
            popup_menu.popup_easy(event.button(), event.time());
            popup_menu.show_all();
        }
        gtk::Inhibit(false)
    });

    let tree_view_selection = item_title_tree_view.selection();
    tree_view_selection.connect_changed(glib::clone!(@weak item_content_text_view => move |tree_selection| {
        tree_view_selection_changed(tree_selection, &item_content_text_view);
    }));

    item_content_text_view.connect_key_release_event(glib::clone!(@weak item_title_tree_view => @default-return Inhibit(false), move |text_view, _| {
        text_view_key_press_event_action(&item_title_tree_view, text_view);
        Inhibit(false)
    }));
    Ok(())
}

fn connect_about_dialog(builder: &gtk::Builder) -> Result<(), Box<dyn Error>> {
    let dialog: gtk::AboutDialog = builder.object("about_dialog").unwrap();
    let menu_item: gtk::MenuItem = builder.object("about_menu_item").unwrap();

    menu_item.connect_activate(glib::clone!(@weak dialog => move |_| {
        dialog.show();
        dialog.run();
        dialog.hide();
    }));

    Ok(())
}

fn connect_change_master_key_dialog(builder: &gtk::Builder) -> Result<(), Box<dyn Error>> {
    let dialog: gtk::Dialog = builder.object("change_master_key_dialog").unwrap();
    let menu_item: gtk::MenuItem = builder.object("change_master_key_menu_item").unwrap();
    let current_key_entry: gtk::Entry = builder.object("change_master_key_dialog_current_key_entry").unwrap();
    let current_key_quality_score_label: gtk::Label = builder.object("change_master_key_dialog_current_key_quality_score_label").unwrap();
    let new_key_entry: gtk::Entry = builder.object("change_master_key_dialog_new_key_entry").unwrap();
    let new_key_quality_score_label: gtk::Label = builder.object("change_master_key_dialog_new_key_quality_score_label").unwrap();
    let ok_button: gtk::Button = builder.object("change_master_key_dialog_ok_button").unwrap();
    let error_dialog: gtk::MessageDialog = builder.object("error_dialog").unwrap();

    current_key_entry.connect_key_release_event(glib::clone!(@weak current_key_quality_score_label => @default-return Inhibit(false), move | entry, _ | {
        let key = entry.buffer().text();
        let score = scorer::score(&analyzer::analyze(&key));
        current_key_quality_score_label.set_label(format!("{}/100", score as i32).as_str());
        Inhibit(false)
    }));

    new_key_entry.connect_key_release_event(glib::clone!(@weak new_key_quality_score_label => @default-return Inhibit(false), move | entry, _ | {
        let key = entry.buffer().text();
        let score = scorer::score(&analyzer::analyze(&key));
        new_key_quality_score_label.set_label(format!("{}/100", score as i32).as_str());
        Inhibit(false)
    }));

    menu_item.connect_activate(glib::clone!(@weak dialog => move |_| {
        dialog.show_all();
    }));

    ok_button.connect_clicked(glib::clone!(@weak dialog, @weak new_key_entry, @strong error_dialog => move |_| {
        let new_master_key_text = new_key_entry.buffer().text();
        let new_master_key_score = scorer::score(&analyzer::analyze(&new_master_key_text));
        if new_master_key_score < 40_f64 {
            error_dialog.set_text("Your key scored < 40...you can do better".into());
            error_dialog.run();
            error_dialog.close();
        } else {
            let mut all_items = item_actions::find_all(None).expect("failed to get items from db");
            let new_magic_crypt = new_magic_crypt!(new_key_entry.buffer().text(), 256);
            let mut current_magic_crypt = crate::APP_CORE.magic_crypt.lock().unwrap();
            let old_magic_crypt = current_magic_crypt.as_ref().expect("failed to get magic_crypt");
            for item in all_items.iter_mut() {
                let contents = item
                    .decrypt_contents(&old_magic_crypt)
                    .expect("failed to decrypt item contents using current key");
                item.contents = Some(new_magic_crypt.encrypt_str_to_base64(contents));
                item_actions::update(&item).expect("failed to update item contents with new key");
            }
            *current_magic_crypt = Some(new_magic_crypt.clone());
            dialog.hide();
        }
    }));

    let tmp_dialog = dialog.clone();
    dialog.connect_close(glib::clone!(@weak current_key_entry, @weak new_key_entry => move |_| {
        current_key_entry.set_text("");
        new_key_entry.set_text("");
        tmp_dialog.hide();
    }));

    let cancel_button: gtk::Button = builder.object("change_master_key_dialog_cancel_button").unwrap();
    cancel_button.connect_clicked(glib::clone!(@weak dialog, @weak current_key_entry, @weak new_key_entry => move |_| {
        current_key_entry.set_text("");
        new_key_entry.set_text("");
        dialog.hide();
    }));

    Ok(())
}

fn connect_generate_password_dialog(builder: &gtk::Builder) -> Result<(), Box<dyn Error>> {
    let dialog: gtk::Dialog = builder.object("generate_password_dialog").unwrap();
    dialog.hide_on_delete();
    dialog.connect_delete_event(|dialog, _| {
        dialog.hide();
        Inhibit(true)
    });
    let tree_view: gtk::TreeView = builder.object("generate_password_dialog_password_tree_view").unwrap();
    let include_numbers_checkbox: gtk::CheckButton = builder.object("generate_password_dialog_include_numbers_checkbox").unwrap();
    let include_uppercase_checkbox: gtk::CheckButton = builder.object("generate_password_dialog_include_uppercase_checkbox").unwrap();
    let include_symbols_checkbox: gtk::CheckButton = builder.object("generate_password_dialog_include_symbols_checkbox").unwrap();
    let length_combobox: gtk::ComboBox = builder.object("generate_password_dialog_length_combobox").unwrap();
    let count_combobox: gtk::ComboBox = builder.object("generate_password_dialog_count_combobox").unwrap();

    let store = gtk::ListStore::new(&[glib::Type::STRING, glib::Type::STRING]);

    tree_view.set_model(Some(&store));

    let password_renderer = gtk::CellRendererTextBuilder::new().editable(true).build();
    let password_column = gtk::TreeViewColumnBuilder::new().title("Password").sort_column_id(0i32).clickable(true).build();
    password_column.pack_start(&password_renderer, true);
    password_column.add_attribute(&password_renderer, "text", 0i32);
    tree_view.append_column(&password_column);

    let password_quality_renderer = gtk::CellRendererTextBuilder::new().build();
    let password_quality_column = gtk::TreeViewColumnBuilder::new()
        .title("Quality")
        .sort_column_id(1i32)
        .fixed_width(20)
        .expand(false)
        .resizable(false)
        .sizing(gtk::TreeViewColumnSizing::Fixed)
        .build();
    password_quality_column.pack_start(&password_quality_renderer, true);
    password_quality_column.add_attribute(&password_quality_renderer, "text", 1i32);
    tree_view.append_column(&password_quality_column);

    let menu_item: gtk::MenuItem = builder.object("generate_password_menu_item").unwrap();
    menu_item.connect_activate(glib::clone!(@weak dialog => move |_| {
        dialog.show();
    }));

    let refresh_button: gtk::Button = builder.object("generate_password_dialog_refresh_button").unwrap();
    refresh_button.connect_clicked(glib::clone!(@weak include_numbers_checkbox, @weak include_uppercase_checkbox, @weak include_symbols_checkbox, @weak length_combobox, @weak count_combobox, @weak store => move |_| {
        generate_password_dialog_refresh_action(&include_numbers_checkbox, &include_uppercase_checkbox, &include_symbols_checkbox, &length_combobox, &count_combobox, &store);
    }));

    let cancel_button: gtk::Button = builder.object("generate_password_dialog_cancel_button").unwrap();
    cancel_button.connect_clicked(glib::clone!(@weak dialog => move |_| {
        dialog.hide();
    }));

    Ok(())
}

fn connect_menu_items(builder: &gtk::Builder, main_window: &gtk::Window, store: &gtk::ListStore, item_title_tree_view: &gtk::TreeView) -> Result<(), Box<dyn Error>> {
    let new_menu_item: gtk::MenuItem = builder.object("new_menu_item").unwrap();
    new_menu_item.connect_activate(glib::clone!(@strong store, @strong item_title_tree_view => move |_| {
        new_menu_item_action(&store, &item_title_tree_view)
    }));

    let import_menu_item: gtk::MenuItem = builder.object("import_menu_item").unwrap();
    import_menu_item.connect_activate(glib::clone!(@weak main_window, @weak store, @weak item_title_tree_view => move |_| {
        import_menu_item_action(&main_window, &store, &item_title_tree_view);
    }));

    let export_menu_item: gtk::MenuItem = builder.object("export_menu_item").unwrap();
    export_menu_item.connect_activate(glib::clone!(@weak main_window => move |_| {
        export_menu_item_action();
    }));

    let quit_menu_item: gtk::MenuItem = builder.object("quit_menu_item").unwrap();
    quit_menu_item.connect_activate(glib::clone!(@weak main_window => move |_| {
        main_window.close();
    }));
    Ok(())
}

fn generate_password_dialog_refresh_action(
    include_numbers_checkbox: &gtk::CheckButton,
    include_uppercase_checkbox: &gtk::CheckButton,
    include_symbols_checkbox: &gtk::CheckButton,
    length_combobox: &gtk::ComboBox,
    count_combobox: &gtk::ComboBox,
    store: &gtk::ListStore,
) {
    let generator = passwords::PasswordGenerator::new()
        .spaces(false)
        .exclude_similar_characters(true)
        .strict(true)
        .lowercase_letters(true)
        .numbers(include_numbers_checkbox.is_active())
        .symbols(include_symbols_checkbox.is_active())
        .uppercase_letters(include_uppercase_checkbox.is_active())
        .length(length_combobox.active_id().unwrap().parse::<usize>().unwrap());
    let passwords = generator
        .generate(count_combobox.active_id().unwrap().parse::<usize>().unwrap())
        .expect("Couldn't generate passwords");
    store.clear();
    for password in passwords {
        let score = scorer::score(&analyzer::analyze(&password));
        store.insert_with_values(None, &[(0, &password), (1, &format!("{}/100", score as i32).as_str())]);
    }
}

fn new_menu_item_action(store: &gtk::ListStore, tree_view: &gtk::TreeView) {
    let mut new_item = models::NewItem::new("New".into());
    let contents: String = "Enter text here".into();
    let mc = crate::APP_CORE.magic_crypt.lock().unwrap().clone();
    let mc_ref = mc.as_ref().expect("failed to get magic_crypt");

    new_item.contents = Some(mc_ref.encrypt_str_to_base64(contents));
    match item_actions::insert(&new_item) {
        Ok(_) => {
            let value = glib::value::Value::from(&new_item.title);
            let iter = store.append();
            store.set_value(&iter, 0u32, &value);
            let path = store.path(&iter).expect("Couldn't get path");
            tree_view.selection().select_path(&path);
        }
        Err(e) => warn!("{}", e),
    }
}

fn import_menu_item_action(main_window: &gtk::Window, store: &gtk::ListStore, tree_view: &gtk::TreeView) {
    let file_chooser_dialog = gtk::FileChooserDialogBuilder::new()
        .title("Choose a file to import")
        .show_hidden(true)
        .select_multiple(true)
        .transient_for(main_window)
        .action(gtk::FileChooserAction::Open)
        .build();

    file_chooser_dialog.add_button("Open", gtk::ResponseType::Ok);
    file_chooser_dialog.add_button("Cancel", gtk::ResponseType::Cancel);

    let mc = crate::APP_CORE.magic_crypt.lock().unwrap().clone();
    let mc_ref = mc.as_ref().expect("failed to get magic_crypt");

    if file_chooser_dialog.run() == gtk::ResponseType::Ok {
        let files = file_chooser_dialog.filenames();
        files.iter().for_each(|z| info!("file: {}", z.to_string_lossy()));
        for path in files.iter() {
            let item_title: String = path.file_name().unwrap().to_os_string().into_string().unwrap();
            let mut new_item = models::NewItem::new(item_title);
            let contents = std::fs::read_to_string(path.as_path()).unwrap();
            new_item.contents = Some(mc_ref.encrypt_str_to_base64(contents));
            match item_actions::insert(&new_item) {
                Ok(_) => {
                    let value = glib::value::Value::from(&new_item.title);
                    let iter = store.append();
                    store.set_value(&iter, 0u32, &value);
                    let path = store.path(&iter).expect("Couldn't get path");
                    tree_view.selection().select_path(&path);
                }
                Err(e) => warn!("{}", e),
            }
        }
    }

    file_chooser_dialog.close();
}

fn export_menu_item_action() {
    let project_dir = dirs::home_dir().unwrap().join(".senoru");
    if !project_dir.as_path().exists() {
        std::fs::create_dir_all(&project_dir).ok();
    }
    let export_dir = project_dir.join("export");
    if !export_dir.as_path().exists() {
        std::fs::create_dir_all(&export_dir).ok();
    }

    let mc = crate::APP_CORE.magic_crypt.lock().unwrap().clone();
    let mc_ref = mc.as_ref().expect("failed to get magic_crypt");

    let items = item_actions::find_all(None).expect("failed to get Items");
    for item in items.iter().cloned() {
        let output_file = export_dir.join(&item.title);
        let mut bw = io::BufWriter::new(fs::File::create(output_file.as_path()).unwrap());
        let contents = item.decrypt_contents(&mc_ref).expect("failed to decrypt item");
        bw.write_all(contents.as_bytes()).expect("Unable to write data");
    }

    let info_dialog = gtk::MessageDialogBuilder::new()
        .title("Export")
        .buttons(gtk::ButtonsType::Ok)
        .message_type(gtk::MessageType::Info)
        .modal(true)
        .text(format!("Items were written to: {}", export_dir.to_string_lossy()).as_str())
        .build();
    info_dialog.run();
    info_dialog.close();
}

fn remove_menu_item_action(store: &gtk::ListStore, tree_view: &gtk::TreeView, text_view: &gtk::TextView) {
    let selection = tree_view.selection();
    let (model, iter) = selection.selected().expect("Couldn't get selected");
    let selected_title = model.value(&iter, 0).get::<String>().expect("failed to get selected title");
    let item = item_actions::find_by_title(&selected_title).expect("failed to find Item by title");
    match item {
        Some(i) => {
            item_actions::delete(&i.id).expect("failed to delete item");
            store.remove(&iter);
            match store.iter_first() {
                Some(_) => {}
                None => {
                    let text_view_buffer = text_view.buffer().expect("Couldn't get buffer");
                    text_view_buffer.set_text(&"");
                }
            }
        }
        None => {}
    }
}

fn tree_view_selection_changed(tree_selection: &gtk::TreeSelection, text_view: &gtk::TextView) {
    let mc = crate::APP_CORE.magic_crypt.lock().unwrap().clone();
    let mc_ref = mc.as_ref().expect("failed to get magic_crypt");
    match tree_selection.selected() {
        Some((model, iter)) => {
            let selected_title = model.value(&iter, 0).get::<String>().expect("failed to get selected title");
            let text_view_buffer = text_view.buffer().expect("Couldn't get buffer");
            // match selected_title {
            //     Some(title) => {
            let item = item_actions::find_by_title(&selected_title).expect("failed to find Item by title");
            match item {
                Some(i) => {
                    text_view_buffer.set_text(&i.decrypt_contents(&mc_ref).unwrap());
                }
                None => text_view_buffer.set_text(&""),
            }
            //     }
            //     _ => text_view_buffer.set_text(&""),
            // }
        }
        None => {}
    }
}

fn tree_view_cell_renderer_edited(new_title: &str, tree_view: &gtk::TreeView, store: &gtk::ListStore) {
    let selection = tree_view.selection();
    let (model, iter) = selection.selected().expect("Couldn't get selected");
    let selected_title = model.value(&iter, 0).get::<String>().expect("failed to get selected title");
    let item = item_actions::find_by_title(&selected_title).expect("failed to find Item by title");
    match item {
        Some(mut i) => {
            i.title = new_title.to_string();
            item_actions::update(&i).expect("failed to update item");
            let value = glib::value::Value::from(&i.title);
            store.set_value(&iter, 0u32, &value);
        }
        None => {}
    }
}

fn text_view_key_press_event_action(tree_view: &gtk::TreeView, text_view: &gtk::TextView) {
    let mc = crate::APP_CORE.magic_crypt.lock().unwrap().clone();
    let mc_ref = mc.as_ref().expect("failed to get magic_crypt");
    let selection = tree_view.selection();
    let (model, iter) = selection.selected().expect("Couldn't get selected");
    let selected_title = model.value(&iter, 0).get::<String>().expect("failed to get selected title");
    let item = item_actions::find_by_title(&selected_title).expect("failed to find Item by title");
    match item {
        Some(mut i) => {
            let buffer = text_view.buffer().expect("Couldn't get buffer");
            let contents = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false)
                .expect("failed to get content")
                .to_string();
            i.contents = Some(mc_ref.encrypt_str_to_base64(contents));
            item_actions::update(&i).expect("failed to update item");
        }
        None => {}
    }
}
