//! Syntax highlighting of 8085 assembly for the editor.

use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{Color32, FontId};

const MNEMONICS: &[&str] = &[
    "ACI", "ADC", "ADD", "ADI", "ANA", "ANI", "ARHL", "CALL", "CC", "CM", "CMA", "CMC", "CMP", "CNC", "CNZ", "CP",
    "CPE", "CPI", "CPO", "CZ", "DAA", "DAD", "DCR", "DCX", "DI", "DSUB", "EI", "HLT", "IN", "INR", "INX", "JC", "JK",
    "JM", "JMP", "JNC", "JNK", "JNZ", "JP", "JPE", "JPO", "JZ", "LDA", "LDAX", "LDHI", "LDSI", "LHLD", "LHLX", "LXI",
    "MOV", "MVI", "NOP", "ORA", "ORI", "OUT", "PCHL", "POP", "PUSH", "RAL", "RAR", "RC", "RDEL", "RET", "RIM", "RLC",
    "RM", "RNC", "RNZ", "RP", "RPE", "RPO", "RRC", "RST", "RSTV", "RZ", "SBB", "SBI", "SHLD", "SHLX", "SIM", "SPHL",
    "STA", "STAX", "STC", "SUB", "SUI", "XCHG", "XRA", "XRI", "XTHL",
];
const DIRECTIVES: &[&str] = &["ORG", "EQU", "SET", "DB", "DW", "DS", "DEFB", "DEFW", "DEFS", "DM", "TEXT", "END"];
const OPERATORS: &[&str] = &["MOD", "SHL", "SHR", "AND", "OR", "XOR", "NOT", "HIGH", "LOW"];
const REGISTERS: &[&str] = &["A", "B", "C", "D", "E", "H", "L", "M", "SP", "PSW"];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Plain,
    Comment,
    Mnemonic,
    Directive,
    Register,
    Number,
    String,
    Label,
    Operator,
}

pub struct Palette {
    pub plain: Color32,
    pub comment: Color32,
    pub mnemonic: Color32,
    pub directive: Color32,
    pub register: Color32,
    pub number: Color32,
    pub string: Color32,
    pub label: Color32,
    pub operator: Color32,
    pub error_bg: Color32,
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        let rgb = Color32::from_rgb;
        if dark {
            Palette {
                plain: rgb(0xd8, 0xd8, 0xd2),
                comment: rgb(0x7f, 0x8f, 0x86),
                mnemonic: rgb(0x6c, 0xb6, 0xff),
                directive: rgb(0xd1, 0x8f, 0xe8),
                register: rgb(0xff, 0xb8, 0x6c),
                number: rgb(0x9f, 0xe1, 0x8a),
                string: rgb(0xe6, 0xd2, 0x7a),
                label: rgb(0xff, 0x8a, 0x8a),
                operator: rgb(0xd1, 0x8f, 0xe8),
                error_bg: Color32::from_rgba_unmultiplied(0xc0, 0x30, 0x30, 70),
            }
        } else {
            Palette {
                plain: rgb(0x1a, 0x1a, 0x18),
                comment: rgb(0x6a, 0x73, 0x6e),
                mnemonic: rgb(0x0b, 0x4f, 0xa8),
                directive: rgb(0x7a, 0x2e, 0x9a),
                register: rgb(0xa8, 0x4a, 0x00),
                number: rgb(0x1d, 0x7a, 0x2e),
                string: rgb(0x8a, 0x6a, 0x00),
                label: rgb(0xb4, 0x14, 0x2a),
                operator: rgb(0x7a, 0x2e, 0x9a),
                error_bg: Color32::from_rgba_unmultiplied(0xe0, 0x40, 0x40, 60),
            }
        }
    }

    fn color(&self, k: Kind) -> Color32 {
        match k {
            Kind::Plain => self.plain,
            Kind::Comment => self.comment,
            Kind::Mnemonic => self.mnemonic,
            Kind::Directive => self.directive,
            Kind::Register => self.register,
            Kind::Number => self.number,
            Kind::String => self.string,
            Kind::Label => self.label,
            Kind::Operator => self.operator,
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c, '_' | '.' | '?' | '@')
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '?' | '@' | '$')
}

