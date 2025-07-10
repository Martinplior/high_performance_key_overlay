use std::time::Instant;

use crate::{
    key::Key,
    key_overlay_core::{
        key_bar::KeyBar, key_draw_cache::KeyDrawCache, key_message::KeyMessage,
        key_property::KeyProperty,
    },
    setting::Setting,
};

#[derive(Debug)]
struct KeyMap {
    map: Box<[Option<Box<[usize]>>; Self::CAP]>,
}

impl KeyMap {
    const CAP: usize = Key::LAST_KEY as usize;

    fn new(key_properties: &[KeyProperty]) -> Self {
        let init = || -> Option<_> {
            let mut init_map: Box<[_; Self::CAP]> = Box::new(std::array::from_fn(|_| Some(vec![])));
            let iter = key_properties
                .iter()
                .filter(|key_property| key_property.key_bind != Key::Unknown)
                .enumerate();
            for (index, key_property) in iter {
                let indexes = init_map.get_mut(key_property.key_bind as usize)?.as_mut()?;
                indexes.push(index);
            }
            let map = Box::new(std::array::from_fn(|index| {
                let init_vec = init_map
                    .get_mut(index)
                    .expect("unreachable")
                    .take()
                    .expect("unreachable");
                if init_vec.is_empty() {
                    None
                } else {
                    Some(init_vec.into_boxed_slice())
                }
            }));
            Some(map)
        };

        let map = init().expect("unreachable");

        Self { map }
    }

    fn get(&self, key: Key) -> Option<&[usize]> {
        debug_assert!(key != Key::Unknown);
        unsafe { self.map.get_unchecked(key as usize) }.as_deref()
    }
}

pub struct KeyHandler {
    key_properties: Box<[KeyProperty]>,
    key_draw_caches: Box<[KeyDrawCache]>,
    key_map: KeyMap,
}

impl KeyHandler {
    pub fn new(setting: Setting) -> Self {
        let Setting {
            window_setting,
            key_properties,
            ..
        } = setting;
        let key_properties = key_properties.into_boxed_slice();
        let key_map = KeyMap::new(&key_properties);
        let key_draw_caches = key_properties
            .iter()
            .map(|key_property| {
                KeyDrawCache::new(&window_setting, key_property.bar_speed, key_property)
            })
            .collect();
        Self {
            key_properties,
            key_draw_caches,
            key_map,
        }
    }

    pub fn reload(&mut self, setting: &Setting) {
        let Setting {
            window_setting,
            key_properties,
            ..
        } = setting;
        let new_key_properties = key_properties.clone().into_boxed_slice();
        let new_key_map = KeyMap::new(&new_key_properties);
        let new_key_draw_caches = new_key_properties
            .iter()
            .map(|key_property| {
                KeyDrawCache::new(window_setting, key_property.bar_speed, key_property)
            })
            .collect();

        self.key_properties = new_key_properties;
        self.key_map = new_key_map;
        self.key_draw_caches = new_key_draw_caches;
    }

    pub fn update(&mut self, key_message: KeyMessage) {
        debug_assert!(key_message.key != Key::Unknown);

        let mut inner_update = |indexes: &[usize]| -> Option<()> {
            let first_key_draw_cache = self.key_draw_caches.get_mut(*indexes.first()?)?;

            let now_pressed = key_message.is_pressed;
            let prev_pressed = first_key_draw_cache.begin_hold_instant.is_some();

            match (prev_pressed, now_pressed) {
                (false, true) => {
                    for index in indexes.iter() {
                        let key_property = self.key_properties.get_mut(*index)?;
                        let key_draw_cache = self.key_draw_caches.get_mut(*index)?;
                        if key_property.key_counter.0 {
                            key_draw_cache.increase_count();
                        }
                        key_draw_cache.begin_hold_instant = Some(key_message.instant);
                    }
                }
                (true, false) => {
                    for index in indexes.iter() {
                        let key_draw_cache = self.key_draw_caches.get_mut(*index)?;
                        let bar = KeyBar::new(
                            key_draw_cache.begin_hold_instant.take()?,
                            key_message.instant,
                        );
                        key_draw_cache.add_bar(bar);
                    }
                }
                _ => (),
            }
            Some(())
        };

        self.key_map
            .get(key_message.key)
            .map(|indexes| unsafe { inner_update(indexes).unwrap_unchecked() });
    }

    pub fn remove_outer_bar(&mut self, instant_now: Instant) {
        self.key_draw_caches.iter_mut().for_each(|key_draw_cache| {
            key_draw_cache.remove_outer_bar(instant_now);
        });
    }

    pub fn key_properties(&self) -> &[KeyProperty] {
        &self.key_properties
    }

    pub fn key_draw_caches(&self) -> &[KeyDrawCache] {
        &self.key_draw_caches
    }

    pub fn need_repaint(&self) -> bool {
        self.key_draw_caches
            .iter()
            .any(|key_draw_cache| key_draw_cache.need_repaint())
    }
}
