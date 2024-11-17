#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use common::setting_app::SettingApp;

fn main() {
    let _ = common::graceful_run(|| SettingApp::new().run());
}
