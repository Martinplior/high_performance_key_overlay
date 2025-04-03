#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use windows::Win32::UI::Input::KeyboardAndMouse::{self, VIRTUAL_KEY};

/// [see also](https://docs.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum Key {
    Escape,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    PrintScreen,
    ScrollLock,
    Pause,
    /// ``` `~ ``` key
    BackTick,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    /// `-_` key
    Minus,
    /// `=+` key
    Equal,
    Backspace,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    /// `[{` key
    LeftSquareBracket,
    /// `]}` key
    RightSquareBracket,
    /// `\|` key
    BackwardSlash,
    /// `;:` key
    Semicolon,
    /// `'"` key
    Apostrophe,
    /// `,<` key
    Comma,
    /// `.>` key
    Period,
    /// `/?` key
    ForwardSlash,
    Enter,
    Space,
    LeftControl,
    RightControl,
    LeftShift,
    RightShift,
    LeftAlt,
    RightAlt,
    LeftWin,
    RightWin,
    Apps,
    Tab,
    CapsLock,
    /// `↑` key
    Up,
    /// `↓` key
    Down,
    /// `←` key
    Left,
    /// `→` key
    Right,
    NumLock,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    Numpad0,
    NumpadPlus,
    NumpadMinus,
    NumpadMultiply,
    NumpadDivide,
    NumpadSeparator,
    NumpadDot,
    NumpadEnter,

    // Mouse extend
    MouseLeft,
    MouseRight,
    MouseMiddle,
    MouseX1,
    MouseX2,

    #[default]
    Unknown,
}

impl Key {
    pub const LAST_KEY: Self = Self::Unknown;

    pub fn to_string(&self) -> String {
        format!("{:?}", self)
    }

