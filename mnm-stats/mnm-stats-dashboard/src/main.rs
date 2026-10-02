fn main() {
    #[cfg(target_arch = "wasm32")]
    leptos::mount::mount_to_body(mnm_stats_dashboard::app::App);
}
