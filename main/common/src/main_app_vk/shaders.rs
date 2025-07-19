pub use key_bar::vs::{Direction, Property, ScreenSize};

pub mod key_bar {
    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            path: "./src/main_app_vk/key_bar/key_bar.vs"
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            path: "./src/main_app_vk/key_bar/key_bar.fs"
        }
    }
}

pub mod press_rect {
    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            path: "./src/main_app_vk/press_rect/press_rect.vs"
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            path: "./src/main_app_vk/press_rect/press_rect.fs"
        }
    }
}

pub mod static_overlay {
    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            path: "./src/main_app_vk/static_overlay/static_overlay.vs"
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            path: "./src/main_app_vk/static_overlay/static_overlay.fs"
        }
    }
}

pub mod frame {
    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            path: "./src/main_app_vk/static_overlay/frame.vs"
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            path: "./src/main_app_vk/static_overlay/frame.fs"
        }
    }
}

pub mod text {
    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            path: "./src/main_app_vk/static_overlay/text.vs"
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            path: "./src/main_app_vk/static_overlay/text.fs"
        }
    }
}

pub mod numbers {
    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            path: "./src/main_app_vk/numbers/numbers.vs"
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            path: "./src/main_app_vk/numbers/numbers.fs"
        }
    }
}
