//! A small two-pass Intel-syntax 8085 assembler.
//!
//! Supports labels (with or without ':' in column 0), EQU/SET, ORG, DB/DW/DS
//! (and DEFB/DEFW/DEFS), END, strings, character constants and expressions
//! with + - * / MOD SHL SHR AND OR XOR NOT HIGH LOW and C-style operators.
//! Numbers: 0FFH, 1010B, 17O/17Q, 123D, 123, 0x1F; `$` is the current address.

use crate::loader::Image;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AsmError {
    pub line: usize,
    pub msg: String,
}

impl std::fmt::Display for AsmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)
    }
}

pub struct Output {
    pub image: Image,
    pub symbols: HashMap<String, u16>,
}

const DIRECTIVES: &[&str] =
    &["ORG", "EQU", "SET", "DB", "DW", "DS", "DEFB", "DEFW", "DEFS", "END", ".ORG", ".DB", ".DW", ".DS", ".END"];

fn reg8(s: &str) -> Option<u8> {
    Some(match s.to_ascii_uppercase().as_str() {
        "B" => 0,
        "C" => 1,
        "D" => 2,
        "E" => 3,
        "H" => 4,
        "L" => 5,
        "M" => 6,
        "A" => 7,
        _ => return None,
    })
}

fn regpair(s: &str, psw: bool) -> Option<u8> {
    Some(match s.to_ascii_uppercase().as_str() {
        "B" | "BC" => 0,
        "D" | "DE" => 1,
        "H" | "HL" => 2,
        "SP" if !psw => 3,
        "PSW" if psw => 3,
        _ => return None,
    })
}

fn implied(m: &str) -> Option<u8> {
    Some(match m {
        "NOP" => 0x00,
        "RLC" => 0x07,
        "RRC" => 0x0F,
        "RAL" => 0x17,
        "RAR" => 0x1F,
        "RIM" => 0x20,
        "DAA" => 0x27,
        "CMA" => 0x2F,
        "SIM" => 0x30,
        "STC" => 0x37,
        "CMC" => 0x3F,
        "HLT" => 0x76,
        "RNZ" => 0xC0,
        "RZ" => 0xC8,
        "RET" => 0xC9,
        "RNC" => 0xD0,
        "RC" => 0xD8,
        "RPO" => 0xE0,
        "XTHL" => 0xE3,
        "RPE" => 0xE8,
        "PCHL" => 0xE9,
        "XCHG" => 0xEB,
        "RP" => 0xF0,
        "DI" => 0xF3,
        "RM" => 0xF8,
        "SPHL" => 0xF9,
        "EI" => 0xFB,
        "DSUB" => 0x08,
        "ARHL" => 0x10,
        "RDEL" => 0x18,
        "RSTV" => 0xCB,
        "SHLX" => 0xD9,
        "LHLX" => 0xED,
        _ => return None,
    })
}

fn alu_reg(m: &str) -> Option<u8> {
    Some(match m {
        "ADD" => 0x80,
        "ADC" => 0x88,
        "SUB" => 0x90,
        "SBB" => 0x98,
        "ANA" => 0xA0,
        "XRA" => 0xA8,
        "ORA" => 0xB0,
        "CMP" => 0xB8,
        _ => return None,
    })
}

fn imm8(m: &str) -> Option<u8> {
    Some(match m {
        "ADI" => 0xC6,
        "ACI" => 0xCE,
        "SUI" => 0xD6,
        "SBI" => 0xDE,
        "ANI" => 0xE6,
        "XRI" => 0xEE,
        "ORI" => 0xF6,
        "CPI" => 0xFE,
        "IN" => 0xDB,
        "OUT" => 0xD3,
        "LDHI" => 0x28,
        "LDSI" => 0x38,
        _ => return None,
    })
}

fn addr16(m: &str) -> Option<u8> {
    Some(match m {
        "JMP" => 0xC3,
        "JNZ" => 0xC2,
        "JZ" => 0xCA,
        "JNC" => 0xD2,
        "JC" => 0xDA,
        "JPO" => 0xE2,
        "JPE" => 0xEA,
        "JP" => 0xF2,
        "JM" => 0xFA,
        "CALL" => 0xCD,
        "CNZ" => 0xC4,
        "CZ" => 0xCC,
        "CNC" => 0xD4,
        "CC" => 0xDC,
        "CPO" => 0xE4,
        "CPE" => 0xEC,
        "CP" => 0xF4,
        "CM" => 0xFC,
        "SHLD" => 0x22,
        "LHLD" => 0x2A,
        "STA" => 0x32,
        "LDA" => 0x3A,
        "JNK" | "JNX5" => 0xDD,
        "JK" | "JX5" => 0xFD,
        _ => return None,
    })
}

