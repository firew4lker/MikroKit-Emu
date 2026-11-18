//! Intel 8085 CPU core with T-state (clock cycle) accounting.
//!
//! Cycle counts are added at the *start* of each instruction, so any memory
//! or I/O access the instruction performs is seen by the bus at the time the
//! instruction completes. The monitor's single-step logic depends on this
//! (see `Machine` for the TRAP timing rule).

/// Everything the CPU needs from the outside world.
pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
    fn input(&mut self, port: u8, now: u64) -> u8;
    fn output(&mut self, port: u8, val: u8, now: u64);
    /// Current level of the SID pin.
    fn sid(&mut self, now: u64) -> bool;
    /// SOD pin changed (called on every SIM with SOE set).
    fn sod(&mut self, level: bool, now: u64);
}

pub const FLAG_S: u8 = 0x80;
pub const FLAG_Z: u8 = 0x40;
pub const FLAG_K: u8 = 0x20; // undocumented "X5"/K flag
pub const FLAG_AC: u8 = 0x10;
pub const FLAG_P: u8 = 0x04;
pub const FLAG_V: u8 = 0x02; // undocumented overflow flag
pub const FLAG_CY: u8 = 0x01;

#[derive(Clone, Debug, Default)]
pub struct Cpu {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
    /// Interrupt enable flip-flop.
    pub ie: bool,
    /// EI takes effect after the following instruction.
    ei_pending: bool,
    /// IE state saved when TRAP is taken (reported by the next RIM).
    trap_saved_ie: Option<bool>,
    /// Interrupt masks for RST 7.5 / 6.5 / 5.5 (true = masked).
    pub m75: bool,
    pub m65: bool,
    pub m55: bool,
    /// RST 7.5 edge latch.
    pub i75: bool,
    pub sod: bool,
    pub halted: bool,
    pub cycles: u64,
    lines: IntLines,
}

/// Interrupt input levels sampled at the end of every instruction.
#[derive(Clone, Copy, Debug, Default)]
pub struct IntLines {
    pub trap: bool,
    pub rst65: bool,
    pub rst55: bool,
    pub intr: bool,
}

fn parity(v: u8) -> bool {
    v.count_ones() % 2 == 0
}

impl Cpu {
    pub fn new() -> Self {
        let mut c = Cpu::default();
        c.reset();
        c
    }

    /// RESET IN: PC=0, interrupts disabled and masked, SOD low.
    /// Other registers keep their values, like the real chip.
    pub fn reset(&mut self) {
        self.pc = 0;
        self.ie = false;
        self.ei_pending = false;
        self.trap_saved_ie = None;
        self.m75 = true;
        self.m65 = true;
        self.m55 = true;
        self.i75 = false;
        self.sod = false;
        self.halted = false;
    }

