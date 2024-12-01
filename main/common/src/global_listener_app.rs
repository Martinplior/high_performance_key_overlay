use std::{
    mem::{ManuallyDrop, MaybeUninit},
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    time::Instant,
};

use bytemuck::{AnyBitPattern, NoUninit};
use windows::Win32::{
    Devices::HumanInterfaceDevice::{HID_USAGE_GENERIC_KEYBOARD, HID_USAGE_PAGE_GENERIC},
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::{
            GetRawInputData, KeyboardAndMouse::VIRTUAL_KEY, RegisterRawInputDevices, HRAWINPUT,
            RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RIDEV_INPUTSINK, RID_INPUT, RIM_TYPEKEYBOARD,
        },
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, GetMessageW,
            PostMessageW, RegisterClassExW, HWND_MESSAGE, MSG, RI_KEY_BREAK, RI_KEY_E0, WM_CLOSE,
            WM_INPUT, WNDCLASSEXW,
        },
    },
};

use crate::{interprocess_channel, key::Key, key_message::KeyMessage};

use interprocess_channel::NonBlockReceiver as IpcReceiver;
use interprocess_channel::NonBlockSender as IpcSender;

pub struct ListenerWrap<T: Send + NoUninit + AnyBitPattern> {
    process: std::process::Child,
    sender_handle: ManuallyDrop<OwnedHandle>,
    receiver: IpcReceiver<T>,
    _phantom_data: std::marker::PhantomData<T>,
}

impl<T: Send + NoUninit + AnyBitPattern> ListenerWrap<T> {
    pub fn new() -> Self {
        let (sender, receiver) = interprocess_channel::bounded(crate::CHANNEL_CAP);
        let sender_handle: OwnedHandle = sender.into();
        let raw_sender_handle = sender_handle.as_raw_handle() as usize;
        let process =
            std::process::Command::new(crate::get_current_dir().join("global_listener_app"))
                .arg(MainApp::UNIQUE_IDENT)
                .arg(raw_sender_handle.to_string())
                .spawn()
                .unwrap();
        Self {
            process,
            sender_handle: ManuallyDrop::new(sender_handle),
            receiver: IpcReceiver::bounded(receiver, crate::CHANNEL_CAP),
            _phantom_data: Default::default(),
        }
    }

    #[inline(always)]
    pub fn try_iter(&mut self) -> crossbeam::channel::TryIter<'_, T> {
        self.receiver.try_iter()
    }
}

impl<T: Send + NoUninit + AnyBitPattern> Drop for ListenerWrap<T> {
    fn drop(&mut self) {
        let hwnd =
            unsafe { FindWindowW(MainApp::window_class_name(), MainApp::window_name()) }.unwrap();
        unsafe { PostMessageW(hwnd, WM_CLOSE, None, None) }.unwrap();
        self.process.wait().unwrap();
        unsafe { ManuallyDrop::drop(&mut self.sender_handle) };
    }
}

pub struct MainApp;

impl MainApp {
    pub const UNIQUE_IDENT: &str = "global_listener_app::MainApp::UNIQUE_IDENT";

    pub fn window_class_name() -> windows::core::PCWSTR {
        windows::core::w!("global_listener_window_class")
    }

    pub fn window_name() -> windows::core::PCWSTR {
        windows::core::w!("global_listener_msg_window")
    }

    pub fn new() -> Self {
        Self
    }

    pub fn run(self) {
        let args: Box<[_]> = std::env::args().collect();
        let len = args.len();
        if len < 3 {
            panic!("违法参数！");
        }
        if args[1] != Self::UNIQUE_IDENT {
            panic!("违法参数！");
        }
        let raw_handle = usize::from_str_radix(&args[2], 10).unwrap();

        let owned_handle = unsafe { OwnedHandle::from_raw_handle(raw_handle as _) };

        let msg_sender = interprocess_channel::Sender::<KeyMessage>::from(owned_handle);
        let cap = crate::CHANNEL_CAP;
        let msg_sender = IpcSender::bounded(msg_sender, cap);

        let window_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as _,
            lpfnWndProc: Some(Self::wnd_proc),
            hInstance: unsafe { GetModuleHandleW(None) }.unwrap().into(),
            lpszClassName: Self::window_class_name(),
            ..Default::default()
        };
        if unsafe { RegisterClassExW(&window_class) } == 0 {
            panic!("RegisterClassExW failed!");
        }

        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                window_class.lpszClassName,
                Self::window_name(),
                Default::default(),
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                None,
                None,
                None,
            )
        }
        .unwrap();

        let raw_input_device = RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_KEYBOARD,
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        };
        unsafe {
            RegisterRawInputDevices(
                &[raw_input_device],
                std::mem::size_of::<RAWINPUTDEVICE>() as _,
            )
        }
        .unwrap();

        loop {
            let mut msg = MaybeUninit::uninit();
            let r = unsafe { GetMessageW(msg.as_mut_ptr(), hwnd, 0, 0) }.0;
            if r == 0 || r == -1 {
                #[cfg(debug_assertions)]
                println!("global_listener get message failed and exit");
                break;
            }
            let msg = unsafe { msg.assume_init() };
            Self::handle_raw_input(&msg, &msg_sender);
            unsafe { DispatchMessageW(&msg) };
        }
        println!("global_listener end");
    }

    #[inline(always)]
    fn handle_raw_input(msg: &MSG, msg_sender: &IpcSender<KeyMessage>) {
        if msg.message != WM_INPUT {
            return;
        }
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
        if raw_input.header.dwType != RIM_TYPEKEYBOARD.0 {
            return;
        }
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
        msg_sender.send(key_message).unwrap();
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }
}
