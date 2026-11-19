//! User-interface language. The English text is the key; `tr` returns the
//! Greek text when Greek is selected. Strings with `{}` placeholders are
//! filled in order by `trf!`, so a translation keeps the same placeholders
//! in the same order.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Language {
    #[default]
    English,
    Greek,
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(lang: Language) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Language {
    if CURRENT.load(Ordering::Relaxed) == Language::Greek as u8 { Language::Greek } else { Language::English }
}

/// Fill the `{}` placeholders of a template in order.
pub fn fill(template: &str, args: &[String]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut args = args.iter();
    let mut rest = template;
    while let Some(i) = rest.find("{}") {
        out.push_str(&rest[..i]);
        out.push_str(args.next().map(String::as_str).unwrap_or(""));
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

/// `trf!("Loaded {} → {}", a, b)`: translate the template, then fill it.
macro_rules! trf {
    ($t:expr $(, $a:expr)* $(,)?) => {
        $crate::i18n::fill($crate::i18n::tr($t), &[$(($a).to_string()),*])
    };
}
pub(crate) use trf;

/// The text in the current language (the English text if there is no
/// translation).
pub fn tr(en: &'static str) -> &'static str {
    if current() == Language::English {
        return en;
    }
    greek(en).unwrap_or(en)
}

fn greek(en: &str) -> Option<&'static str> {
    GREEK.iter().find(|(e, _)| *e == en).map(|(_, g)| *g)
}

/// (English, Greek) pairs.
const GREEK: &[(&str, &str)] = &[
    // ----- menus
    ("File", "Αρχείο"),
    ("Load program…", "Φόρτωση προγράμματος…"),
    ("Reload last program", "Επαναφόρτωση τελευταίου προγράμματος"),
    ("Save memory range…", "Αποθήκευση περιοχής μνήμης…"),
    ("Monitor ROM", "ROM του monitor"),
    ("Quit", "Έξοδος"),
    ("Machine", "Μηχάνημα"),
    ("Pause", "Παύση"),
    ("Run", "Εκτέλεση"),
    ("Reset (RS)   F9", "Επαναφορά (RS)   F9"),
    ("Vector interrupt (VI)   F10", "Διακοπή (VI)   F10"),
    ("Turbo speed", "Ταχύτητα turbo"),
    ("Serial", "Σειριακή"),
    ("Send HEX file to DB9-1 (8251)…", "Αποστολή αρχείου HEX στη DB9-1 (8251)…"),
    ("Send HEX file to DB9-2 (SID)…", "Αποστολή αρχείου HEX στη DB9-2 (SID)…"),
    ("Type in capitals (the monitor needs them)", "Πληκτρολόγηση με κεφαλαία (τα θέλει το monitor)"),
    ("Clear terminal", "Καθαρισμός τερματικού"),
    ("Cancel pending input", "Ακύρωση εισόδου σε αναμονή"),
    ("Tape", "Κασέτα"),
    ("Show tape recorder", "Εμφάνιση κασετοφώνου"),
    ("Load WAV…", "Φόρτωση WAV…"),
    ("Save WAV…", "Αποθήκευση WAV…"),
    ("Debug", "Αποσφαλμάτωση"),
    ("Show I/O ports", "Εμφάνιση θυρών I/O"),
    ("Show debug panel", "Εμφάνιση πίνακα αποσφαλμάτωσης"),
    ("Set GO address after loading a program", "Ορισμός διεύθυνσης GO μετά τη φόρτωση προγράμματος"),
    ("View", "Προβολή"),
    ("Theme", "Θέμα"),
    ("Follow the system", "Όπως το σύστημα"),
    ("Dark (light text on dark)", "Σκούρο (ανοιχτό κείμενο σε σκούρο φόντο)"),
    ("Light (dark text on light)", "Ανοιχτό (σκούρο κείμενο σε ανοιχτό φόντο)"),
    ("Tape recorder", "Κασετόφωνο"),
    ("I/O ports", "Θύρες I/O"),
    ("Debug panel", "Πίνακας αποσφαλμάτωσης"),
    ("Help", "Βοήθεια"),
    ("Keys and usage", "Πλήκτρα και χρήση"),
    ("Language", "Γλώσσα"),
    ("Language: English", "Γλώσσα: Ελληνικά"),
    // ----- monitor ROM
    ("Original monitor 5.12", "Αρχικό monitor 5.12"),
    ("File: {}", "Αρχείο: {}"),
    ("Load from file…", "Φόρτωση από αρχείο…"),
    ("Load monitor ROM", "Φόρτωση ROM του monitor"),
    ("ROM image or source", "Εικόνα ROM ή πηγαίος κώδικας"),
    ("All files", "Όλα τα αρχεία"),
    ("Monitor ROM: {} (built in)", "ROM του monitor: {} (ενσωματωμένη)"),
    ("Unknown built-in ROM", "Άγνωστη ενσωματωμένη ROM"),
    ("file is {} bytes; the ROM socket holds 8 KB", "το αρχείο είναι {} byte· η θέση της ROM χωράει 8 KB"),
    ("Monitor ROM loaded from {}", "Η ROM του monitor φορτώθηκε από το {}"),
    ("Couldn't load the ROM", "Δεν φορτώθηκε η ROM"),
    // ----- loading and saving
    ("Couldn't load {}", "Δεν φορτώθηκε το {}"),
    ("Nothing to load", "Τίποτα για φόρτωση"),
    ("{} contains no bytes.", "Το {} δεν περιέχει κανένα byte."),
    ("Program overlaps the ROM", "Το πρόγραμμα πέφτει πάνω στη ROM"),
    ("Address {} is in the EPROM (0000–1FFF). Load user programs into RAM (2000–3FFF), or use File → Monitor ROM → Load from file… to replace the monitor.", "Η διεύθυνση {} είναι στην EPROM (0000–1FFF). Φορτώστε τα προγράμματα χρήστη στη RAM (2000–3FFF), ή χρησιμοποιήστε Αρχείο → ROM του monitor → Φόρτωση από αρχείο… για να αντικαταστήσετε το monitor."),
    ("Program doesn't fit", "Το πρόγραμμα δεν χωράει"),
    ("There's no memory at {}. RAM is 2000–3FFF (and 4000–40FF in the 8155).", "Δεν υπάρχει μνήμη στη διεύθυνση {}. Η RAM είναι 2000–3FFF (και 4000–40FF στο 8155)."),
    (" — warning: it writes into the monitor's area 4000–40FF", " — προσοχή: γράφει στην περιοχή του monitor 4000–40FF"),
    ("Loaded {} → {}", "Φορτώθηκε το {} → {}"),
    (", GO address {}", ", διεύθυνση GO {}"),
    ("Send file to DB9-{}", "Αποστολή αρχείου στη DB9-{}"),
    ("Intel HEX, 8085 source or BASIC text", "Intel HEX, πηγαίος κώδικας 8085 ή κείμενο BASIC"),
    ("Couldn't assemble", "Η συμβολομετάφραση απέτυχε"),
    ("Couldn't read the file", "Δεν διαβάστηκε το αρχείο"),
    ("Typing {} into the terminal…", "Πληκτρολόγηση του {} στο τερματικό…"),
    ("Note: J18 is set to tape, so DB9-2 isn't connected to SID.", "Σημείωση: ο J18 είναι στην κασέτα, οπότε η DB9-2 δεν συνδέεται στο SID."),
    ("Sending {} ({} bytes) to DB9-{}…", "Αποστολή του {} ({} byte) στη DB9-{}…"),
    ("Wrong range", "Λάθος περιοχή"),
    ("The end address is before the start address.", "Η διεύθυνση τέλους είναι πριν από τη διεύθυνση αρχής."),
    ("Save memory", "Αποθήκευση μνήμης"),
    ("Binary", "Δυαδικό"),
    ("Saved {}–{} to {}", "Η περιοχή {}–{} αποθηκεύτηκε στο {}"),
    ("Couldn't save", "Η αποθήκευση απέτυχε"),
    ("Stopped at breakpoint {}", "Σταμάτησε στο σημείο διακοπής {}"),
    ("J16 changed — press RS (reset) for the monitor to switch.", "Ο J16 άλλαξε — πατήστε RS (επαναφορά) για να αλλάξει το monitor."),
    ("Load program into RAM", "Φόρτωση προγράμματος στη RAM"),
    ("Programs", "Προγράμματα"),
    // ----- toolbar
    ("⏸ Pause", "⏸ Παύση"),
    ("Pause the CPU", "Παύση της CPU"),
    ("▶ Run", "▶ Εκτέλεση"),
    ("Resume the CPU", "Συνέχιση της CPU"),
    ("⟲ Reset (RS)", "⟲ Επαναφορά (RS)"),
    ("Same as the RS key (F9)", "Όπως το πλήκτρο RS (F9)"),
    ("📂 Load program", "📂 Φόρτωση"),
    (".hex, .bin or .asm into RAM", ".hex, .bin ή .asm στη RAM"),
    ("↻ Reload", "↻ Επαναφόρτωση"),
    ("Speed", "Ταχύτητα"),
    ("3.072 MHz (real)", "3.072 MHz (πραγματική)"),
    ("📼 Tape", "📼 Κασέτα"),
    ("🔌 I/O ports", "🔌 Θύρες I/O"),
    // ----- board
    ("address", "διεύθυνση"),
    ("data", "δεδομένα"),
    ("MICROKIT V2.0 · click here, then type: 0-F, M R G S N . (Enter = EX)", "MICROKIT V2.0 · κλικ εδώ και πληκτρολογήστε: 0-F, M R G S N . (Enter = EX)"),
    ("RS: reset (F9)", "RS: επαναφορά (F9)"),
    ("VI: vector interrupt, RST 7.5 (F10)", "VI: διακοπή, RST 7.5 (F10)"),
    ("SS: single step (S)", "SS: βηματική εκτέλεση (S)"),
    ("GO: execute program (G)", "GO: εκτέλεση προγράμματος (G)"),
    ("M: examine/modify memory (M); inside M it steps back", "M: εξέταση/αλλαγή μνήμης (M)· μέσα στο M πηγαίνει πίσω"),
    ("R: examine/modify registers (R)", "R: εξέταση/αλλαγή καταχωρητών (R)"),
    ("N: next (N or Space)", "N: επόμενο (N ή Space)"),
    ("EX: execute / end (. or Enter)", "EX: εκτέλεση / τέλος (. ή Enter)"),
    ("hex digit", "δεκαεξαδικό ψηφίο"),
    // ----- terminal
    ("Clear", "Καθαρισμός"),
    ("Send HEX → DB9-2", "Αποστολή HEX → DB9-2"),
    ("Send HEX → DB9-1", "Αποστολή HEX → DB9-1"),
    ("Software serial on SID: the default port of GO 1600 and L", "Σειριακή με λογισμικό στο SID: η προεπιλεγμένη θύρα των GO 1600 και L"),
    ("The 8251 port (select it with P → 1, or GO 1800 → 1)", "Η θύρα του 8251 (επιλέγεται με P → 1 ή GO 1800 → 1)"),
    ("Terminal — DB9-1 · {} baud", "Τερματικό — DB9-1 · {} baud"),
    (" · sending {} bytes", " · αποστολή {} byte"),
    (" · J16 off: keypad monitor active", " · J16 off: ενεργό το monitor πληκτρολογίου"),
    ("Selection copied", "Η επιλογή αντιγράφηκε"),
    ("Terminal text copied", "Το κείμενο του τερματικού αντιγράφηκε"),
    ("Nothing to paste ({})", "Τίποτα για επικόλληση ({})"),
    ("Copy", "Αντιγραφή"),
    ("Copy all text", "Αντιγραφή όλου του κειμένου"),
    ("Paste", "Επικόλληση"),
    ("Send Ctrl+C", "Αποστολή Ctrl+C"),
    // ----- status bar and jumpers
    ("Jumpers", "Βραχυκυκλωτήρες"),
    ("J16 terminal mode", "J16 τερματικό"),
    ("ON: terminal (CRT) monitor on DB9-1. OFF: keypad monitor. Press RS after changing.", "ON: monitor τερματικού (CRT) στη DB9-1. OFF: monitor πληκτρολογίου. Πατήστε RS μετά την αλλαγή."),
    ("J13 single step", "J13 βηματική εκτέλεση"),
    ("8155 timer drives TRAP (needed for SS and T)", "Ο χρονιστής του 8155 οδηγεί το TRAP (χρειάζεται για SS και T)"),
    ("J18 SID: tape", "J18 SID: κασέτα"),
    ("Route SID/SOD to the DB9-2 serial port or to the tape connector", "Σύνδεση των SID/SOD στη σειριακή θύρα DB9-2 ή στον ακροδέκτη της κασέτας"),
    ("pos {}: {} baud", "θέση {}: {} baud"),
    ("Baud rate of DB9-1 (8251 in x16 mode)", "Ταχύτητα της DB9-1 (8251 σε λειτουργία x16)"),
    ("running", "σε λειτουργία"),
    ("paused", "σε παύση"),
    ("stopped", "σταματημένος"),
    ("CPU registers (updated 4 times a second while running)", "Καταχωρητές της CPU (ενημερώνονται 4 φορές το δευτερόλεπτο όσο τρέχει)"),
    ("J16 terminal monitor (press RS after)", "J16 monitor τερματικού (μετά πατήστε RS)"),
    ("J13 timer → TRAP (single step)", "J13 χρονιστής → TRAP (βηματική εκτέλεση)"),
    ("J18 SID/SOD on DB9-2 (off = tape)", "J18 SID/SOD στη DB9-2 (off = κασέτα)"),
    ("J14 8155 RAM enabled", "J14 ενεργή η RAM του 8155"),
    // ----- debug panel
    ("Step", "Βήμα"),
    ("Step ×100", "Βήμα ×100"),
    ("Disassembly", "Αποσυμβολομετάφραση"),
    ("Breakpoints", "Σημεία διακοπής"),
    ("Add", "Προσθήκη"),
    ("Memory", "Μνήμη"),
    ("Monitor's saved user registers", "Αποθηκευμένοι καταχωρητές χρήστη (monitor)"),
    ("timer", "χρονιστής"),
    // ----- tape
    ("Load tape (WAV)", "Φόρτωση κασέτας (WAV)"),
    ("WAV audio", "Ήχος WAV"),
    ("Tape loaded: {} ({} s)", "Η κασέτα φορτώθηκε: {} ({} s)"),
    ("Couldn't load the tape", "Δεν φορτώθηκε η κασέτα"),
    ("Save tape as WAV", "Αποθήκευση κασέτας ως WAV"),
    ("Tape saved to {}", "Η κασέτα αποθηκεύτηκε στο {}"),
    ("Couldn't save the tape", "Η αποθήκευση της κασέτας απέτυχε"),
    ("J18 routes SID/SOD to DB9-2, so the recorder isn't connected.", "Ο J18 συνδέει τα SID/SOD στη DB9-2, οπότε το κασετόφωνο δεν είναι συνδεδεμένο."),
    ("Set J18 to tape", "J18 στην κασέτα"),
    ("(blank tape)", "(κενή κασέτα)"),
    (" • unsaved", " • δεν έχει αποθηκευτεί"),
    ("empty — press Record to record", "κενή — πατήστε Εγγραφή για να γράψετε"),
    ("Rewind", "Γύρισμα στην αρχή"),
    ("▶ Play", "▶ Αναπαραγωγή"),
    ("⏺ Record", "⏺ Εγγραφή"),
    ("Records from the current position, over anything already there", "Γράφει από την τρέχουσα θέση, πάνω σε ό,τι υπάρχει ήδη"),
    ("⏹ Stop", "⏹ Διακοπή"),
    ("Blank tape", "Σβήσιμο κασέτας"),
    ("Write: GO 1000 EX (keypad) or W (terminal); press Record, then the last EX / Return.\nRead: rewind, GO 1300 EX or R; press Play, then the last EX / Return.", "Εγγραφή: GO 1000 EX (πληκτρολόγιο) ή W (τερματικό)· πατήστε Εγγραφή και μετά το τελευταίο EX / Return.\nΑνάγνωση: γυρίστε στην αρχή, GO 1300 EX ή R· πατήστε Αναπαραγωγή και μετά το τελευταίο EX / Return."),
    ("📼 Tape recorder", "📼 Κασετόφωνο"),
    ("Erase the tape?", "Σβήσιμο της κασέτας;"),
    ("Replace the tape?", "Αντικατάσταση της κασέτας;"),
    ("The current tape has a recording you haven't saved.", "Η κασέτα έχει εγγραφή που δεν έχετε αποθηκεύσει."),
    ("Discard recording", "Απόρριψη της εγγραφής"),
    // ----- I/O ports
    ("input (ALT1)", "είσοδος (ALT1)"),
    ("output (ALT2)", "έξοδος (ALT2)"),
    ("handshake for A (ALT3)", "χειραψία για την A (ALT3)"),
    ("handshake for A and B (ALT4)", "χειραψία για τις A και B (ALT4)"),
    ("8155 (IC12) — connector X2", "8155 (IC12) — ακροδέκτης X2"),
    ("out", "έξοδος"),
    ("in", "είσοδος"),
    ("Outputs are LEDs; inputs are switches you can click.", "Οι έξοδοι είναι LED· οι είσοδοι είναι διακόπτες που αλλάζουν με κλικ."),
    ("Input ports (IC21)", "Θύρες εισόδου (IC21)"),
    ("Connector X3, pins 1-3 → D0-D2", "Ακροδέκτης X3, ακίδες 1-3 → D0-D2"),
    ("X3 pins 1-3", "X3 ακίδες 1-3"),
    ("D0 = J16 (0 = terminal monitor)", "D0 = J16 (0 = monitor τερματικού)"),
    ("single square wave", "μονός τετραγωνικός παλμός"),
    ("square wave", "τετραγωνικό κύμα"),
    ("single pulse", "μονός παλμός"),
    ("continuous pulses", "συνεχείς παλμοί"),
    ("Timer: count {} ({}H), {}, {} → TRAP via J13", "Χρονιστής: μέτρηση {} ({}H), {}, {} → TRAP μέσω J13"),
    ("1 — click to set 0", "1 — κλικ για 0"),
    ("0 — click to set 1", "0 — κλικ για 1"),
    // ----- dialogs
    ("Cancel", "Άκυρο"),
    ("Load binary", "Φόρτωση δυαδικού αρχείου"),
    ("{} ({} bytes)", "{} ({} byte)"),
    ("Load address (hex):", "Διεύθυνση φόρτωσης (hex):"),
    ("Load", "Φόρτωση"),
    ("Enter a hex address like 2000", "Δώστε μια δεκαεξαδική διεύθυνση, π.χ. 2000"),
    ("Save memory range", "Αποθήκευση περιοχής μνήμης"),
    ("From", "Από"),
    ("to", "έως"),
    ("Save…", "Αποθήκευση…"),
    ("Enter hex addresses like 2000 and 20FF", "Δώστε δεκαεξαδικές διευθύνσεις, π.χ. 2000 και 20FF"),
    ("MicroKit-Emu — keys and usage", "MicroKit-Emu — πλήκτρα και χρήση"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_in_order() {
        assert_eq!(fill("{} ({} bytes)", &["a.bin".into(), "12".into()]), "a.bin (12 bytes)");
        assert_eq!(fill("no args", &[]), "no args");
    }

    #[test]
    fn translations_keep_placeholders() {
        for (en, el) in GREEK {
            assert_eq!(en.matches("{}").count(), el.matches("{}").count(), "{en}");
            assert_eq!(en.ends_with('\n'), el.ends_with('\n'), "{en}");
        }
        let mut keys: Vec<_> = GREEK.iter().map(|(e, _)| e).collect();
        keys.sort();
        let n = keys.len();
        keys.dedup();
        assert_eq!(n, keys.len(), "duplicate English keys");
    }
}
