use cfg_if::cfg_if;
pub mod app;
pub mod fileserv;

cfg_if! { if #[cfg(feature = "hydrate")] {
    use leptos::prelude::*;
    use wasm_bindgen::prelude::wasm_bindgen;
    use crate::app::*;

    #[wasm_bindgen]
    pub fn hydrate() {
        // initializes logging using the `log` crate
        _ = console_log::init_with_level(log::Level::Debug);
        console_error_panic_hook::set_once();

        // Updated for Leptos 0.7 - use hydrate_body instead of mount_to_body
        leptos::mount::hydrate_body(App);
    }
}}