use std::time::Instant;

use crate::key::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct KeyMessage {
    pub key: Key,
    pub is_pressed: bool,
    pub instant: Instant,
}

impl KeyMessage {
    pub fn new(key: Key, is_pressed: bool, instant: Instant) -> Self {
        Self {
            key,
            is_pressed,
            instant,
        }
    }
}

unsafe impl bytemuck::Zeroable for KeyMessage {}
unsafe impl bytemuck::NoUninit for KeyMessage {}
unsafe impl bytemuck::AnyBitPattern for KeyMessage {}