    pub fn from_virtual_key(virtual_key: VIRTUAL_KEY, is_extend: bool) -> Self {
        use Key::*;
        use KeyboardAndMouse::*;
        const LUT: [Key; 0x200] = {
            let mut lut = [Unknown; 0x200];
            lut[VK_ESCAPE.0 as usize] = Escape;
            lut[VK_F1.0 as usize] = F1;
            lut[VK_F2.0 as usize] = F2;
            lut[VK_F3.0 as usize] = F3;
            lut[VK_F4.0 as usize] = F4;
            lut[VK_F5.0 as usize] = F5;
            lut[VK_F6.0 as usize] = F6;
            lut[VK_F7.0 as usize] = F7;
            lut[VK_F8.0 as usize] = F8;
            lut[VK_F9.0 as usize] = F9;
            lut[VK_F10.0 as usize] = F10;
            lut[VK_F11.0 as usize] = F11;
            lut[VK_F12.0 as usize] = F12;
            lut[VK_SNAPSHOT.0 as usize] = PrintScreen;
            lut[VK_SCROLL.0 as usize] = ScrollLock;
            lut[VK_PAUSE.0 as usize] = Pause;
            lut[VK_OEM_3.0 as usize] = BackTick;
            lut[VK_1.0 as usize] = Key1;
            lut[VK_2.0 as usize] = Key2;
            lut[VK_3.0 as usize] = Key3;
            lut[VK_4.0 as usize] = Key4;
            lut[VK_5.0 as usize] = Key5;
            lut[VK_6.0 as usize] = Key6;
            lut[VK_7.0 as usize] = Key7;
            lut[VK_8.0 as usize] = Key8;
            lut[VK_9.0 as usize] = Key9;
            lut[VK_0.0 as usize] = Key0;
            lut[VK_OEM_MINUS.0 as usize] = Minus;
            lut[VK_OEM_PLUS.0 as usize] = Equal;
            lut[VK_BACK.0 as usize] = Backspace;
            lut[VK_INSERT.0 as usize] = Insert;
            lut[VK_DELETE.0 as usize] = Delete;
            lut[VK_HOME.0 as usize] = Home;
            lut[VK_END.0 as usize] = End;
            lut[VK_PRIOR.0 as usize] = PageUp;
            lut[VK_NEXT.0 as usize] = PageDown;
            lut[VK_A.0 as usize] = KeyA;
            lut[VK_B.0 as usize] = KeyB;
            lut[VK_C.0 as usize] = KeyC;
            lut[VK_D.0 as usize] = KeyD;
            lut[VK_E.0 as usize] = KeyE;
            lut[VK_F.0 as usize] = KeyF;
            lut[VK_G.0 as usize] = KeyG;
            lut[VK_H.0 as usize] = KeyH;
            lut[VK_I.0 as usize] = KeyI;
            lut[VK_J.0 as usize] = KeyJ;
            lut[VK_K.0 as usize] = KeyK;
            lut[VK_L.0 as usize] = KeyL;
            lut[VK_M.0 as usize] = KeyM;
            lut[VK_N.0 as usize] = KeyN;
            lut[VK_O.0 as usize] = KeyO;
            lut[VK_P.0 as usize] = KeyP;
            lut[VK_Q.0 as usize] = KeyQ;
            lut[VK_R.0 as usize] = KeyR;
            lut[VK_S.0 as usize] = KeyS;
            lut[VK_T.0 as usize] = KeyT;
            lut[VK_U.0 as usize] = KeyU;
            lut[VK_V.0 as usize] = KeyV;
            lut[VK_W.0 as usize] = KeyW;
            lut[VK_X.0 as usize] = KeyX;
            lut[VK_Y.0 as usize] = KeyY;
            lut[VK_Z.0 as usize] = KeyZ;
            lut[VK_OEM_4.0 as usize] = LeftSquareBracket;
            lut[VK_OEM_6.0 as usize] = RightSquareBracket;
            lut[VK_OEM_5.0 as usize] = BackwardSlash;
            lut[VK_OEM_1.0 as usize] = Semicolon;
            lut[VK_OEM_7.0 as usize] = Apostrophe;
            lut[VK_OEM_COMMA.0 as usize] = Comma;
            lut[VK_OEM_PERIOD.0 as usize] = Period;
            lut[VK_OEM_2.0 as usize] = ForwardSlash;

            lut[VK_RETURN.0 as usize] = Enter;
            lut[VK_RETURN.0 as usize + 0x100] = NumpadEnter;
            lut[VK_CONTROL.0 as usize] = LeftControl;
            lut[VK_CONTROL.0 as usize + 0x100] = RightControl;
            lut[VK_SHIFT.0 as usize] = LeftShift;
            lut[VK_SHIFT.0 as usize + 0x100] = RightShift;
            lut[VK_MENU.0 as usize] = LeftAlt;
            lut[VK_MENU.0 as usize + 0x100] = RightAlt;

            lut[VK_LCONTROL.0 as usize] = LeftControl;
            lut[VK_RCONTROL.0 as usize] = RightControl;
            lut[VK_LSHIFT.0 as usize] = LeftShift;
            lut[VK_RSHIFT.0 as usize] = RightShift;
            lut[VK_LMENU.0 as usize] = LeftAlt;
            lut[VK_RMENU.0 as usize] = RightAlt;
            lut[VK_SPACE.0 as usize] = Space;
            lut[VK_LWIN.0 as usize] = LeftWin;
            lut[VK_RWIN.0 as usize] = RightWin;
            lut[VK_APPS.0 as usize] = Apps;
            lut[VK_TAB.0 as usize] = Tab;
            lut[VK_CAPITAL.0 as usize] = CapsLock;

            lut[VK_UP.0 as usize] = Up;
            lut[VK_UP.0 as usize + 0x100] = Up;
            lut[VK_DOWN.0 as usize] = Down;
            lut[VK_DOWN.0 as usize + 0x100] = Down;
            lut[VK_LEFT.0 as usize] = Left;
            lut[VK_LEFT.0 as usize + 0x100] = Left;
            lut[VK_RIGHT.0 as usize] = Right;
            lut[VK_RIGHT.0 as usize + 0x100] = Right;

            lut[VK_NUMLOCK.0 as usize] = NumLock;
            lut[VK_NUMPAD1.0 as usize] = Numpad1;
            lut[VK_NUMPAD2.0 as usize] = Numpad2;
            lut[VK_NUMPAD3.0 as usize] = Numpad3;
            lut[VK_NUMPAD4.0 as usize] = Numpad4;
            lut[VK_NUMPAD5.0 as usize] = Numpad5;
            lut[VK_NUMPAD6.0 as usize] = Numpad6;
            lut[VK_NUMPAD7.0 as usize] = Numpad7;
            lut[VK_NUMPAD8.0 as usize] = Numpad8;
            lut[VK_NUMPAD9.0 as usize] = Numpad9;
            lut[VK_NUMPAD0.0 as usize] = Numpad0;
            lut[VK_ADD.0 as usize] = NumpadPlus;
            lut[VK_SUBTRACT.0 as usize] = NumpadMinus;
            lut[VK_MULTIPLY.0 as usize] = NumpadMultiply;
            lut[VK_DIVIDE.0 as usize] = NumpadDivide;
            lut[VK_SEPARATOR.0 as usize] = NumpadSeparator;
            lut[VK_DECIMAL.0 as usize] = NumpadDot;

            lut[VK_LBUTTON.0 as usize] = MouseLeft;
            lut[VK_RBUTTON.0 as usize] = MouseRight;
            lut[VK_MBUTTON.0 as usize] = MouseMiddle;
            lut[VK_XBUTTON1.0 as usize] = MouseX1;
            lut[VK_XBUTTON2.0 as usize] = MouseX2;

            lut
        };
        let index = virtual_key.0 as usize + ((is_extend as usize) << 8);
        unsafe { *LUT.get_unchecked(index) }
    }