fn is_mnemonic(m: &str) -> bool {
    implied(m).is_some()
        || alu_reg(m).is_some()
        || imm8(m).is_some()
        || addr16(m).is_some()
        || matches!(
            m,
            "MOV" | "MVI" | "INR" | "DCR" | "LXI" | "INX" | "DCX" | "DAD" | "PUSH" | "POP" | "STAX" | "LDAX"
                | "RST"
        )
}

fn strip_comment(line: &str) -> &str {
    let mut q: Option<char> = None;
    for (i, ch) in line.char_indices() {
        match q {
            Some(c) if ch == c => q = None,
            Some(_) => {}
            None if ch == '"' || ch == '\'' => q = Some(ch),
            None if ch == ';' => return &line[..i],
            None => {}
        }
    }
    line
}

/// Split operands on commas outside quotes.
fn split_operands(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut q: Option<char> = None;
    for ch in s.chars() {
        match q {
            Some(c) => {
                cur.push(ch);
                if ch == c {
                    q = None
                }
            }
            None if ch == '"' || ch == '\'' => {
                q = Some(ch);
                cur.push(ch)
            }
            None if ch == ',' => out.push(std::mem::take(&mut cur).trim().to_string()),
            None => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() || !out.is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

// ----- expression evaluation ------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(i64),
    Id(String),
    Op(String),
    LParen,
    RParen,
}

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '(' {
            out.push(Tok::LParen);
            i += 1;
        } else if ch == ')' {
            out.push(Tok::RParen);
            i += 1;
        } else if ch == '\'' || ch == '"' {
            // character constant: 'A' or 'AB'
            let mut j = i + 1;
            let mut v: i64 = 0;
            while j < c.len() && c[j] != ch {
                v = (v << 8) | (c[j] as i64 & 0xFF);
                j += 1;
            }
            if j >= c.len() {
                return Err("unterminated character constant".into());
            }
            out.push(Tok::Num(v));
            i = j + 1;
        } else if ch == '$' && !(i + 1 < c.len() && c[i + 1].is_ascii_hexdigit()) {
            out.push(Tok::Id("$".into()));
            i += 1;
        } else if ch == '$' {
            let mut j = i + 1;
            while j < c.len() && c[j].is_ascii_hexdigit() {
                j += 1;
            }
            let t: String = c[i + 1..j].iter().collect();
            out.push(Tok::Num(i64::from_str_radix(&t, 16).map_err(|e| e.to_string())?));
            i = j;
        } else if ch.is_ascii_digit() {
            let mut j = i;
            while j < c.len() && (c[j].is_ascii_alphanumeric()) {
                j += 1;
            }
            let t: String = c[i..j].iter().collect::<String>().to_ascii_uppercase();
            out.push(Tok::Num(parse_number(&t).ok_or(format!("bad number '{t}'"))?));
            i = j;
        } else if ch.is_ascii_alphabetic() || ch == '_' || ch == '.' || ch == '?' || ch == '@' {
            let mut j = i;
            while j < c.len() && (c[j].is_ascii_alphanumeric() || "_.?@".contains(c[j])) {
                j += 1;
            }
            let t: String = c[i..j].iter().collect::<String>().to_ascii_uppercase();
            match t.as_str() {
                "MOD" | "SHL" | "SHR" | "AND" | "OR" | "XOR" | "NOT" | "HIGH" | "LOW" => out.push(Tok::Op(t)),
                _ => out.push(Tok::Id(t)),
            }
            i = j;
        } else {
            let two: String = c[i..(i + 2).min(c.len())].iter().collect();
            if ["<<", ">>"].contains(&two.as_str()) {
                out.push(Tok::Op(two));
                i += 2;
            } else if "+-*/%&|^~".contains(ch) {
                out.push(Tok::Op(ch.to_string()));
                i += 1;
            } else {
                return Err(format!("unexpected character '{ch}'"));
            }
        }
    }
    Ok(out)
}

