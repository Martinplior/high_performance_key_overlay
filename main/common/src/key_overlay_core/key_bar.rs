use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub struct KeyBar {
    pub press_instant: Instant,
    pub release_instant: Instant,
}

impl KeyBar {
    pub fn new(press_instant: Instant, release_instant: Instant) -> Self {
        Self {
            press_instant,
            release_instant,
        }
    }
}
