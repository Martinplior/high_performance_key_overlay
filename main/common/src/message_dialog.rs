pub fn error(description: impl Into<String>) -> rfd::MessageDialog {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("错误")
        .set_description(description.into())
}

pub fn warning(description: impl Into<String>) -> rfd::MessageDialog {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title("警告")
        .set_description(description.into())
}

pub fn info(description: impl Into<String>) -> rfd::MessageDialog {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title("信息")
        .set_description(description.into())
}

pub fn confirm(description: impl Into<String>) -> rfd::MessageDialog {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title("确认")
        .set_description(description.into())
        .set_buttons(rfd::MessageButtons::OkCancel)
}
