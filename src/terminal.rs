//! A simple "glass teletype" screen for the serial terminal.

pub struct Terminal {
    pub lines: Vec<Vec<char>>,
    pub col: usize,
    pub cols: usize,
    max_lines: usize,
}

impl Default for Terminal {
    fn default() -> Self {
        Terminal { lines: vec![Vec::new()], col: 0, cols: 80, max_lines: 3000 }
    }
}

impl Terminal {
    pub fn clear(&mut self) {
        self.lines = vec![Vec::new()];
        self.col = 0;
    }

    fn newline(&mut self) {
        self.lines.push(Vec::new());
        if self.lines.len() > self.max_lines {
            let extra = self.lines.len() - self.max_lines;
            self.lines.drain(..extra);
        }
    }

    pub fn feed(&mut self, b: u8) {
        match b & 0x7F {
            b'\r' => self.col = 0,
            b'\n' => self.newline(),
            0x08 => self.col = self.col.saturating_sub(1),
            b'\t' => {
                let next = (self.col / 8 + 1) * 8;
                while self.col < next.min(self.cols) {
                    self.put(' ');
                }
            }
            0x0C => self.clear(),
            c if c >= 0x20 && c < 0x7F => self.put(c as char),
            _ => {}
        }
    }

    fn put(&mut self, ch: char) {
        if self.col >= self.cols {
            self.newline();
            self.col = 0;
        }
        let line = self.lines.last_mut().unwrap();
        while line.len() < self.col {
            line.push(' ');
        }
        if self.col < line.len() {
            line[self.col] = ch;
        } else {
            line.push(ch);
        }
        self.col += 1;
    }

    pub fn text(&self) -> String {
        let mut s = String::new();
        for (i, l) in self.lines.iter().enumerate() {
            if i > 0 {
                s.push('\n');
            }
            s.extend(l.iter());
        }
        s
    }
}