/// Split one line into (byte range, kind).
pub fn tokens(line: &str) -> Vec<(std::ops::Range<usize>, Kind)> {
    let mut out = Vec::new();
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let end_of = |i: usize| chars.get(i).map(|c| c.0).unwrap_or(line.len());
    let mut i = 0;
    let mut words = 0; // words seen so far in this statement (label, mnemonic, operands)
    let mut after_mnemonic = false;
    while i < chars.len() {
        let (start, c) = chars[i];
        if c == ';' {
            out.push((start..line.len(), Kind::Comment));
            break;
        }
        if c == '\'' || c == '"' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].1 != c {
                j += 1;
            }
            j = (j + 1).min(chars.len());
            out.push((start..end_of(j), Kind::String));
            i = j;
            continue;
        }
        let number_start = c.is_ascii_digit()
            || (c == '$' && chars.get(i + 1).is_some_and(|n| n.1.is_ascii_hexdigit()));
        if number_start {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].1.is_ascii_alphanumeric()) {
                j += 1;
            }
            out.push((start..end_of(j), Kind::Number));
            words += 1;
            i = j;
            continue;
        }
        if is_ident_start(c) {
            let mut j = i + 1;
            while j < chars.len() && is_ident(chars[j].1) {
                j += 1;
            }
            let word = &line[start..end_of(j)];
            let upper = word.to_ascii_uppercase();
            let colon = chars.get(j).is_some_and(|n| n.1 == ':');
            let kind = if words == 0 && (colon || (start == 0 && !MNEMONICS.contains(&upper.as_str()) && !DIRECTIVES.contains(&upper.as_str()))) {
                Kind::Label
            } else if !after_mnemonic && MNEMONICS.contains(&upper.as_str()) {
                after_mnemonic = true;
                Kind::Mnemonic
            } else if DIRECTIVES.contains(&upper.as_str()) {
                after_mnemonic = true;
                Kind::Directive
            } else if OPERATORS.contains(&upper.as_str()) {
                Kind::Operator
            } else if after_mnemonic && REGISTERS.contains(&upper.as_str()) {
                Kind::Register
            } else {
                Kind::Plain
            };
            out.push((start..end_of(j), kind));
            words += 1;
            i = j;
            continue;
        }
        out.push((start..end_of(i + 1), Kind::Plain));
        i += 1;
    }
    out
}

/// The whole text as a coloured layout; `error_line` (1-based) gets a red background.
pub fn layout(text: &str, font: FontId, pal: &Palette, error_line: Option<usize>) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (n, line) in text.split_inclusive('\n').enumerate() {
        let bg = if error_line == Some(n + 1) { pal.error_bg } else { Color32::TRANSPARENT };
        let body = line.strip_suffix('\n').unwrap_or(line);
        let mut pos = 0;
        for (range, kind) in tokens(body) {
            if range.start > pos {
                job.append(&body[pos..range.start], 0.0, fmt(&font, pal.plain, bg));
            }
            job.append(&body[range.clone()], 0.0, fmt(&font, pal.color(kind), bg));
            pos = range.end;
        }
        if pos < body.len() {
            job.append(&body[pos..], 0.0, fmt(&font, pal.plain, bg));
        }
        if line.ends_with('\n') {
            job.append("\n", 0.0, fmt(&font, pal.plain, bg));
        }
    }
    job
}

fn fmt(font: &FontId, color: Color32, background: Color32) -> TextFormat {
    TextFormat { font_id: font.clone(), color, background, ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str) -> Vec<(String, Kind)> {
        tokens(line)
            .into_iter()
            .filter(|(_, k)| *k != Kind::Plain)
            .map(|(r, k)| (line[r].to_string(), k))
            .collect()
    }

    #[test]
    fn a_typical_line() {
        assert_eq!(
            kinds("LOOP:   MOV A,M      ; get a byte"),
            vec![
                ("LOOP".into(), Kind::Label),
                ("MOV".into(), Kind::Mnemonic),
                ("A".into(), Kind::Register),
                ("M".into(), Kind::Register),
                ("; get a byte".into(), Kind::Comment),
            ]
        );
    }

    #[test]
    fn directives_numbers_strings() {
        assert_eq!(
            kinds("MSG     DB \"Hi; there\", 0DH, 0"),
            vec![
                ("MSG".into(), Kind::Label),
                ("DB".into(), Kind::Directive),
                ("\"Hi; there\"".into(), Kind::String),
                ("0DH".into(), Kind::Number),
                ("0".into(), Kind::Number),
            ]
        );
        assert_eq!(
            kinds("        ORG 2000H"),
            vec![("ORG".into(), Kind::Directive), ("2000H".into(), Kind::Number)]
        );
        // a label without a colon at the start of the line, then EQU
        assert_eq!(kinds("CSR EQU 40H")[0], ("CSR".into(), Kind::Label));
        // a register name used as a symbol before the mnemonic isn't a register
        assert_eq!(kinds("        LXI H,TABLE+2")[1], ("H".into(), Kind::Register));
    }
}
