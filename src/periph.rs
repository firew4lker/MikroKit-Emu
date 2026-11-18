//! Peripheral chips of the Microkit: 8279 keyboard/display controller,
//! 8155 RAM/ports/timer, 8251 USART, and the bit-level serial line that
//! drives the 8085 SID pin (DB9-2).

use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// 8279 keyboard / display controller
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct I8279 {
    /// Display RAM, raw bytes as written by the CPU (segments are active low
    /// on the Microkit: a 0 bit lights the segment).
    pub display: [u8; 16],
    addr: u8,
    auto_inc: bool,
    read_display: bool,
    fifo: VecDeque<u8>,
    overrun: bool,
    blank_code: u8,
}

impl Default for I8279 {
    fn default() -> Self {
        I8279 {
            display: [0xFF; 16],
            addr: 0,
            auto_inc: false,
            read_display: false,
            fifo: VecDeque::new(),
            overrun: false,
            blank_code: 0x00,
        }
    }
}

impl I8279 {
    pub fn reset(&mut self) {
        self.fifo.clear();
        self.overrun = false;
        self.addr = 0;
        self.auto_inc = false;
        self.read_display = false;
    }

    /// A key was pressed: push its scan code (scan line * 8 + return line).
    pub fn key_press(&mut self, code: u8) {
        if self.fifo.len() >= 8 {
            self.overrun = true;
        } else {
            // SHIFT and CNTL inputs are pulled high on the board.
            self.fifo.push_back(0xC0 | (code & 0x3F));
        }
    }

    /// IRQ output (wired to RST 5.5): high while the FIFO holds data.
    pub fn irq(&self) -> bool {
        !self.fifo.is_empty()
    }

    pub fn write_cmd(&mut self, v: u8) {
        match v >> 5 {
            0 | 1 => {} // keyboard/display mode set, clock prescaler
            2 => {
                self.read_display = false;
                self.auto_inc = v & 0x10 != 0;
            }
            3 => {
                self.read_display = true;
                self.auto_inc = v & 0x10 != 0;
                self.addr = v & 0x0F;
            }
            4 => {
                self.auto_inc = v & 0x10 != 0;
                self.addr = v & 0x0F;
            }
            5 => {} // display write inhibit / blanking
            6 => {
                // clear: D3 D2 select the blanking code, D4 enables the clear
                self.blank_code = match (v >> 2) & 3 {
                    2 => 0x20,
                    3 => 0xFF,
                    _ => 0x00,
                };
                if v & 0x10 != 0 || v & 0x01 != 0 {
                    self.display = [self.blank_code; 16];
                }
                if v & 0x02 != 0 || v & 0x01 != 0 {
                    self.fifo.clear();
                    self.overrun = false;
                }
            }
            _ => {} // end interrupt
        }
    }

    pub fn write_data(&mut self, v: u8) {
        self.display[self.addr as usize] = v;
        if self.auto_inc {
            self.addr = (self.addr + 1) & 0x0F;
        }
    }

    pub fn read_data(&mut self) -> u8 {
        if self.read_display {
            let v = self.display[self.addr as usize];
            if self.auto_inc {
                self.addr = (self.addr + 1) & 0x0F;
            }
            v
        } else {
            self.fifo.pop_front().unwrap_or(0xFF)
        }
    }

    pub fn read_status(&self) -> u8 {
        let n = self.fifo.len() as u8;
        (n.min(7)) | if n >= 8 { 0x08 } else { 0 } | if self.overrun { 0x20 } else { 0 }
    }
}

// ---------------------------------------------------------------------------
// 8155 RAM / I/O ports / timer
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct I8155 {
    pub ram: [u8; 256],
    pub csr: u8,
    pub out: [u8; 3],
    /// Levels presented on the port pins by the outside world (connector X2).
    pub ext_in: [u8; 3],
    count_reg: u16, // 14-bit count + 2 mode bits in 15..14
    running: bool,
    stop_after_tc: bool,
    period: u64,
    /// CPU cycle at which the next terminal count happens.
    next_tc: u64,
    tc_flag: bool,
    last_start: u64,
}

impl Default for I8155 {
    fn default() -> Self {
        I8155 {
            ram: [0; 256],
            csr: 0,
            out: [0; 3],
            ext_in: [0xFF; 3],
            count_reg: 0,
            running: false,
            stop_after_tc: false,
            period: 2,
            next_tc: 0,
            tc_flag: false,
            last_start: 0,
        }
    }
}

impl I8155 {
    pub fn reset(&mut self) {
        self.csr = 0;
        self.running = false;
        self.stop_after_tc = false;
    }

    fn mode(&self) -> u8 {
        (self.count_reg >> 14) as u8
    }

    pub fn timer_running(&self) -> bool {
        self.running
    }

