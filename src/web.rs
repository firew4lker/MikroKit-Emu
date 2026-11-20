//! The browser version: entry point, plus what a web page does differently
//! from the desktop: files are uploaded (Open) and downloaded (Save), and
//! the clipboard is read asynchronously.

use crate::app::{PasteTo, Pick};
use std::cell::RefCell;
use std::path::PathBuf;
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

// ----- results that arrive later ---------------------------------------------

/// Something the browser delivered after the frame that asked for it.
pub enum Event {
    Picked(Pick, PathBuf),
    Pasted(PasteTo, Result<String, String>),
}

thread_local! {
    static EVENTS: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
    static CONTEXT: RefCell<Option<eframe::egui::Context>> = const { RefCell::new(None) };
}

pub fn set_context(ctx: eframe::egui::Context) {
    CONTEXT.with(|c| *c.borrow_mut() = Some(ctx));
}

fn push(ev: Event) {
    EVENTS.with(|e| e.borrow_mut().push(ev));
    CONTEXT.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.request_repaint();
        }
    });
}

pub fn take_events() -> Vec<Event> {
    EVENTS.with(|e| std::mem::take(&mut *e.borrow_mut()))
}

// ----- files -----------------------------------------------------------------

/// The desktop's file dialog, as far as the app uses it. In the browser it
/// only collects the settings: `pick` opens the upload dialog, and a save
/// "chooses" the suggested name, which `fs::write` then downloads.
#[derive(Default)]
pub struct FileDialog {
    filters: Vec<(String, Vec<String>)>,
    file_name: Option<String>,
}

pub mod rfd {
    pub use super::FileDialog;
}

impl FileDialog {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set_title(self, _: impl Into<String>) -> Self {
        self
    }
    pub fn set_file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }
    pub fn add_filter(mut self, name: impl Into<String>, exts: &[impl ToString]) -> Self {
        let exts: Vec<String> = exts.iter().map(|e| e.to_string()).filter(|e| e != "*").collect();
        if !exts.is_empty() {
            self.filters.push((name.into(), exts));
        }
        self
    }
    pub fn save_file(self) -> Option<PathBuf> {
        Some(PathBuf::from(self.file_name.unwrap_or_else(|| "download".into())))
    }
}

/// Open the browser's upload dialog; the file arrives as `Event::Picked`
/// and is then readable through `fs` under its name.
pub fn pick(what: Pick, dialog: FileDialog) {
    wasm_bindgen_futures::spawn_local(async move {
        let mut d = ::rfd::AsyncFileDialog::new();
        for (name, exts) in &dialog.filters {
            d = d.add_filter(name.as_str(), exts);
        }
        if let Some(h) = d.pick_file().await {
            let name = h.file_name();
            let bytes = h.read().await;
            fs::insert(&name, bytes);
            push(Event::Picked(what, PathBuf::from(name)));
        }
    });
}

/// A file dropped on the page: read it, then load it as a program.
pub fn dropped(f: eframe::egui::DroppedFileHandle) {
    wasm_bindgen_futures::spawn_local(async move {
        let name = f.path().to_string_lossy().into_owned();
        if let Ok(bytes) = f.bytes_async().await {
            fs::insert(&name, bytes);
            push(Event::Picked(Pick::Program, PathBuf::from(name)));
        }
    });
}

/// `std::fs` for the browser: reads come from the files uploaded in this
/// session, writes become downloads.
pub mod fs {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io;
    use std::path::Path;
    use wasm_bindgen::JsCast;

    thread_local! {
        static FILES: RefCell<HashMap<String, Vec<u8>>> = RefCell::new(HashMap::new());
    }

    fn key(p: &Path) -> String {
        p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    pub fn insert(name: &str, bytes: Vec<u8>) {
        FILES.with(|f| f.borrow_mut().insert(key(Path::new(name)), bytes));
    }

    pub fn read(p: impl AsRef<Path>) -> io::Result<Vec<u8>> {
        let k = key(p.as_ref());
        FILES.with(|f| f.borrow().get(&k).cloned()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{k} is no longer loaded in this browser tab; open it again"),
            )
        })
    }

    pub fn read_to_string(p: impl AsRef<Path>) -> io::Result<String> {
        String::from_utf8(read(p)?).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Offer the data as a download named after the path.
    pub fn write(p: impl AsRef<Path>, data: impl AsRef<[u8]>) -> io::Result<()> {
        let name = key(p.as_ref());
        let data = data.as_ref();
        insert(&name, data.to_vec());
        download(&name, data).map_err(|e| io::Error::other(format!("{e:?}")))
    }

    fn download(name: &str, data: &[u8]) -> Result<(), wasm_bindgen::JsValue> {
        let array = js_sys::Uint8Array::from(data);
        let parts = js_sys::Array::of1(&array);
        let opts = web_sys::BlobPropertyBag::new();
        opts.set_type("application/octet-stream");
        let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob)?;
        let document = web_sys::window().unwrap().document().unwrap();
        let a = document.create_element("a")?.dyn_into::<web_sys::HtmlAnchorElement>()?;
        a.set_href(&url);
        a.set_download(name);
        a.click();
        web_sys::Url::revoke_object_url(&url)?;
        Ok(())
    }
}

// ----- clipboard ---------------------------------------------------------------

/// Ask the browser for the clipboard's text (it may ask the user first);
/// it arrives as `Event::Pasted`.
pub fn read_clipboard(to: PasteTo) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = async {
            let clipboard = web_sys::window().ok_or("no window")?.navigator().clipboard();
            let text = wasm_bindgen_futures::JsFuture::from(clipboard.read_text())
                .await
                .map_err(|_| "the browser didn't allow reading the clipboard; use Ctrl+V")?;
            text.as_string().ok_or("the clipboard has no text")
        }
        .await
        .map_err(str::to_string);
        push(Event::Pasted(to, result));
    });
}
