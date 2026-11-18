//! Decoding of 8279 display bytes into 7-segment states.
//!
//! On the Microkit the 8279 outputs A3..A0,B3..B0 drive segments
//! d,c,b,a,DP,g,f,e (manual fig. 2), and a segment lights when its bit is 0.

/// Segment states in the order a, b, c, d, e, f, g, dp.
pub fn decode(raw: u8) -> [bool; 8] {
    let lit = !raw;
    let bit = |n: u8| lit & (1 << n) != 0;
    [bit(4), bit(5), bit(6), bit(7), bit(0), bit(1), bit(2), bit(3)]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn digits() {
        // DSPTB entries complemented, as OUTPT sends them
        assert_eq!(decode(!0xF3), [true, true, true, true, true, true, false, false]); // 0
        assert_eq!(decode(!0x60), [false, true, true, false, false, false, false, false]); // 1
        assert_eq!(decode(0xFB), [false, false, false, false, false, false, true, false]); // prompt '-'
        assert_eq!(decode(!(0xF7 | 0x08))[7], true); // 8 with dot
    }
}

/// Best-effort character for a display byte (for tests and tooltips).
pub fn to_char(raw: u8) -> char {
    let s = decode(raw);
    let mask: u8 = s[..7].iter().enumerate().fold(0, |m, (i, &on)| m | (on as u8) << i);
    match mask {
        0b0111111 => '0',
        0b0000110 => '1',
        0b1011011 => '2',
        0b1001111 => '3',
        0b1100110 => '4',
        0b1101101 => '5',
        0b1111101 => '6',
        0b0000111 => '7',
        0b1111111 => '8',
        0b1100111 => '9',
        0b1110111 => 'A',
        0b1111100 => 'b',
        0b0111001 => 'C',
        0b1011110 => 'd',
        0b1111001 => 'E',
        0b1110001 => 'F',
        0b1110110 => 'H',
        0b0111000 => 'L',
        0b1110011 => 'P',
        0b1010000 => 'r',
        0b1010100 => 'n',
        0b1011100 => 'o',
        0b1111000 => 't',
        0b1000000 => '-',
        0 => ' ',
        _ => '?',
    }
}

/// The whole display as text, e.g. "-80 85" -> "- 80 85" (address|data).
pub fn display_text(d: &[u8; 6]) -> String {
    let a: String = d[..4].iter().map(|&b| to_char(b)).collect();
    let b: String = d[4..].iter().map(|&b| to_char(b)).collect();
    format!("{a}|{b}")
}