fn parse_number(t: &str) -> Option<i64> {
    if let Some(h) = t.strip_prefix("0X") {
        return i64::from_str_radix(h, 16).ok();
    }
    let (body, radix) = match t.chars().last()? {
        'H' => (&t[..t.len() - 1], 16),
        'B' if t[..t.len() - 1].chars().all(|c| c == '0' || c == '1') => (&t[..t.len() - 1], 2),
        'O' | 'Q' => (&t[..t.len() - 1], 8),
        'D' if t[..t.len() - 1].chars().all(|c| c.is_ascii_digit()) => (&t[..t.len() - 1], 10),
        _ => (t, 10),
    };
    i64::from_str_radix(body, radix).ok()
}

struct Eval<'a> {
    toks: Vec<Tok>,
    pos: usize,
    syms: &'a HashMap<String, u16>,
    pc: u16,
    undefined: Option<String>,
}

impl Eval<'_> {
    fn peek_op(&self) -> Option<&str> {
        match self.toks.get(self.pos) {
            Some(Tok::Op(o)) => Some(o.as_str()),
            _ => None,
        }
    }
    fn binary(&mut self, level: usize) -> Result<i64, String> {
        const LEVELS: &[&[&str]] = &[
            &["OR", "|", "XOR", "^"],
            &["AND", "&"],
            &["SHL", "SHR", "<<", ">>"],
            &["+", "-"],
            &["*", "/", "MOD", "%"],
        ];
        if level == LEVELS.len() {
            return self.unary();
        }
        let mut v = self.binary(level + 1)?;
        while let Some(op) = self.peek_op().map(str::to_string) {
            if !LEVELS[level].contains(&op.as_str()) {
                break;
            }
            self.pos += 1;
            let r = self.binary(level + 1)?;
            v = match op.as_str() {
                "OR" | "|" => v | r,
                "XOR" | "^" => v ^ r,
                "AND" | "&" => v & r,
                "SHL" | "<<" => v << (r & 31),
                "SHR" | ">>" => v >> (r & 31),
                "+" => v + r,
                "-" => v - r,
                "*" => v * r,
                "/" if r == 0 => return Err("division by zero".into()),
                "/" => v / r,
                _ if r == 0 => return Err("division by zero".into()),
                _ => v % r,
            };
        }
        Ok(v)
    }
    fn unary(&mut self) -> Result<i64, String> {
        match self.peek_op() {
            Some("-") => {
                self.pos += 1;
                Ok(-self.unary()?)
            }
            Some("+") => {
                self.pos += 1;
                self.unary()
            }
            Some("NOT") | Some("~") => {
                self.pos += 1;
                Ok(!self.unary()?)
            }
            Some("HIGH") => {
                self.pos += 1;
                Ok((self.unary()? >> 8) & 0xFF)
            }
            Some("LOW") => {
                self.pos += 1;
                Ok(self.unary()? & 0xFF)
            }
            _ => self.atom(),
        }
    }
    fn atom(&mut self) -> Result<i64, String> {
        let t = self.toks.get(self.pos).cloned().ok_or("missing operand")?;
        self.pos += 1;
        match t {
            Tok::Num(n) => Ok(n),
            Tok::Id(id) if id == "$" => Ok(self.pc as i64),
            Tok::Id(id) => match self.syms.get(&id) {
                Some(&v) => Ok(v as i64),
                None => {
                    if self.undefined.is_none() {
                        self.undefined = Some(id);
                    }
                    Ok(0)
                }
            },
            Tok::LParen => {
                let v = self.binary(0)?;
                if self.toks.get(self.pos) != Some(&Tok::RParen) {
                    return Err("missing ')'".into());
                }
                self.pos += 1;
                Ok(v)
            }
            _ => Err("syntax error in expression".into()),
        }
    }
}

struct Assembler {
    syms: HashMap<String, u16>,
    final_pass: bool,
    changed: bool,
}

impl Assembler {
    /// Evaluate; in the final pass undefined symbols are errors.
    fn eval(&self, e: &str, pc: u16) -> Result<u16, String> {
        if e.trim().is_empty() {
            return Err("missing operand".into());
        }
        let mut ev = Eval { toks: tokenize(e)?, pos: 0, syms: &self.syms, pc, undefined: None };
        let v = ev.binary(0)?;
        if ev.pos != ev.toks.len() {
            return Err(format!("unexpected text in expression '{e}'"));
        }
        if let Some(u) = ev.undefined {
            if self.final_pass {
                return Err(format!("undefined symbol '{u}'"));
            }
        }
        Ok(v as u16)
    }