    /// (14-bit count, 2-bit mode) as last written to ports 44H/45H.
    pub fn timer_setting(&self) -> (u16, u8) {
        (self.count_reg & 0x3FFF, self.mode())
    }

    pub fn write_io(&mut self, reg: u8, v: u8, now: u64) {
        match reg {
            0 => {
                self.csr = v;
                match v >> 6 {
                    1 => self.running = false,
                    2 => {
                        if self.running {
                            self.stop_after_tc = true
                        }
                    }
                    3 => {
                        self.period = ((self.count_reg & 0x3FFF) as u64).max(2);
                        self.running = true;
                        self.stop_after_tc = false;
                        self.last_start = now;
                        self.next_tc = now + self.period;
                    }
                    _ => {}
                }
            }
            1..=3 => self.out[reg as usize - 1] = v,
            4 => self.count_reg = (self.count_reg & 0xFF00) | v as u16,
            5 => self.count_reg = (self.count_reg & 0x00FF) | (v as u16) << 8,
            _ => {}
        }
    }

    pub fn read_io(&mut self, reg: u8, now: u64) -> u8 {
        match reg {
            0 => {
                let v = (self.tc_flag as u8) << 6;
                self.tc_flag = false;
                v
            }
            1 => {
                if self.csr & 1 != 0 { self.out[0] } else { self.ext_in[0] }
            }
            2 => {
                if self.csr & 2 != 0 { self.out[1] } else { self.ext_in[1] }
            }
            3 => {
                if self.csr & 0x0C == 0x0C { self.out[2] } else { self.ext_in[2] | 0xC0 }
            }
            4 | 5 => {
                let left = if self.running {
                    (self.next_tc.saturating_sub(now)).min(self.period) as u16
                } else {
                    self.count_reg & 0x3FFF
                };
                let v = (left & 0x3FFF) | (self.count_reg & 0xC000);
                if reg == 4 { v as u8 } else { (v >> 8) as u8 }
            }
            _ => 0xFF,
        }
    }

    /// Advance the timer. Returns true when a terminal count happened
    /// strictly before `now` (that edge drives TRAP).
    pub fn update(&mut self, now: u64) -> bool {
        let mut fired = false;
        while self.running && self.next_tc < now {
            fired = true;
            self.tc_flag = true;
            let continuous = self.mode() & 1 != 0;
            if !continuous || self.stop_after_tc {
                self.running = false;
            } else {
                self.next_tc += self.period;
            }
        }
        fired
    }
}

// ---------------------------------------------------------------------------
// 8251 USART (main serial port, DB9-1)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct I8251 {
    expect_mode: bool,
    pub mode: u8,
    pub cmd: u8,
    /// Clock fed to TxC/RxC by the J17 divider, in Hz.
    pub clock_hz: u32,
    cpu_hz: u32,
    tx_buf: Option<u8>,
    tx_shift_end: u64,
    rx_buf: Option<u8>,
    rx_queue: VecDeque<u8>,
    /// Keyboard input: released one character at a time, only after the
    /// monitor has stopped printing (its output routine discards input).
    typed: VecDeque<u8>,
    next_rx: u64,
    overrun: bool,
    /// Characters transmitted by the CPU, collected by the terminal.
    pub tx_out: VecDeque<u8>,
}

impl I8251 {
    pub fn new(cpu_hz: u32, clock_hz: u32) -> Self {
        I8251 {
            expect_mode: true,
            mode: 0,
            cmd: 0,
            clock_hz,
            cpu_hz,
            tx_buf: None,
            tx_shift_end: 0,
            rx_buf: None,
            rx_queue: VecDeque::new(),
            typed: VecDeque::new(),
            next_rx: 0,
            overrun: false,
            tx_out: VecDeque::new(),
        }
    }

    pub fn reset(&mut self) {
        self.expect_mode = true;
        self.cmd = 0;
        self.tx_buf = None;
        self.rx_buf = None;
        self.overrun = false;
    }

    /// Baud rate resulting from the J17 clock and the programmed factor.
    pub fn baud(&self) -> u32 {
        let div = match self.mode & 3 {
            1 => 1,
            3 => 64,
            _ => 16,
        };
        self.clock_hz / div
    }

    fn char_cycles(&self) -> u64 {
        let data = 5 + ((self.mode >> 2) & 3) as u64;
        let parity = ((self.mode >> 4) & 1) as u64;
        // stop bits in half-bit units: 01=1, 10=1.5, 11=2
        let stop2 = match self.mode >> 6 {
            2 => 3,
            3 => 4,
            _ => 2,
        };
        let half_bits = 2 * (1 + data + parity) + stop2;
        (self.cpu_hz as u64 * half_bits) / (2 * self.baud().max(1) as u64)
    }

