//! Program images: Intel HEX read/write and raw binaries.

use std::collections::BTreeMap;

/// A sparse memory image.
#[derive(Clone, Debug, Default)]
pub struct Image {
    pub bytes: BTreeMap<u16, u8>,
    /// Entry point if the source specified one (HEX type 03/05 or asm END x).
    pub entry: Option<u16>,
}

impl Image {
    pub fn from_bin(data: &[u8], base: u16) -> Image {
        let mut img = Image::default();
        for (i, &b) in data.iter().enumerate() {
            img.bytes.insert(base.wrapping_add(i as u16), b);
        }
        img
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn start(&self) -> Option<u16> {
        self.entry.or_else(|| self.bytes.keys().next().copied())
    }

    /// Contiguous address ranges covered by the image.
    pub fn ranges(&self) -> Vec<(u16, u16)> {
        let mut out: Vec<(u16, u16)> = Vec::new();
        for &a in self.bytes.keys() {
            match out.last_mut() {
                Some((_, end)) if end.wrapping_add(1) == a && *end != 0xFFFF => *end = a,
                _ => out.push((a, a)),
            }
        }
        out
    }

    pub fn describe(&self) -> String {
        let r = self.ranges();
        let parts: Vec<String> = r.iter().take(3).map(|(a, b)| format!("{:04X}–{:04X}", a, b)).collect();
        let mut s = parts.join(", ");
        if r.len() > 3 {
            s.push_str(&format!(" (+{} more)", r.len() - 3));
        }
        s
    }

    /// Flat 8 KB ROM image (0000-1FFF) from this image, gaps filled with 00
    /// like the original EPROM.
    pub fn to_rom(&self) -> Vec<u8> {
        let mut rom = vec![0u8; 0x2000];
        for (&a, &b) in &self.bytes {
            if (a as usize) < rom.len() {
                rom[a as usize] = b;
            }
        }
        rom
    }
}

fn hex_byte(s: &str, i: usize) -> Option<u8> {
    u8::from_str_radix(s.get(i..i + 2)?, 16).ok()
}

pub fn parse_ihex(text: &str) -> Result<Image, String> {
    let mut img = Image::default();
    let mut base: u32 = 0;
    let mut saw_record = false;
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let ln = n + 1;
        let Some(rec) = line.strip_prefix(':') else {
            return Err(format!("line {ln}: doesn't start with ':'"));
        };
        let bytes: Vec<u8> = (0..rec.len() / 2)
            .map(|i| hex_byte(rec, i * 2))
            .collect::<Option<_>>()
            .ok_or(format!("line {ln}: bad hex digits"))?;
        if bytes.len() < 5 || bytes.len() != bytes[0] as usize + 5 {
            return Err(format!("line {ln}: wrong record length"));
        }
        let sum = bytes.iter().fold(0u8, |a, &b| a.wrapping_add(b));
        if sum != 0 {
            return Err(format!("line {ln}: checksum error"));
        }
        saw_record = true;
        let len = bytes[0] as usize;
        let addr = u16::from_be_bytes([bytes[1], bytes[2]]) as u32;
        let data = &bytes[4..4 + len];
        match bytes[3] {
            0x00 => {
                for (i, &b) in data.iter().enumerate() {
                    let a = (base + addr + i as u32) & 0xFFFF;
                    img.bytes.insert(a as u16, b);
                }
            }
            0x01 => break,
            0x02 if len == 2 => base = (u16::from_be_bytes([data[0], data[1]]) as u32) << 4,
            0x04 if len == 2 => base = (u16::from_be_bytes([data[0], data[1]]) as u32) << 16,
            0x03 if len == 4 => img.entry = Some(u16::from_be_bytes([data[2], data[3]])),
            0x05 if len == 4 => img.entry = Some(u16::from_be_bytes([data[2], data[3]])),
            t => return Err(format!("line {ln}: unsupported record type {t:02X}")),
        }
    }
    if !saw_record {
        return Err("no Intel HEX records found".into());
    }
    Ok(img)
}

pub fn write_ihex(start: u16, data: &[u8]) -> String {
    let mut s = String::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        let addr = start.wrapping_add((i * 16) as u16);
        let mut rec = vec![chunk.len() as u8, (addr >> 8) as u8, addr as u8, 0];
        rec.extend_from_slice(chunk);
        let sum = rec.iter().fold(0u8, |a, &b| a.wrapping_add(b));
        rec.push(sum.wrapping_neg());
        s.push(':');
        for b in rec {
            s.push_str(&format!("{b:02X}"));
        }
        s.push_str("\r\n");
    }
    s.push_str(":00000001FF\r\n");
    s
}

/// Parse "2000", "2000H", "0x2000" or "$2000" as a hex address.
pub fn parse_addr(s: &str) -> Option<u16> {
    let t = s.trim().to_ascii_uppercase();
    let t = t.strip_prefix("0X").or_else(|| t.strip_prefix('$')).unwrap_or(&t);
    let t = t.strip_suffix('H').unwrap_or(t);
    u16::from_str_radix(t, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let data: Vec<u8> = (0..40).collect();
        let text = write_ihex(0x2000, &data);
        let img = parse_ihex(&text).unwrap();
        assert_eq!(img.ranges(), vec![(0x2000, 0x2027)]);
        assert_eq!(img.bytes[&0x2027], 39);
    }
    #[test]
    fn bad_checksum() {
        assert!(parse_ihex(":0100000000FE\n").is_err());
    }
}
