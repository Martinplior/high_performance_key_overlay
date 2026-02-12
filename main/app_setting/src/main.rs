#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use common::app_setting::SettingApp;

fn main() {
    let _ = common::graceful_run(SettingApp::run);
}
