#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use common::kps_dashboard_app::KPSApp;

fn main() {
    let _ = common::graceful_run(KPSApp::run);
}