    pub fn bc(&self) -> u16 {
        u16::from_be_bytes([self.b, self.c])
    }
    pub fn de(&self) -> u16 {
        u16::from_be_bytes([self.d, self.e])
    }
    pub fn hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }
    fn set_bc(&mut self, v: u16) {
        [self.b, self.c] = v.to_be_bytes();
    }
    fn set_de(&mut self, v: u16) {
        [self.d, self.e] = v.to_be_bytes();
    }
    fn set_hl(&mut self, v: u16) {
        [self.h, self.l] = v.to_be_bytes();
    }

    /// Edge on the RST 7.5 pin (the "VI" key).
    pub fn pulse_rst75(&mut self) {
        self.i75 = true;
    }

    fn flag(&self, f: u8) -> bool {
        self.f & f != 0
    }
    fn setf(&mut self, f: u8, on: bool) {
        if on {
            self.f |= f
        } else {
            self.f &= !f
        }
    }
    fn szp(&mut self, v: u8) {
        self.setf(FLAG_S, v & 0x80 != 0);
        self.setf(FLAG_Z, v == 0);
        self.setf(FLAG_P, parity(v));
    }

    fn fetch<B: Bus>(&mut self, bus: &mut B) -> u8 {
        let v = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        v
    }
    fn fetch16<B: Bus>(&mut self, bus: &mut B) -> u16 {
        let lo = self.fetch(bus);
        let hi = self.fetch(bus);
        u16::from_le_bytes([lo, hi])
    }
    fn push<B: Bus>(&mut self, bus: &mut B, v: u16) {
        let [hi, lo] = v.to_be_bytes();
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, hi);
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, lo);
    }
    fn pop<B: Bus>(&mut self, bus: &mut B) -> u16 {
        let lo = bus.read(self.sp);
        self.sp = self.sp.wrapping_add(1);
        let hi = bus.read(self.sp);
        self.sp = self.sp.wrapping_add(1);
        u16::from_le_bytes([lo, hi])
    }

    fn reg<B: Bus>(&mut self, bus: &mut B, r: u8) -> u8 {
        match r {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => bus.read(self.hl()),
            _ => self.a,
        }
    }
    fn set_reg<B: Bus>(&mut self, bus: &mut B, r: u8, v: u8) {
        match r {
            0 => self.b = v,
            1 => self.c = v,
            2 => self.d = v,
            3 => self.e = v,
            4 => self.h = v,
            5 => self.l = v,
            6 => bus.write(self.hl(), v),
            _ => self.a = v,
        }
    }
    fn rp(&self, p: u8) -> u16 {
        match p {
            0 => self.bc(),
            1 => self.de(),
            2 => self.hl(),
            _ => self.sp,
        }
    }
    fn set_rp(&mut self, p: u8, v: u16) {
        match p {
            0 => self.set_bc(v),
            1 => self.set_de(v),
            2 => self.set_hl(v),
            _ => self.sp = v,
        }
    }
    fn cond(&self, c: u8) -> bool {
        match c {
            0 => !self.flag(FLAG_Z),
            1 => self.flag(FLAG_Z),
            2 => !self.flag(FLAG_CY),
            3 => self.flag(FLAG_CY),
            4 => !self.flag(FLAG_P),
            5 => self.flag(FLAG_P),
            6 => !self.flag(FLAG_S),
            _ => self.flag(FLAG_S),
        }
    }

    /// 8-bit add with carry-in; sets all arithmetic flags.
    fn add8(&mut self, a: u8, b: u8, cin: bool) -> u8 {
        let r16 = a as u16 + b as u16 + cin as u16;
        let r = r16 as u8;
        self.szp(r);
        self.setf(FLAG_CY, r16 > 0xFF);
        self.setf(FLAG_AC, (a & 0x0F) + (b & 0x0F) + cin as u8 > 0x0F);
        let v = (a ^ r) & (b ^ r) & 0x80 != 0;
        self.setf(FLAG_V, v);
        self.setf(FLAG_K, v ^ (r & 0x80 != 0));
        r
    }
    /// 8-bit subtract with borrow-in; CY = borrow.
    fn sub8(&mut self, a: u8, b: u8, bin: bool) -> u8 {
        let r = a.wrapping_sub(b).wrapping_sub(bin as u8);
        self.szp(r);
        self.setf(FLAG_CY, (a as u16) < b as u16 + bin as u16);
        // 8085 sets AC on subtraction as the carry out of bit 3 of a + ~b + 1
        self.setf(FLAG_AC, (a & 0x0F) as i16 - (b & 0x0F) as i16 - bin as i16 >= 0);
        let v = (a ^ b) & (a ^ r) & 0x80 != 0;
        self.setf(FLAG_V, v);
        self.setf(FLAG_K, v ^ (r & 0x80 != 0));
        r
    }
    fn alu(&mut self, op: u8, v: u8) {
        match op {
            0 => self.a = self.add8(self.a, v, false),
            1 => self.a = self.add8(self.a, v, self.flag(FLAG_CY)),
            2 => self.a = self.sub8(self.a, v, false),
            3 => self.a = self.sub8(self.a, v, self.flag(FLAG_CY)),
            4 => {
                // ANA: AC is set on the 8085
                self.a &= v;
                let a = self.a;
                self.szp(a);
                self.f &= !(FLAG_CY | FLAG_V);
                self.f |= FLAG_AC;
            }
            5 => {
                self.a ^= v;
                let a = self.a;
                self.szp(a);
                self.f &= !(FLAG_CY | FLAG_AC | FLAG_V);
            }
            6 => {
                self.a |= v;
                let a = self.a;
                self.szp(a);
                self.f &= !(FLAG_CY | FLAG_AC | FLAG_V);
            }
            _ => {
                let a = self.a;
                self.sub8(a, v, false);
            }
        }
    }

    /// Check pending interrupts at an instruction boundary. Returns true if
    /// one was taken.
    fn service_interrupts<B: Bus>(&mut self, bus: &mut B, lines: IntLines) -> bool {
        let vector = if lines.trap {
            self.trap_saved_ie = Some(self.ie);
            Some(0x24)
        } else if !self.ie {
            None
        } else if self.i75 && !self.m75 {
            self.i75 = false;
            Some(0x3C)
        } else if lines.rst65 && !self.m65 {
            Some(0x34)
        } else if lines.rst55 && !self.m55 {
            Some(0x2C)
        } else if lines.intr {
            Some(0x38) // INTR with RST 7 on the bus (pull-ups)
        } else {
            None
        };
        if let Some(v) = vector {
            self.halted = false;
            self.ie = false;
            self.ei_pending = false;
            self.cycles += 12;
            let pc = self.pc;
            self.push(bus, pc);
            self.pc = v;
            true
        } else {
            false
        }
    }

    /// Execute one instruction (or take one interrupt). `trap` must be an
    /// edge that the caller clears once it has been delivered.
    pub fn step<B: Bus>(&mut self, bus: &mut B, lines: IntLines) -> u64 {
        let start = self.cycles;
        self.lines = lines;
        // While an EI is pending `ie` is still false, so only TRAP can be taken.
        if self.service_interrupts(bus, lines) {
            return self.cycles - start;
        }
        if self.ei_pending {
            self.ei_pending = false;
            self.ie = true;
        }
        if self.halted {
            self.cycles += 4;
            return self.cycles - start;
        }
        let op = self.fetch(bus);
        self.exec(bus, op);
        self.cycles - start
    }

    fn exec<B: Bus>(&mut self, bus: &mut B, op: u8) {
        let ddd = (op >> 3) & 7;
        let sss = op & 7;
        let rp = (op >> 4) & 3;
        match op {
            // MOV / HLT
            0x76 => {
                self.cycles += 5;
                self.halted = true;
            }
            0x40..=0x7F => {
                self.cycles += if ddd == 6 || sss == 6 { 7 } else { 4 };
                let v = self.reg(bus, sss);
                self.set_reg(bus, ddd, v);
            }
            // ALU r
            0x80..=0xBF => {
                self.cycles += if sss == 6 { 7 } else { 4 };
                let v = self.reg(bus, sss);
                self.alu(ddd, v);
            }
            // ALU imm
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => {
                self.cycles += 7;
                let v = self.fetch(bus);
                self.alu(ddd, v);
            }
            // MVI
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E => {
                self.cycles += if ddd == 6 { 10 } else { 7 };
                let v = self.fetch(bus);
                self.set_reg(bus, ddd, v);
            }
            // INR / DCR (CY unaffected)
            0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => {
                self.cycles += if ddd == 6 { 10 } else { 4 };
                let v = self.reg(bus, ddd);
                let cy = self.flag(FLAG_CY);
                let r = self.add8(v, 1, false);
                self.setf(FLAG_CY, cy);
                self.set_reg(bus, ddd, r);
            }
            0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => {
                self.cycles += if ddd == 6 { 10 } else { 4 };
                let v = self.reg(bus, ddd);
                let cy = self.flag(FLAG_CY);
                let r = self.sub8(v, 1, false);
                self.setf(FLAG_CY, cy);
                self.set_reg(bus, ddd, r);
            }
            // LXI / INX / DCX / DAD
            0x01 | 0x11 | 0x21 | 0x31 => {
                self.cycles += 10;
                let v = self.fetch16(bus);
                self.set_rp(rp, v);
            }
            0x03 | 0x13 | 0x23 | 0x33 => {
                self.cycles += 6;
                let v = self.rp(rp).wrapping_add(1);
                self.set_rp(rp, v);
            }
            0x0B | 0x1B | 0x2B | 0x3B => {
                self.cycles += 6;
                let v = self.rp(rp).wrapping_sub(1);
                self.set_rp(rp, v);
            }
            0x09 | 0x19 | 0x29 | 0x39 => {
                self.cycles += 10;
                let r = self.hl() as u32 + self.rp(rp) as u32;
                self.setf(FLAG_CY, r > 0xFFFF);
                self.set_hl(r as u16);
            }
            // STAX / LDAX
            0x02 | 0x12 => {
                self.cycles += 7;
                let a = self.rp(rp);
                bus.write(a, self.a);
            }
            0x0A | 0x1A => {
                self.cycles += 7;
                let a = self.rp(rp);
                self.a = bus.read(a);
            }
            0x22 => {
                self.cycles += 16;
                let a = self.fetch16(bus);
                bus.write(a, self.l);
                bus.write(a.wrapping_add(1), self.h);
            }
            0x2A => {
                self.cycles += 16;
                let a = self.fetch16(bus);
                self.l = bus.read(a);
                self.h = bus.read(a.wrapping_add(1));
            }
            0x32 => {
                self.cycles += 13;
                let a = self.fetch16(bus);
                bus.write(a, self.a);
            }
            0x3A => {
                self.cycles += 13;
                let a = self.fetch16(bus);
                self.a = bus.read(a);
            }
            // rotates
            0x07 => {
                self.cycles += 4;
                let c = self.a >> 7;
                self.a = self.a.rotate_left(1);
                self.setf(FLAG_CY, c != 0);
            }
            0x0F => {
                self.cycles += 4;
                let c = self.a & 1;
                self.a = self.a.rotate_right(1);
                self.setf(FLAG_CY, c != 0);
            }
            0x17 => {
                self.cycles += 4;
                let c = self.a >> 7;
                self.a = (self.a << 1) | self.flag(FLAG_CY) as u8;
                self.setf(FLAG_CY, c != 0);
            }
            0x1F => {
                self.cycles += 4;
                let c = self.a & 1;
                self.a = (self.a >> 1) | ((self.flag(FLAG_CY) as u8) << 7);
                self.setf(FLAG_CY, c != 0);
            }
            0x27 => {
                // DAA
                self.cycles += 4;
                let mut add = 0u8;
                let mut cy = self.flag(FLAG_CY);
                if self.a & 0x0F > 9 || self.flag(FLAG_AC) {
                    add |= 0x06;
                }
                if self.a > 0x99 || cy {
                    add |= 0x60;
                    cy = true;
                }
                let a = self.a;
                let r = self.add8(a, add, false);
                self.a = r;
                self.setf(FLAG_CY, cy);
            }
            0x2F => {
                self.cycles += 4;
                self.a = !self.a;
            }
            0x37 => {
                self.cycles += 4;
                self.setf(FLAG_CY, true);
            }
            0x3F => {
                self.cycles += 4;
                let c = self.flag(FLAG_CY);
                self.setf(FLAG_CY, !c);
            }
            0x00 => self.cycles += 4,
            // RIM
            0x20 => {
                self.cycles += 4;
                let now = self.cycles;
                let ie = self.trap_saved_ie.take().unwrap_or(self.ie);
                let mut v = (self.m55 as u8) | (self.m65 as u8) << 1 | (self.m75 as u8) << 2;
                v |= (ie as u8) << 3;
                v |= (self.i75 as u8) << 6;
                v |= (self.lines.rst55 as u8) << 4;
                v |= (self.lines.rst65 as u8) << 5;
                v |= (bus.sid(now) as u8) << 7;
                self.a = v;
            }
            // SIM
            0x30 => {
                self.cycles += 4;
                let a = self.a;
                if a & 0x08 != 0 {
                    self.m55 = a & 1 != 0;
                    self.m65 = a & 2 != 0;
                    self.m75 = a & 4 != 0;
                }
                if a & 0x10 != 0 {
                    self.i75 = false;
                }
                if a & 0x40 != 0 {
                    self.sod = a & 0x80 != 0;
                    let now = self.cycles;
                    bus.sod(self.sod, now);
                }
            }
            // undocumented 8085 instructions
            0x08 => {
                // DSUB: HL = HL - BC
                self.cycles += 10;
                let (hl, bc) = (self.hl(), self.bc());
                let lo = self.sub8(self.l, self.c, false);
                let hi = self.sub8(self.h, self.b, self.flag(FLAG_CY));
                let _ = (hl, bc);
                self.l = lo;
                self.h = hi;
                self.setf(FLAG_Z, lo == 0 && hi == 0);
            }
            0x10 => {
                // ARHL: arithmetic shift HL right
                self.cycles += 7;
                let hl = self.hl();
                self.setf(FLAG_CY, hl & 1 != 0);
                self.set_hl((hl >> 1) | (hl & 0x8000));
            }
            0x18 => {
                // RDEL: rotate DE left through carry
                self.cycles += 10;
                let de = self.de();
                let c = self.flag(FLAG_CY) as u16;
                self.setf(FLAG_CY, de & 0x8000 != 0);
                self.setf(FLAG_V, ((de << 1) ^ de) & 0x8000 != 0);
                self.set_de((de << 1) | c);
            }
            0x28 => {
                // LDHI d8: DE = HL + d8
                self.cycles += 10;
                let v = self.fetch(bus) as u16;
                let r = self.hl().wrapping_add(v);
                self.set_de(r);
            }
            0x38 => {
                // LDSI d8: DE = SP + d8
                self.cycles += 10;
                let v = self.fetch(bus) as u16;
                let r = self.sp.wrapping_add(v);
                self.set_de(r);
            }
            0xD9 => {
                // SHLX: (DE) = HL
                self.cycles += 10;
                let a = self.de();
                bus.write(a, self.l);
                bus.write(a.wrapping_add(1), self.h);
            }
            0xED => {
                // LHLX: HL = (DE)
                self.cycles += 10;
                let a = self.de();
                self.l = bus.read(a);
                self.h = bus.read(a.wrapping_add(1));
            }
            0xDD | 0xFD => {
                // JNK / JK
                let k = self.flag(FLAG_K);
                let take = if op == 0xFD { k } else { !k };
                let a = self.fetch16(bus);
                if take {
                    self.cycles += 10;
                    self.pc = a;
                } else {
                    self.cycles += 7;
                }
            }
            0xCB => {
                // RSTV: RST 8 (0040h) if V set
                if self.flag(FLAG_V) {
                    self.cycles += 12;
                    let pc = self.pc;
                    self.push(bus, pc);
                    self.pc = 0x40;
                } else {
                    self.cycles += 6;
                }
            }
            // jumps / calls / returns
            0xC3 => {
                self.cycles += 10;
                self.pc = self.fetch16(bus);
            }
            0xC2 | 0xCA | 0xD2 | 0xDA | 0xE2 | 0xEA | 0xF2 | 0xFA => {
                let a = self.fetch16(bus);
                if self.cond(ddd) {
                    self.cycles += 10;
                    self.pc = a;
                } else {
                    self.cycles += 7;
                }
            }
            0xCD => {
                self.cycles += 18;
                let a = self.fetch16(bus);
                let pc = self.pc;
                self.push(bus, pc);
                self.pc = a;
            }
            0xC4 | 0xCC | 0xD4 | 0xDC | 0xE4 | 0xEC | 0xF4 | 0xFC => {
                let a = self.fetch16(bus);
                if self.cond(ddd) {
                    self.cycles += 18;
                    let pc = self.pc;
                    self.push(bus, pc);
                    self.pc = a;
                } else {
                    self.cycles += 9;
                }
            }
            0xC9 => {
                self.cycles += 10;
                self.pc = self.pop(bus);
            }
            0xC0 | 0xC8 | 0xD0 | 0xD8 | 0xE0 | 0xE8 | 0xF0 | 0xF8 => {
                if self.cond(ddd) {
                    self.cycles += 12;
                    self.pc = self.pop(bus);
                } else {
                    self.cycles += 6;
                }
            }
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                self.cycles += 12;
                let pc = self.pc;
                self.push(bus, pc);
                self.pc = (ddd as u16) * 8;
            }
            0xE9 => {
                self.cycles += 6;
                self.pc = self.hl();
            }
            0xF9 => {
                self.cycles += 6;
                self.sp = self.hl();
            }
            // stack
            0xC5 | 0xD5 | 0xE5 => {
                self.cycles += 12;
                let v = self.rp(rp);
                self.push(bus, v);
            }
            0xF5 => {
                self.cycles += 12;
                let v = u16::from_be_bytes([self.a, self.f]);
                self.push(bus, v);
            }
            0xC1 | 0xD1 | 0xE1 => {
                self.cycles += 10;
                let v = self.pop(bus);
                self.set_rp(rp, v);
            }
            0xF1 => {
                self.cycles += 10;
                let v = self.pop(bus);
                [self.a, self.f] = v.to_be_bytes();
            }
            0xE3 => {
                self.cycles += 16;
                let lo = bus.read(self.sp);
                let hi = bus.read(self.sp.wrapping_add(1));
                bus.write(self.sp, self.l);
                bus.write(self.sp.wrapping_add(1), self.h);
                self.l = lo;
                self.h = hi;
            }
            0xEB => {
                self.cycles += 4;
                std::mem::swap(&mut self.d, &mut self.h);
                std::mem::swap(&mut self.e, &mut self.l);
            }
            // I/O
            0xD3 => {
                self.cycles += 10;
                let p = self.fetch(bus);
                let now = self.cycles;
                bus.output(p, self.a, now);
            }
            0xDB => {
                self.cycles += 10;
                let p = self.fetch(bus);
                let now = self.cycles;
                self.a = bus.input(p, now);
            }
            0xF3 => {
                self.cycles += 4;
                self.ie = false;
            }
            0xFB => {
                self.cycles += 4;
                self.ei_pending = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Ram(Vec<u8>);
    impl Bus for Ram {
        fn read(&mut self, a: u16) -> u8 {
            self.0[a as usize]
        }
        fn write(&mut self, a: u16, v: u8) {
            self.0[a as usize] = v
        }
        fn input(&mut self, _: u8, _: u64) -> u8 {
            0xFF
        }
        fn output(&mut self, _: u8, _: u8, _: u64) {}
        fn sid(&mut self, _: u64) -> bool {
            true
        }
        fn sod(&mut self, _: bool, _: u64) {}
    }

    fn run(prog: &[u8], steps: usize) -> (Cpu, Ram) {
        let mut m = Ram(vec![0; 65536]);
        m.0[..prog.len()].copy_from_slice(prog);
        let mut c = Cpu::new();
        c.sp = 0xF000;
        for _ in 0..steps {
            c.step(&mut m, IntLines::default());
        }
        (c, m)
    }

    #[test]
    fn add_and_flags() {
        // MVI A,7F ; ADI 01
        let (c, _) = run(&[0x3E, 0x7F, 0xC6, 0x01], 2);
        assert_eq!(c.a, 0x80);
        assert!(c.f & FLAG_S != 0 && c.f & FLAG_AC != 0 && c.f & FLAG_V != 0);
        assert!(c.f & FLAG_CY == 0);
        assert_eq!(c.cycles, 14);
    }

    #[test]
    fn sub_borrow() {
        // MVI A,01 ; SUI 02 ; CPI FF
        let (c, _) = run(&[0x3E, 0x01, 0xD6, 0x02, 0xFE, 0xFF], 3);
        assert_eq!(c.a, 0xFF);
        assert!(c.f & FLAG_Z != 0);
    }

    #[test]
    fn daa() {
        // MVI A,19 ; ADI 28 ; DAA  -> 47
        let (c, _) = run(&[0x3E, 0x19, 0xC6, 0x28, 0x27], 3);
        assert_eq!(c.a, 0x47);
    }

    #[test]
    fn call_ret_push_pop() {
        // LXI B,1234 ; PUSH B ; CALL 0010 ; POP D ; HLT ... 0010: RET
        let mut p = vec![0x01, 0x34, 0x12, 0xC5, 0xCD, 0x10, 0x00, 0xD1, 0x76];
        p.resize(0x11, 0);
        p[0x10] = 0xC9;
        let (c, _) = run(&p, 6);
        assert_eq!(c.de(), 0x1234);
        assert!(c.halted);
        assert_eq!(c.sp, 0xF000);
    }

    #[test]
    fn ei_delay_and_rst55() {
        let mut m = Ram(vec![0; 65536]);
        // MVI A,0E? no: unmask 5.5 -> MVI A,0E ; SIM ; EI ; NOP ; NOP
        m.0[..6].copy_from_slice(&[0x3E, 0x0E, 0x30, 0xFB, 0x00, 0x00]);
        let mut c = Cpu::new();
        c.sp = 0xF000;
        let lines = IntLines { rst55: true, ..Default::default() };
        c.step(&mut m, lines); // MVI
        c.step(&mut m, lines); // SIM
        c.step(&mut m, lines); // EI
        c.step(&mut m, lines); // NOP executes (EI delay)
        assert_eq!(c.pc, 5);
        c.step(&mut m, lines); // interrupt taken
        assert_eq!(c.pc, 0x2C);
        assert!(!c.ie);
    }
}
