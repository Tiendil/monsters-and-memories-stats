fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let root = leptos::prelude::document()
            .get_element_by_id("app")
            .expect("dashboard mount element");
        root.set_inner_html("");
        leptos::mount::mount_to(root.unchecked_into(), mnm_stats_dashboard::app::App).forget();
    }
}
