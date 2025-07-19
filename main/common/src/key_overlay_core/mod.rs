pub mod key_bar;
pub mod key_draw_cache;
pub mod key_handler;
pub mod key_message;
pub mod key_property;

use std::time::Instant;

use crate::{
    key_overlay_core::{key_handler::KeyHandler, key_message::KeyMessage},
    setting::Setting,
};

use sak_rs::sync::mpmc::queue::BoundedReceiver as MpscReceiver;

pub struct KeyOverlayCore {
    key_messages_buffer: Vec<KeyMessage>,
    keys_receiver: MpscReceiver<KeyMessage>,
    key_handler: KeyHandler,
}

impl KeyOverlayCore {
    const DEFAULT_BUFFER_CAPACITY: usize = 64;

    pub fn new(setting: Setting, keys_receiver: MpscReceiver<KeyMessage>) -> Self {
        Self {
            key_messages_buffer: Vec::with_capacity(Self::DEFAULT_BUFFER_CAPACITY),
            keys_receiver,
            key_handler: KeyHandler::new(setting),
        }
    }

    pub fn update(&mut self, instant_now: Instant) {
        self.key_messages_buffer
            .extend(self.keys_receiver.try_iter());
        self.key_messages_buffer.drain(..).for_each(|key_message| {
            self.key_handler.update(key_message);
        });
        self.key_handler.remove_outer_bar(instant_now);
    }

    pub fn reload(&mut self, setting: &Setting) {
        self.key_handler.reload(setting);
    }

    pub fn keys_receiver(&self) -> &MpscReceiver<KeyMessage> {
        &self.keys_receiver
    }

    pub fn key_handler(&self) -> &KeyHandler {
        &self.key_handler
    }

    pub fn need_repaint(&self) -> bool {
        self.key_handler.need_repaint()
    }
}
