//! Desktop light/dark preference.
//!
//! winit 0.30 never reports the system theme on Linux (X11 or Wayland), so
//! egui falls back to dark even on a light desktop. On Linux we read the
//! freedesktop portal setting `org.freedesktop.appearance color-scheme`
//! (0 = no preference, 1 = dark, 2 = light) and keep listening for changes.
//! Other platforms report the theme through winit already.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

/// Latest known desktop preference: 0 unknown, 1 dark, 2 light.
#[derive(Clone, Default)]
pub struct SystemTheme(Arc<AtomicU8>);

impl SystemTheme {
    /// Start watching (Linux only; elsewhere this stays "unknown").
    pub fn watch(ctx: eframe::egui::Context) -> Self {
        let st = SystemTheme::default();
        #[cfg(target_os = "linux")]
        {
            let shared = st.0.clone();
            std::thread::Builder::new()
                .name("theme-watch".into())
                .spawn(move || linux::run(shared, ctx))
                .ok();
        }
        #[cfg(not(target_os = "linux"))]
        let _ = ctx;
        st
    }

    pub fn get(&self) -> Option<eframe::egui::Theme> {
        match self.0.load(Ordering::Relaxed) {
            1 => Some(eframe::egui::Theme::Dark),
            2 => Some(eframe::egui::Theme::Light),
            _ => None,
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use zbus::blocking::{Connection, Proxy};
    use zbus::zvariant::{OwnedValue, Value};

    const DEST: &str = "org.freedesktop.portal.Desktop";
    const PATH: &str = "/org/freedesktop/portal/desktop";
    const IFACE: &str = "org.freedesktop.portal.Settings";

    fn to_u8(v: &Value<'_>) -> Option<u8> {
        match v {
            Value::U32(n) => Some(*n as u8),
            Value::Value(inner) => to_u8(inner),
            _ => None,
        }
    }

    fn read(proxy: &Proxy<'_>) -> Option<u8> {
        // ReadOne is the current API; Read is the older one (double-wrapped)
        let v: OwnedValue = proxy
            .call("ReadOne", &("org.freedesktop.appearance", "color-scheme"))
            .or_else(|_| proxy.call("Read", &("org.freedesktop.appearance", "color-scheme")))
            .ok()?;
        to_u8(&v)
    }

    fn store(shared: &AtomicU8, ctx: &eframe::egui::Context, n: u8) {
        if std::env::var_os("MICROKIT_DEBUG").is_some() {
            eprintln!("desktop color-scheme = {n} (0 none, 1 dark, 2 light)");
        }
        shared.store(n, Ordering::Relaxed);
        ctx.request_repaint();
    }

    pub fn run(shared: Arc<AtomicU8>, ctx: eframe::egui::Context) {
        let Ok(conn) = Connection::session() else { return };
        let Ok(proxy) = Proxy::new(&conn, DEST, PATH, IFACE) else { return };
        if let Some(n) = read(&proxy) {
            store(&shared, &ctx, n);
        }
        let Ok(signals) = proxy.receive_signal("SettingChanged") else { return };
        for msg in signals {
            let Ok((ns, key, value)) = msg.body().deserialize::<(String, String, OwnedValue)>() else {
                continue;
            };
            if ns == "org.freedesktop.appearance" && key == "color-scheme" {
                if let Some(n) = to_u8(&value) {
                    store(&shared, &ctx, n);
                }
            }
        }
    }
}