    fn define(&mut self, name: &str, v: u16, redefinable: bool, line: usize) -> Result<(), AsmError> {
        let key = name.to_ascii_uppercase();
        if let Some(&old) = self.syms.get(&key) {
            if old != v {
                if !redefinable && self.final_pass {
                    return Err(AsmError { line, msg: format!("symbol '{name}' defined twice") });
                }
                self.changed = true;
            }
        } else {
            self.changed = true;
        }
        self.syms.insert(key, v);
        Ok(())
    }

    fn pass(&mut self, src: &str, img: &mut Image) -> Result<(), AsmError> {
        let mut pc: u16 = 0;
        for (n, raw) in src.lines().enumerate() {
            let line = n + 1;
            let err = |msg: String| AsmError { line, msg };
            let text = strip_comment(raw);
            if text.trim().is_empty() {
                continue;
            }
            let starts_col0 = !text.starts_with(|c: char| c.is_whitespace());
            let mut rest = text.trim();
            let mut label: Option<String> = None;

            // "LABEL:" anywhere at the start
            let ident_len = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || "_.?@".contains(c)))
                .unwrap_or(rest.len());
            if ident_len > 0 && rest[ident_len..].starts_with(':') {
                label = Some(rest[..ident_len].to_string());
                rest = rest[ident_len + 1..].trim();
            } else if ident_len > 0 && starts_col0 {
                let word = rest[..ident_len].to_ascii_uppercase();
                let next_word = rest[ident_len..].split_whitespace().next().unwrap_or("").to_ascii_uppercase();
                let next_is_dir = DIRECTIVES.contains(&next_word.as_str());
                if !(is_mnemonic(&word) || DIRECTIVES.contains(&word.as_str())) || next_is_dir {
                    label = Some(rest[..ident_len].to_string());
                    rest = rest[ident_len..].trim();
                }
            }

            let (mnem, args) = match rest.split_once(char::is_whitespace) {
                Some((m, a)) => (m.to_ascii_uppercase(), a.trim()),
                None => (rest.to_ascii_uppercase(), ""),
            };
            let mnem = mnem.trim_start_matches('.').to_string();