    pub fn to_virtual_key(self) -> VIRTUAL_KEY {
        use Key::*;
        use KeyboardAndMouse::*;
        match self {
            Escape => VK_ESCAPE,
            F1 => VK_F1,
            F2 => VK_F2,
            F3 => VK_F3,
            F4 => VK_F4,
            F5 => VK_F5,
            F6 => VK_F6,
            F7 => VK_F7,
            F8 => VK_F8,
            F9 => VK_F9,
            F10 => VK_F10,
            F11 => VK_F11,
            F12 => VK_F12,
            PrintScreen => VK_SNAPSHOT,
            ScrollLock => VK_SCROLL,
            Pause => VK_PAUSE,
            BackTick => VK_OEM_3,
            Key1 => VK_1,
            Key2 => VK_2,
            Key3 => VK_3,
            Key4 => VK_4,
            Key5 => VK_5,
            Key6 => VK_6,
            Key7 => VK_7,
            Key8 => VK_8,
            Key9 => VK_9,
            Key0 => VK_0,
            Minus => VK_OEM_MINUS,
            Equal => VK_OEM_PLUS,
            Backspace => VK_BACK,
            Insert => VK_INSERT,
            Delete => VK_DELETE,
            Home => VK_HOME,
            End => VK_END,
            PageUp => VK_PRIOR,
            PageDown => VK_NEXT,
            KeyA => VK_A,
            KeyB => VK_B,
            KeyC => VK_C,
            KeyD => VK_D,
            KeyE => VK_E,
            KeyF => VK_F,
            KeyG => VK_G,
            KeyH => VK_H,
            KeyI => VK_I,
            KeyJ => VK_J,
            KeyK => VK_K,
            KeyL => VK_L,
            KeyM => VK_M,
            KeyN => VK_N,
            KeyO => VK_O,
            KeyP => VK_P,
            KeyQ => VK_Q,
            KeyR => VK_R,
            KeyS => VK_S,
            KeyT => VK_T,
            KeyU => VK_U,
            KeyV => VK_V,
            KeyW => VK_W,
            KeyX => VK_X,
            KeyY => VK_Y,
            KeyZ => VK_Z,
            LeftSquareBracket => VK_OEM_4,
            RightSquareBracket => VK_OEM_6,
            BackwardSlash => VK_OEM_5,
            Semicolon => VK_OEM_1,
            Apostrophe => VK_OEM_7,
            Comma => VK_OEM_COMMA,
            Period => VK_OEM_PERIOD,
            ForwardSlash => VK_OEM_2,
            Enter => VK_RETURN,
            Space => VK_SPACE,
            LeftControl => VK_LCONTROL,
            RightControl => VK_RCONTROL,
            LeftShift => VK_LSHIFT,
            RightShift => VK_RSHIFT,
            LeftAlt => VK_LMENU,
            RightAlt => VK_RMENU,
            LeftWin => VK_LWIN,
            RightWin => VK_RWIN,
            Apps => VK_APPS,
            Tab => VK_TAB,
            CapsLock => VK_CAPITAL,
            Up => VK_UP,
            Down => VK_DOWN,
            Left => VK_LEFT,
            Right => VK_RIGHT,
            NumLock => VK_NUMLOCK,
            Numpad1 => VK_NUMPAD1,
            Numpad2 => VK_NUMPAD2,
            Numpad3 => VK_NUMPAD3,
            Numpad4 => VK_NUMPAD4,
            Numpad5 => VK_NUMPAD5,
            Numpad6 => VK_NUMPAD6,
            Numpad7 => VK_NUMPAD7,
            Numpad8 => VK_NUMPAD8,
            Numpad9 => VK_NUMPAD9,
            Numpad0 => VK_NUMPAD0,
            NumpadPlus => VK_ADD,
            NumpadMinus => VK_SUBTRACT,
            NumpadMultiply => VK_MULTIPLY,
            NumpadDivide => VK_DIVIDE,
            NumpadSeparator => VK_SEPARATOR,
            NumpadDot => VK_DECIMAL,
            NumpadEnter => VK_RETURN,

            MouseLeft => VK_LBUTTON,
            MouseRight => VK_RBUTTON,
            MouseMiddle => VK_MBUTTON,
            MouseX1 => VK_XBUTTON1,
            MouseX2 => VK_XBUTTON2,

            Unknown => Default::default(),
        }
    }

    pub fn iter() -> impl DoubleEndedIterator<Item = Self> + Clone {
        (0..=Self::LAST_KEY as u8).map(|v| unsafe { std::mem::transmute::<u8, Self>(v) })
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn t1() {
        let vec: Vec<_> = Key::iter()
            .map(|x| {
                let k = x.to_virtual_key();
                println!("{:?}", k);
                k
            })
            .collect();
        vec.into_iter()
            .for_each(|vk| println!("{}", Key::from_virtual_key(vk, false)));
    }

    #[test]
    fn t2() {
        use KeyboardAndMouse::*;
        [
            (VK_RETURN, false),
            (VK_RETURN, true),
            (VK_CONTROL, false),
            (VK_CONTROL, true),
            (VK_SHIFT, false),
            (VK_SHIFT, true),
            (VK_MENU, false),
            (VK_MENU, true),
        ]
        .into_iter()
        .for_each(|(vk, is_extend)| {
            let k = Key::from_virtual_key(vk, is_extend);
            println!("{:?}", k);
        });
    }
}
