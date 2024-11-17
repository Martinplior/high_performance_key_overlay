use std::{cell::OnceCell, mem::MaybeUninit, rc::Rc, time::Instant};

use windows::Win32::UI::{
    Input::{
        GetRawInputData, KeyboardAndMouse::VIRTUAL_KEY, HRAWINPUT, RAWINPUT, RAWINPUTHEADER,
        RID_INPUT, RIM_TYPEKEYBOARD,
    },
    WindowsAndMessaging::{
        MSG, RI_KEY_BREAK, RI_KEY_E0, WM_INPUT, WM_KEYDOWN, WM_KEYUP, WM_MOUSEMOVE, WM_SYSKEYDOWN,
        WM_SYSKEYUP,
    },
};

use crate::{key::Key, key_message::KeyMessage};

use crossbeam::channel::Sender as MpscSender;

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
    keys_sender: MpscSender<KeyMessage>,
    hook_shared: Rc<HookShared>,
) -> impl FnMut(*const std::ffi::c_void) -> bool {
    move |msg| {
        let msg = unsafe { &*(msg as *const MSG) };
        if msg.message == WM_INPUT {
            handle_raw_input(msg, &keys_sender, &hook_shared);
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

#[inline(always)]
fn handle_raw_input(msg: &MSG, keys_sender: &MpscSender<KeyMessage>, hook_shared: &HookShared) {
    let l_param = msg.lParam.0 as usize;
    let raw_input = {
        let mut raw_input = MaybeUninit::<RAWINPUT>::uninit();
        let mut size = std::mem::size_of::<RAWINPUT>() as _;
        let header_size = std::mem::size_of::<RAWINPUTHEADER>() as _;
        let r = unsafe {
            GetRawInputData(
                HRAWINPUT(l_param as _),
                RID_INPUT,
                Some(raw_input.as_mut_ptr() as _),
                &mut size,
                header_size,
            )
        };
        if r == 0 || r as i32 == -1 {
            panic!("GetRawInputData Failed!");
        }
        unsafe { raw_input.assume_init() }
    };
    if raw_input.header.dwType == RIM_TYPEKEYBOARD.0 {
        let keyboard = unsafe { raw_input.data.keyboard };
        let virtual_key = VIRTUAL_KEY(keyboard.VKey);
        let is_extend = (keyboard.Flags & RI_KEY_E0 as u16) != 0;
        let key = Key::from_virtual_key(virtual_key, is_extend);
        if key == Key::Unknown {
            #[cfg(debug_assertions)]
            println!("vk = {:?}, is_ext = {:?}", virtual_key, is_extend);
            return;
        }
        let is_pressed = (keyboard.Flags & RI_KEY_BREAK as u16) == 0;
        let key_message = KeyMessage::new(key, is_pressed, Instant::now());
        #[cfg(debug_assertions)]
        println!("{:?}", key_message);
        keys_sender.send(key_message).unwrap();
        hook_shared
            .egui_ctx
            .get()
            .map(|egui_ctx| egui_ctx.request_repaint());
    }
}
