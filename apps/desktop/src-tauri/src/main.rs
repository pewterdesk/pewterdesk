// Blank shell: loads the frontend (apps/desktop's Vite build) into a native
// window. No custom Tauri commands yet — those come with the first real
// feature that needs Rust-side capability (OS keychain access for signing
// keys, first and foremost). See CLAUDE.md's security-sensitive-code section
// before adding anything there.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running pewterdesk");
}
