# MikroKit-Emu in a web browser

These files are the browser version of MikroKit-Emu, the emulator of the
Micro Design Mikrokit V2.0 8085 trainer. The same version runs at
<https://firew4lker.github.io/MikroKit-Emu/>.

| File | What it is |
|---|---|
| `index.html` | the page |
| `mikrokit-emu.js` | the loader |
| `mikrokit-emu_bg.wasm` | the emulator (WebAssembly) |
| `icon-64.png` | the browser-tab icon |

## Hosting it on your own server

Copy the files into any folder your web server serves (a subfolder is fine).
It is a static site: no server program, PHP or database.

- Serve `.wasm` as `application/wasm` (current nginx and Apache already
  do). With another type it still works, a little slower.
- Turn on gzip or brotli for `.wasm`, `.js` and `.html`: the download goes
  from about 8 MB to about 3 MB. nginx:
  `gzip on; gzip_types application/wasm application/javascript;`
- Use HTTPS if you can. Without it, Paste in the right-click menus can't
  read the clipboard (Ctrl+V still works).

Opening `index.html` straight from the disk (file://) doesn't work:
browsers only load WebAssembly from a web server. To try it locally:

```bash
python3 -m http.server 8000
```

then open <http://localhost:8000/>.

## Building it from the source

In the repository (<https://github.com/firew4lker/MikroKit-Emu>):

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock> --locked
web/build.sh      # writes web/dist; uses wasm-opt (binaryen) if installed
```

MikroKit-Emu is free software under the GNU GPL v3 or later (see LICENSE).
