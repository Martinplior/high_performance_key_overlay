use std::{cell::OnceCell, rc::Rc};

use windows::Win32::UI::WindowsAndMessaging::{
    MSG, WM_INPUT, WM_KEYDOWN, WM_KEYUP, WM_MOUSEMOVE, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

#[derive(Debug, Default)]
pub struct HookShared {
    pub egui_ctx: OnceCell<egui::Context>,
}

impl HookShared {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}

pub fn create_msg_hook<const FILTER: bool>(
    hook_shared: Rc<HookShared>,
) -> impl FnMut(*const std::ffi::c_void) -> bool {
    move |msg| {
        let msg = unsafe { &*(msg as *const MSG) };
        if msg.message == WM_INPUT {
            hook_shared
                .egui_ctx
                .get()
                .map(|egui_ctx| egui_ctx.request_repaint());
            return true;
        }
        if const { FILTER } {
            if matches!(
                msg.message,
                WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP | WM_MOUSEMOVE
            ) {
                return true;
            }
        }
        false
    }
}
