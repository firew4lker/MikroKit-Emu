//! Monitor ROMs built into the emulator.

/// Micro Design Mikrokit monitor v5.12, the kit's original EPROM.
pub const ORIGINAL: &[u8] = include_bytes!("../roms/monitor-5.12.bin");
/// Monitor 5.12 plus Palo Alto Tiny BASIC 2.0 (roms/monitor-basic.asm).
pub const BASIC: &[u8] = include_bytes!("../roms/monitor-basic.bin");

/// A built-in ROM: (command-line name, menu label, image).
pub const BUILTIN: [(&str, &str, &[u8]); 2] = [
    ("original", "Original monitor 5.12", ORIGINAL),
    ("basic", "Monitor 5.12 + Tiny BASIC", BASIC),
];
