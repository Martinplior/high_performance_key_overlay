use crate::{message_dialog, setting::Setting};

use super::AppSharedData;

pub struct Menu {
    file: File,
    modified: bool,
    request_discard: bool,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            file: File::new(),
            modified: false,
            request_discard: false,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("文件", |ui| self.file.show(ui));
                let button = egui::Button::new("放弃所有修改");
                ui.add_enabled(self.modified, button).clicked().then(|| {
                    let r = message_dialog::confirm("是否放弃所有修改？")
                        .set_level(rfd::MessageLevel::Warning)
                        .show();
                    if r == rfd::MessageDialogResult::Ok {
                        self.request_discard = true;
                    }
                });
            });
        });
    }

    pub fn update(&mut self, egui_ctx: &egui::Context, app_shared_data: &mut AppSharedData) {
        self.modified = app_shared_data.modified;
        self.handle_discard(app_shared_data);
        self.handle_exit(egui_ctx);
        self.handle_keyboard_shortcut(egui_ctx);
        self.file.update(app_shared_data);
    }

    fn handle_discard(&mut self, app_shared_data: &mut AppSharedData) {
        std::mem::take(&mut self.request_discard).then(|| {
            app_shared_data.pending_setting = Some(app_shared_data.loaded_setting.clone());
        });
    }

    fn handle_exit(&mut self, egui_ctx: &egui::Context) {
        let close_requested =
            egui_ctx.input(|input_state| input_state.viewport().close_requested());

        if self.modified && close_requested {
            // FIX: 当处于modified状态，聚焦到预览窗口，再直接关闭窗口时，会导致警告窗口无法
            // 正确弹出
            // that's weird...
            std::thread::scope(|s| {
                s.spawn(|| {
                    let r = message_dialog::confirm("当前配置未保存，是否继续退出？")
                        .set_level(rfd::MessageLevel::Warning)
                        .show();
                    if r == rfd::MessageDialogResult::Cancel {
                        egui_ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    };
                });
            });
        };
    }

    fn handle_keyboard_shortcut(&mut self, egui_ctx: &egui::Context) {
        egui_ctx.input_mut(|input_state| {
            let ctrl_shit_s = input_state.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                egui::Key::S,
            ));
            if ctrl_shit_s {
                self.file.response = Some(FileResponse::SaveFileAs);
                return;
            }
            let ctrl_s = input_state.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::S,
            ));
            if ctrl_s {
                self.file.response = Some(FileResponse::SaveFile);
            };
        });
    }
}

#[derive(Debug)]
enum FileResponse {
    LoadFile,
    SaveFile,
    SaveFileAs,
    LoadDefaultSetting(fn() -> Setting),
}

struct File {
    response: Option<FileResponse>,
}

impl File {
    fn new() -> Self {
        Self { response: None }
    }

    fn update(&mut self, app_shared_data: &mut AppSharedData) {
        let file_dialog = || {
            rfd::FileDialog::new()
                .set_directory(crate::get_current_dir())
                .add_filter("", &["json"])
        };
        self.response.take().map(|r| match r {
            FileResponse::LoadFile => {
                if app_shared_data.modified {
                    let r = message_dialog::confirm("当前配置未保存，是否继续打开新文件？")
                        .set_level(rfd::MessageLevel::Warning)
                        .show();
                    if r == rfd::MessageDialogResult::Cancel {
                        return;
                    }
                }
                file_dialog().pick_file().map(|path| {
                    #[cfg(debug_assertions)]
                    dbg!(&path);
                    let _ = Setting::from_file(&path)
                        .map(|setting| {
                            app_shared_data.loaded_setting = setting.clone();
                            app_shared_data.pending_setting = Some(setting);
                            app_shared_data.load_path = path;
                            app_shared_data.modified = false;
                        })
                        .map_err(|err| {
                            message_dialog::warning(err).show();
                        });
                });
            }
            FileResponse::SaveFile => {
                let path = &app_shared_data.load_path;
                let _ = app_shared_data
                    .current_setting
                    .clone()
                    .to_file(path)
                    .map(|_| {
                        app_shared_data.loaded_setting = app_shared_data.current_setting.clone();
                        app_shared_data.modified = false;
                        message_dialog::info("保存成功！").show();
                    })
                    .map_err(|err| {
                        message_dialog::warning(err).show();
                    });
            }
            FileResponse::SaveFileAs => {
                file_dialog()
                    .set_file_name("新建配置文件.json")
                    .save_file()
                    .map(|mut path| {
                        let extention = path.extension().map_or_else(
                            || "json".to_string(),
                            |extention| {
                                let extention = extention.to_str().unwrap_or_default().to_string();
                                if extention == "json" {
                                    extention
                                } else {
                                    extention + ".json"
                                }
                            },
                        );
                        path.set_extension(extention);
                        #[cfg(debug_assertions)]
                        dbg!(&path);
                        let _ = app_shared_data
                            .current_setting
                            .clone()
                            .to_file(&path)
                            .map(|_| {
                                app_shared_data.loaded_setting =
                                    app_shared_data.current_setting.clone();
                                app_shared_data.load_path = path;
                                app_shared_data.modified = false;
                                message_dialog::info("保存成功！").show();
                            })
                            .map_err(|err| {
                                message_dialog::warning(err).show();
                            });
                    });
            }
            FileResponse::LoadDefaultSetting(setting) => {
                app_shared_data.pending_setting = Some(setting());
            }
        });
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        ui.button("打开...")
            .on_hover_text("打开一个配置文件")
            .clicked()
            .then(|| {
                self.response = Some(FileResponse::LoadFile);
                ui.close_menu();
            });

        ui.add(egui::Button::new("保存").shortcut_text("Ctrl + S"))
            .on_hover_text("保存当前配置文件")
            .clicked()
            .then(|| {
                self.response = Some(FileResponse::SaveFile);
                ui.close_menu();
            });

        ui.add(egui::Button::new("另存为...").shortcut_text("Ctrl + Shift + S"))
            .on_hover_text("将当前配置保存到新文件")
            .clicked()
            .then(|| {
                self.response = Some(FileResponse::SaveFileAs);
                ui.close_menu();
            });

        ui.menu_button("加载预设配置", |ui| {
            ui.button("ZXC").clicked().then(|| {
                self.response = Some(FileResponse::LoadDefaultSetting(Setting::default_zxc));
            });
            ui.button("鼠标").clicked().then(|| {
                self.response = Some(FileResponse::LoadDefaultSetting(Setting::default_mouse));
            });
            ui.button("方向键").clicked().then(|| {
                self.response = Some(FileResponse::LoadDefaultSetting(
                    Setting::default_four_directions,
                ));
            });
            ui.button("4K").clicked().then(|| {
                self.response = Some(FileResponse::LoadDefaultSetting(Setting::default_4k));
            });
            ui.button("7K").clicked().then(|| {
                self.response = Some(FileResponse::LoadDefaultSetting(Setting::default_7k));
            });
            ui.button("26K").clicked().then(|| {
                self.response = Some(FileResponse::LoadDefaultSetting(Setting::default_26k));
            });
            ui.button("HelloWorld").clicked().then(|| {
                self.response = Some(FileResponse::LoadDefaultSetting(
                    Setting::default_hello_world,
                ));
            });
            ui.button("单个计数器").clicked().then(|| {
                self.response = Some(FileResponse::LoadDefaultSetting(
                    Setting::default_single_counter,
                ));
            });
        });
    }
}