    /// Queue characters arriving from the terminal (RxD).
    pub fn receive(&mut self, bytes: &[u8]) {
        self.rx_queue.extend(bytes);
    }

    /// Queue characters typed by a person (paced, see `typed`).
    pub fn type_chars(&mut self, bytes: &[u8]) {
        self.typed.extend(bytes);
    }

    pub fn rx_pending(&self) -> usize {
        self.rx_queue.len() + self.typed.len()
    }

    pub fn clear_rx(&mut self) {
        self.rx_queue.clear();
        self.typed.clear();
    }

    pub fn update(&mut self, now: u64) {
        if let Some(c) = self.tx_buf {
            if now >= self.tx_shift_end {
                self.tx_buf = None;
                self.tx_shift_end = now + self.char_cycles();
                self.tx_out.push_back(c);
            }
        }
        if self.rx_queue.is_empty() && self.rx_buf.is_none() && !self.typed.is_empty() {
            let quiet = 3 * self.char_cycles();
            if self.tx_buf.is_none() && now >= self.tx_shift_end + quiet && now >= self.next_rx {
                let c = self.typed.pop_front().unwrap();
                self.rx_queue.push_back(c);
            }
        }
        if now >= self.next_rx && !self.rx_queue.is_empty() {
            if self.cmd & 0x04 != 0 {
                let c = self.rx_queue.pop_front().unwrap();
                if self.rx_buf.is_some() {
                    self.overrun = true;
                }
                self.rx_buf = Some(c);
            } else {
                // receiver disabled: characters are lost
                self.rx_queue.pop_front();
            }
            self.next_rx = now + self.char_cycles();
        }
    }

    pub fn write_ctl(&mut self, v: u8) {
        if self.expect_mode {
            self.mode = v;
            self.expect_mode = false;
        } else {
            self.cmd = v;
            if v & 0x10 != 0 {
                self.overrun = false;
            }
            if v & 0x40 != 0 {
                self.reset();
            }
        }
    }

    pub fn write_data(&mut self, v: u8, now: u64) {
        if self.cmd & 0x01 == 0 {
            return;
        }
        if self.tx_buf.is_none() && now >= self.tx_shift_end {
            self.tx_shift_end = now + self.char_cycles();
            self.tx_out.push_back(v);
        } else {
            self.tx_buf = Some(v);
        }
    }

    pub fn read_data(&mut self) -> u8 {
        self.rx_buf.take().unwrap_or(0)
    }

    pub fn read_status(&self, now: u64) -> u8 {
        let tx_rdy = self.tx_buf.is_none() && self.cmd & 0x01 != 0;
        let tx_empty = self.tx_buf.is_none() && now >= self.tx_shift_end;
        (tx_rdy as u8)
            | (self.rx_buf.is_some() as u8) << 1
            | (tx_empty as u8) << 2
            | (self.overrun as u8) << 4
    }
}

// ---------------------------------------------------------------------------
// Bit-level serial line into SID (DB9-2, "secondary serial port")
// ---------------------------------------------------------------------------

/// Sends bytes as 1 start + 8 data (LSB first) + `stop` stop bits, back to
/// back, with exact timing in CPU cycles. The first byte of a burst starts
/// when the line is first sampled after it was queued.
#[derive(Clone, Debug)]
pub struct SerialLine {
    queue: VecDeque<u8>,
    cur: Option<u8>,
    frame_start: u64,
    pub bit_cycles: u64,
    pub stop_bits: u64,
}

impl SerialLine {
    pub fn new(bit_cycles: u64) -> Self {
        SerialLine { queue: VecDeque::new(), cur: None, frame_start: 0, bit_cycles, stop_bits: 2 }
    }

    pub fn send(&mut self, bytes: &[u8]) {
        self.queue.extend(bytes);
    }

    pub fn pending(&self) -> usize {
        self.queue.len() + self.cur.is_some() as usize
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.cur = None;
    }

    fn frame_len(&self) -> u64 {
        (10 + self.stop_bits) * self.bit_cycles
    }

    /// Line level at `now` (true = mark/idle).
    pub fn level(&mut self, now: u64) -> bool {
        loop {
            match self.cur {
                None => {
                    let Some(c) = self.queue.pop_front() else { return true };
                    self.cur = Some(c);
                    self.frame_start = now;
                }
                Some(_) if now >= self.frame_start + self.frame_len() => {
                    let end = self.frame_start + self.frame_len();
                    self.cur = self.queue.pop_front();
                    self.frame_start = end;
                }
                Some(c) => {
                    let bit = (now - self.frame_start) / self.bit_cycles;
                    return match bit {
                        0 => false,
                        1..=8 => (c >> (bit - 1)) & 1 != 0,
                        _ => true,
                    };
                }
            }
        }
    }
}
