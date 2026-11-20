//! The browser version: entry point, plus stand-ins for what a web page
//! can't do the desktop way (blocking file dialogs, the system clipboard).
//! Proof of concept: file loading and saving are not available yet.

use wasm_bindgen::JsCast;

/// Start the app on the page's `<canvas id="mikrokit">`.
pub fn start() {
    // Rust panics show up in the browser's console
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().expect("a window").document().expect("a document");
        let canvas = document
            .get_element_by_id("mikrokit")
            .expect("the page has <canvas id=\"mikrokit\">")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("#mikrokit is a <canvas>");
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(crate::app::App::new(cc, crate::app::Options::default())))),
            )
            .await;
        // replace the "Loading…" text, or show why it failed
        if let Some(el) = document.get_element_by_id("loading") {
            match result {
                Ok(()) => el.remove(),
                Err(e) => el.set_inner_html(&format!("MikroKit-Emu couldn't start: {e:?}")),
            }
        }
    });
}

/// File dialogs: none yet in the browser (they always come back "cancelled").
pub mod rfd {
    use std::path::PathBuf;

    #[derive(Default)]
    pub struct FileDialog;

    impl FileDialog {
        pub fn new() -> Self {
            FileDialog
        }
        pub fn set_title<T>(self, _: T) -> Self {
            self
        }
        pub fn set_file_name<T>(self, _: T) -> Self {
            self
        }
        pub fn add_filter<T, E>(self, _: T, _: &[E]) -> Self {
            self
        }
        pub fn pick_file(self) -> Option<PathBuf> {
            None
        }
        pub fn save_file(self) -> Option<PathBuf> {
            None
        }
    }
}

/// The system clipboard can't be read directly by a web page; Ctrl+V still
/// pastes through the browser.
pub mod arboard {
    pub struct Clipboard;

    impl Clipboard {
        pub fn new() -> Result<Self, String> {
            Err("use Ctrl+V in the browser".into())
        }
        pub fn get_text(&mut self) -> Result<String, String> {
            Err("use Ctrl+V in the browser".into())
        }
    }
}
