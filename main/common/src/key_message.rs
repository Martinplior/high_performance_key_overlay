use std::time::Instant;

use crate::key::Key;

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VKeyMessage {
    pub key: Key,
    pub is_pressed: bool,
    pub instant: Instant,
}

impl VKeyMessage {
    pub fn new(key: Key, is_pressed: bool, instant: Instant) -> Self {
        Self {
            key,
            is_pressed,
            instant,
        }
    }
}
