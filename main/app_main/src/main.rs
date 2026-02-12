#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use common::app_main::MainApp;

fn main() {
    let _ = common::graceful_run(MainApp::run);
}
