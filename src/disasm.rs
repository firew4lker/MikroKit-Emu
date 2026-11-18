//! 8085 disassembler (one instruction at a time).

const R: [&str; 8] = ["B", "C", "D", "E", "H", "L", "M", "A"];
const RP: [&str; 4] = ["B", "D", "H", "SP"];
const CC: [&str; 8] = ["NZ", "Z", "NC", "C", "PO", "PE", "P", "M"];
const ALU: [&str; 8] = ["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"];
const ALUI: [&str; 8] = ["ADI", "ACI", "SUI", "SBI", "ANI", "XRI", "ORI", "CPI"];

/// Returns (text, length in bytes).
pub fn disasm(read: impl Fn(u16) -> u8, pc: u16) -> (String, u16) {
    let op = read(pc);
    let b1 = read(pc.wrapping_add(1));
    let w = u16::from_le_bytes([b1, read(pc.wrapping_add(2))]);
    let d = ((op >> 3) & 7) as usize;
    let s = (op & 7) as usize;
    let p = ((op >> 4) & 3) as usize;
    let one = |t: String| (t, 1);
    let two = |t: String| (t, 2);
    let three = |t: String| (t, 3);
    match op {
        0x76 => one("HLT".into()),
        0x40..=0x7F => one(format!("MOV {},{}", R[d], R[s])),
        0x80..=0xBF => one(format!("{} {}", ALU[d], R[s])),
        _ if op & 0xC7 == 0xC6 => two(format!("{} {:02X}H", ALUI[d], b1)),
        _ if op & 0xC7 == 0x06 => two(format!("MVI {},{:02X}H", R[d], b1)),
        _ if op & 0xC7 == 0x04 => one(format!("INR {}", R[d])),
        _ if op & 0xC7 == 0x05 => one(format!("DCR {}", R[d])),
        _ if op & 0xCF == 0x01 => three(format!("LXI {},{:04X}H", RP[p], w)),
        _ if op & 0xCF == 0x03 => one(format!("INX {}", RP[p])),
        _ if op & 0xCF == 0x0B => one(format!("DCX {}", RP[p])),
        _ if op & 0xCF == 0x09 => one(format!("DAD {}", RP[p])),
        0x02 | 0x12 => one(format!("STAX {}", RP[p])),
        0x0A | 0x1A => one(format!("LDAX {}", RP[p])),
        0x22 => three(format!("SHLD {:04X}H", w)),
        0x2A => three(format!("LHLD {:04X}H", w)),
        0x32 => three(format!("STA {:04X}H", w)),
        0x3A => three(format!("LDA {:04X}H", w)),
        _ if op & 0xC7 == 0xC2 => three(format!("J{} {:04X}H", CC[d], w)),
        _ if op & 0xC7 == 0xC4 => three(format!("C{} {:04X}H", CC[d], w)),
        _ if op & 0xC7 == 0xC0 => one(format!("R{}", CC[d])),
        _ if op & 0xC7 == 0xC7 => one(format!("RST {}", d)),
        0xC5 | 0xD5 | 0xE5 => one(format!("PUSH {}", RP[p])),
        0xC1 | 0xD1 | 0xE1 => one(format!("POP {}", RP[p])),
        0xF5 => one("PUSH PSW".into()),
        0xF1 => one("POP PSW".into()),
        0xC3 => three(format!("JMP {:04X}H", w)),
        0xCD => three(format!("CALL {:04X}H", w)),
        0xC9 => one("RET".into()),
        0xD3 => two(format!("OUT {:02X}H", b1)),
        0xDB => two(format!("IN {:02X}H", b1)),
        0x28 => two(format!("LDHI {:02X}H", b1)),
        0x38 => two(format!("LDSI {:02X}H", b1)),
        0xDD => three(format!("JNK {:04X}H", w)),
        0xFD => three(format!("JK {:04X}H", w)),
        _ => one(
            match op {
                0x00 => "NOP",
                0x07 => "RLC",
                0x0F => "RRC",
                0x17 => "RAL",
                0x1F => "RAR",
                0x20 => "RIM",
                0x27 => "DAA",
                0x2F => "CMA",
                0x30 => "SIM",
                0x37 => "STC",
                0x3F => "CMC",
                0x08 => "DSUB",
                0x10 => "ARHL",
                0x18 => "RDEL",
                0xCB => "RSTV",
                0xD9 => "SHLX",
                0xED => "LHLX",
                0xE3 => "XTHL",
                0xE9 => "PCHL",
                0xEB => "XCHG",
                0xF3 => "DI",
                0xF9 => "SPHL",
                0xFB => "EI",
                _ => "???",
            }
            .to_string(),
        ),
    }
}
