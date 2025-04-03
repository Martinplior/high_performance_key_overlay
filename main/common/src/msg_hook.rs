use sak_rs::os::windows::input::{global_listener::WinMsg, raw_input};
use windows::Win32::{
    Foundation::HWND,
    UI::{Input::KeyboardAndMouse::VK_SHIFT, WindowsAndMessaging::WM_INPUT},
};

use crossbeam::channel::Sender as MpscSender;

use crate::{key::Key, key_message::KeyMessage};

#[derive(Debug, Default)]
pub struct HookShared {
    pub egui_ctx: egui::Context,
}

pub fn create_register_raw_input_hook() -> impl FnOnce(&HWND) {
    |&hwnd| {
        use sak_rs::os::windows::input::raw_input::device;
        device::register(
            device::DeviceType::Keyboard,
            device::OptionType::inputsink_with_no_legacy(hwnd),
        );
        device::register(
            device::DeviceType::Mouse,
            device::OptionType::inputsink(hwnd),
        );
    }
}

pub fn create_msg_hook(
    msg_sender: MpscSender<KeyMessage>,
    hook_shared: HookShared,
) -> impl FnMut(&WinMsg) -> bool {
    move |msg| {
        if msg.msg.message == WM_INPUT {
            handle_raw_input(msg, &msg_sender);
            let ctx = &hook_shared.egui_ctx;
            (!ctx.has_requested_repaint()).then(|| ctx.request_repaint());
            return true;
        }
        false
    }
}

#[inline(always)]
fn handle_raw_input(msg: &WinMsg, msg_sender: &MpscSender<KeyMessage>) {
    let raw_input = raw_input::RawInput::from_msg(&msg.msg);
    match raw_input {
        raw_input::RawInput::Keyboard(keyboard) => {
            let virtual_key = keyboard.virtual_key();
            let is_extend = if virtual_key == VK_SHIFT {
                keyboard.make_code() == 0x0036
            } else {
                keyboard.has_e0()
            };
            let key = if keyboard.make_code() == 0x0037 && is_extend {
                Key::PrintScreen
            } else {
                Key::from_virtual_key(virtual_key, is_extend)
            };
            if key == Key::Unknown {
                #[cfg(debug_assertions)]
                println!(
                    "unkown: mc = {:#x?}, vk = {:?}, is_ext = {:?}",
                    keyboard.make_code(),
                    virtual_key,
                    is_extend
                );
                return;
            }
            let is_pressed = keyboard.key_is_down();
            let key_message = KeyMessage::new(key, is_pressed, msg.instant);
            #[cfg(debug_assertions)]
            println!("{:?}", key_message);
            msg_sender.send(key_message).unwrap();
        }
        raw_input::RawInput::Mouse(mouse) => {
            [
                (mouse.is_left_button_down(), Key::MouseLeft, true),
                (mouse.is_right_button_down(), Key::MouseRight, true),
                (mouse.is_middle_button_down(), Key::MouseMiddle, true),
                (mouse.is_ext1_button_down(), Key::MouseX1, true),
                (mouse.is_ext2_button_down(), Key::MouseX2, true),
                (mouse.is_left_button_up(), Key::MouseLeft, false),
                (mouse.is_right_button_up(), Key::MouseRight, false),
                (mouse.is_middle_button_up(), Key::MouseMiddle, false),
                (mouse.is_ext1_button_up(), Key::MouseX1, false),
                (mouse.is_ext2_button_up(), Key::MouseX2, false),
            ]
            .into_iter()
            .filter(|(cond, ..)| *cond)
            .for_each(|(_, key, is_pressed)| {
                let key_message = KeyMessage::new(key, is_pressed, msg.instant);
                #[cfg(debug_assertions)]
                println!("{:?}", key_message);
                msg_sender.send(key_message).unwrap();
            });
        }
        _ => unreachable!("unexpected raw input"),
    };
}