            if mnem == "EQU" || mnem == "SET" || mnem == "=" {
                let name = label.ok_or_else(|| err("EQU without a name".into()))?;
                let v = self.eval(args, pc).map_err(err)?;
                self.define(&name, v, mnem == "SET", line)?;
                continue;
            }
            if let Some(l) = &label {
                self.define(l, pc, false, line)?;
            }
            if mnem.is_empty() {
                continue;
            }
            let ops = split_operands(args);
            let e = |s: &str| self.eval(s, pc).map_err(|m| err(m));
            let mut out: Vec<Option<u8>> = Vec::new();
            match mnem.as_str() {
                "ORG" => {
                    pc = e(args)?;
                    continue;
                }
                "END" => {
                    if !args.is_empty() {
                        img.entry = Some(e(args)?);
                    }
                    break;
                }
                "DB" | "DEFB" | "DM" | "TEXT" => {
                    for o in &ops {
                        let q = o.chars().next();
                        if (q == Some('"') || q == Some('\'')) && o.len() >= 2 && o.ends_with(q.unwrap()) && o.len() != 3 {
                            out.extend(o[1..o.len() - 1].bytes().map(Some));
                        } else {
                            out.push(Some(e(o)? as u8));
                        }
                    }
                }
                "DW" | "DEFW" => {
                    for o in &ops {
                        let v = e(o)?;
                        out.push(Some(v as u8));
                        out.push(Some((v >> 8) as u8));
                    }
                }
                "DS" | "DEFS" => {
                    let n = e(ops.first().map(String::as_str).unwrap_or(""))?;
                    pc = pc.wrapping_add(n);
                    continue;
                }
                m => out = self.encode(m, &ops, pc).map_err(err)?.into_iter().map(Some).collect(),
            }
            for b in out {
                if let Some(b) = b {
                    if self.final_pass {
                        img.bytes.insert(pc, b);
                    }
                }
                pc = pc.wrapping_add(1);
            }
        }
        Ok(())
    }

    fn encode(&self, m: &str, ops: &[String], pc: u16) -> Result<Vec<u8>, String> {
        let want = |n: usize| -> Result<(), String> {
            if ops.len() != n {
                Err(format!("{m} needs {n} operand(s)"))
            } else {
                Ok(())
            }
        };
        let r8 = |s: &str| reg8(s).ok_or(format!("'{s}' is not a register"));
        let rp = |s: &str, psw: bool| regpair(s, psw).ok_or(format!("'{s}' is not a register pair"));
        let v = |s: &str| self.eval(s, pc);
        if let Some(op) = implied(m) {
            want(0)?;
            return Ok(vec![op]);
        }
        if let Some(op) = alu_reg(m) {
            want(1)?;
            return Ok(vec![op | r8(&ops[0])?]);
        }
        if let Some(op) = imm8(m) {
            want(1)?;
            return Ok(vec![op, v(&ops[0])? as u8]);
        }
        if let Some(op) = addr16(m) {
            want(1)?;
            let a = v(&ops[0])?;
            return Ok(vec![op, a as u8, (a >> 8) as u8]);
        }
        Ok(match m {
            "MOV" => {
                want(2)?;
                let (d, s) = (r8(&ops[0])?, r8(&ops[1])?);
                if d == 6 && s == 6 {
                    return Err("MOV M,M is not valid".into());
                }
                vec![0x40 | d << 3 | s]
            }
            "MVI" => {
                want(2)?;
                vec![0x06 | r8(&ops[0])? << 3, v(&ops[1])? as u8]
            }
            "INR" => {
                want(1)?;
                vec![0x04 | r8(&ops[0])? << 3]
            }
            "DCR" => {
                want(1)?;
                vec![0x05 | r8(&ops[0])? << 3]
            }
            "LXI" => {
                want(2)?;
                let a = v(&ops[1])?;
                vec![0x01 | rp(&ops[0], false)? << 4, a as u8, (a >> 8) as u8]
            }
            "INX" => {
                want(1)?;
                vec![0x03 | rp(&ops[0], false)? << 4]
            }
            "DCX" => {
                want(1)?;
                vec![0x0B | rp(&ops[0], false)? << 4]
            }
            "DAD" => {
                want(1)?;
                vec![0x09 | rp(&ops[0], false)? << 4]
            }
            "PUSH" => {
                want(1)?;
                vec![0xC5 | rp(&ops[0], true)? << 4]
            }
            "POP" => {
                want(1)?;
                vec![0xC1 | rp(&ops[0], true)? << 4]
            }
            "STAX" | "LDAX" => {
                want(1)?;
                let p = rp(&ops[0], false)?;
                if p > 1 {
                    return Err(format!("{m} only works with B or D"));
                }
                vec![if m == "STAX" { 0x02 } else { 0x0A } | p << 4]
            }
            "RST" => {
                want(1)?;
                let n = v(&ops[0])?;
                if n > 7 {
                    return Err("RST number must be 0-7".into());
                }
                vec![0xC7 | (n as u8) << 3]
            }
            _ => return Err(format!("unknown instruction '{m}'")),
        })
    }
}

pub fn assemble(src: &str) -> Result<Output, AsmError> {
    let mut a = Assembler { syms: HashMap::new(), final_pass: false, changed: true };
    let mut img = Image::default();
    let mut passes = 0;
    while a.changed && passes < 6 {
        a.changed = false;
        passes += 1;
        a.pass(src, &mut img)?;
    }
    a.final_pass = true;
    img = Image::default();
    a.pass(src, &mut img)?;
    Ok(Output { image: img, symbols: a.syms })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let src = "        ORG 2000H\nSTART:  MVI A,47H\n        LXI H,DATA\n        MOV M,A\n        RST 1\nDATA    DB 'A',\"BC\",0\n        DW START\n";
        let out = assemble(src).unwrap();
        let b: Vec<u8> = out.image.bytes.values().copied().collect();
        assert_eq!(b, vec![0x3E, 0x47, 0x21, 0x07, 0x20, 0x77, 0xCF, 0x41, 0x42, 0x43, 0x00, 0x00, 0x20]);
    }

    #[test]
    fn expressions() {
        let out = assemble("X EQU (1 SHL 4) + 0AH - 2*3\n DB X, HIGH 1234H, LOW 1234H, 1010B, 17Q\n").unwrap();
        let b: Vec<u8> = out.image.bytes.values().copied().collect();
        assert_eq!(b, vec![20, 0x12, 0x34, 10, 15]);
    }

    #[test]
    fn errors_have_lines() {
        let e = assemble(" MVI A,1\n JMP NOWHERE\n").err().unwrap();
        assert_eq!(e.line, 2);
    }
}
