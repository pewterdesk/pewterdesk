// Native shell for the PewterDesk frontend. The only Rust-side capability so
// far is the OS-keychain bridge in `keychain`
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod keychain;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            keychain::store_secret,
            keychain::get_secret,
            keychain::has_secret,
            keychain::delete_secret,
        ])
        .run(tauri::generate_context!())
        .expect("error while running pewterdesk");
}
