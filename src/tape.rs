//! Cassette recorder connected to SOD (MIC) and SID (EAR) through J18.
//!
//! Recording turns the SOD square wave into audio samples (with a high-pass
//! filter, like the AC-coupled recorder input). Playback feeds SID through a
//! model of the board's LM324 circuit: amplifier, diode rectifier and
//! comparator, i.e. an envelope detector — SID is 1 while a tone is present
//! and 0 during silence. That's what the monitor's BITIN/TAPEIN expect.

use crate::machine::CPU_HZ;

pub const RATE: u32 = 44_100;
const AMPLITUDE: f32 = 20_000.0;
/// High-pass coefficient (~35 Hz corner at 44.1 kHz).
const HP: f32 = 0.995;
/// How long the envelope detector stays high after the last zero crossing.
const HOLD_SECONDS: f64 = 0.0005;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TapeState {
    Stopped,
    Playing { start_cycle: u64, start_pos: f64 },
    Recording { start_cycle: u64, start_pos: usize },
}

#[derive(Clone, Debug)]
pub struct Tape {
    /// Audio, mono 16-bit at `rate`.
    pub samples: Vec<i16>,
    pub rate: u32,
    env: Vec<bool>,
    /// Position in samples while stopped.
    pos: f64,
    pub state: TapeState,
    /// Unsaved recording on the tape.
    pub modified: bool,
    pub name: Option<String>,
    // recording state
    sod: bool,
    /// Cycle of the last SOD change while recording (for the MIC light).
    pub last_sod_edge: u64,
    written: usize,
    x_prev: f32,
    y_prev: f32,
}

impl Default for Tape {
    fn default() -> Self {
        Tape {
            samples: Vec::new(),
            rate: RATE,
            env: Vec::new(),
            pos: 0.0,
            state: TapeState::Stopped,
            modified: false,
            name: None,
            sod: false,
            last_sod_edge: 0,
            written: 0,
            x_prev: 0.0,
            y_prev: 0.0,
        }
    }
}

impl Tape {
    fn cycles_to_samples(&self, cycles: u64) -> f64 {
        cycles as f64 * self.rate as f64 / CPU_HZ as f64
    }

    /// Current position in samples.
    pub fn position(&self, now: u64) -> f64 {
        match self.state {
            TapeState::Stopped => self.pos,
            TapeState::Playing { start_cycle, start_pos } => {
                start_pos + self.cycles_to_samples(now.saturating_sub(start_cycle))
            }
            TapeState::Recording { .. } => self.written as f64,
        }
    }

    pub fn seconds(&self, samples: f64) -> f64 {
        samples / self.rate as f64
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn play(&mut self, now: u64) {
        self.stop(now);
        if self.pos as usize >= self.samples.len() {
            self.pos = 0.0;
        }
        self.state = TapeState::Playing { start_cycle: now, start_pos: self.pos };
    }

    pub fn record(&mut self, now: u64, sod: bool) {
        self.stop(now);
        let start = (self.pos as usize).min(self.samples.len());
        self.written = start;
        self.sod = sod;
        self.x_prev = if sod { AMPLITUDE } else { -AMPLITUDE };
        self.y_prev = 0.0;
        self.state = TapeState::Recording { start_cycle: now, start_pos: start };
    }

    pub fn stop(&mut self, now: u64) {
        match self.state {
            TapeState::Stopped => {}
            TapeState::Playing { .. } => {
                self.pos = self.position(now).min(self.samples.len() as f64);
            }
            TapeState::Recording { .. } => {
                self.fill_to(now);
                self.stop_click();
                self.pos = self.written as f64;
                self.modified = true;
                self.compute_envelope();
            }
        }
        self.state = TapeState::Stopped;
    }

    pub fn rewind(&mut self, now: u64) {
        self.stop(now);
        self.pos = 0.0;
    }

    /// Move the stopped tape to `seconds`.
    pub fn seek(&mut self, seconds: f64) {
        if self.state == TapeState::Stopped {
            self.pos = (seconds * self.rate as f64).clamp(0.0, self.samples.len() as f64);
        }
    }

    pub fn clear(&mut self) {
        *self = Tape::default();
    }

    /// Called regularly: writes silence while recording, stops at the end
    /// of the tape while playing.
    pub fn update(&mut self, now: u64) {
        match self.state {
            TapeState::Recording { .. } => self.fill_to(now),
            TapeState::Playing { .. } if self.position(now) >= self.samples.len() as f64 => {
                self.pos = self.samples.len() as f64;
                self.state = TapeState::Stopped;
            }
            _ => {}
        }
    }

    /// SOD changed (only called while J18 routes SOD to the tape).
    pub fn sod(&mut self, level: bool, now: u64) {
        if let TapeState::Recording { .. } = self.state {
            self.fill_to(now);
            if level != self.sod {
                self.last_sod_edge = now;
            }
        }
        self.sod = level;
    }

    /// SID level: the comparator output for the audio under the head.
    pub fn sid(&self, now: u64) -> bool {
        match self.state {
            TapeState::Playing { .. } => {
                let i = self.position(now) as usize;
                self.env.get(i).copied().unwrap_or(false)
            }
            _ => false,
        }
    }

    /// The transient a recorder writes when recording stops. The monitor's
    /// read routine needs it: after the checksum byte it waits for the next
    /// sound to begin before returning.
    fn stop_click(&mut self) {
        let half = (self.rate / 2000).max(1) as usize; // ~1 kHz
        let n = (self.rate / 50) as usize; // 20 ms
        for i in 0..n + (self.rate / 20) as usize {
            let x = if i < n && (i / half) % 2 == 0 { AMPLITUDE } else if i < n { -AMPLITUDE } else { self.x_prev };
            let y = HP * (self.y_prev + x - self.x_prev);
            self.x_prev = x;
            self.y_prev = y;
            let s = y.clamp(-32767.0, 32767.0) as i16;
            if self.written < self.samples.len() {
                self.samples[self.written] = s;
            } else {
                self.samples.push(s);
            }
            self.written += 1;
        }
    }

    fn fill_to(&mut self, now: u64) {
        let TapeState::Recording { start_cycle, start_pos } = self.state else { return };
        let target = start_pos + self.cycles_to_samples(now.saturating_sub(start_cycle)) as usize;
        while self.written < target {
            let x = if self.sod { AMPLITUDE } else { -AMPLITUDE };
            let y = HP * (self.y_prev + x - self.x_prev);
            self.x_prev = x;
            self.y_prev = y;
            let s = y.clamp(-32767.0, 32767.0) as i16;
            if self.written < self.samples.len() {
                self.samples[self.written] = s;
            } else {
                self.samples.push(s);
            }
            self.written += 1;
        }
    }

    /// Envelope detector: high while zero crossings (with hysteresis) keep
    /// coming, released HOLD_SECONDS after the last one.
    pub fn compute_envelope(&mut self) {
        let peak = self.samples.iter().map(|s| (*s as i32).abs()).max().unwrap_or(0);
        let th = (peak / 8).max(300);
        let hold = (HOLD_SECONDS * self.rate as f64).ceil() as usize;
        let mut env = vec![false; self.samples.len()];
        let mut positive = false;
        let mut last_cross: Option<usize> = None;
        for (i, &s) in self.samples.iter().enumerate() {
            let s = s as i32;
            if (positive && s < -th) || (!positive && s > th) {
                positive = !positive;
                last_cross = Some(i);
            }
            env[i] = last_cross.is_some_and(|c| i - c <= hold);
        }
        self.env = env;
    }

    // ----- WAV files -----------------------------------------------------------

    pub fn to_wav(&self) -> Vec<u8> {
        let data_len = (self.samples.len() * 2) as u32;
        let mut v = Vec::with_capacity(44 + data_len as usize);
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&1u16.to_le_bytes()); // mono
        v.extend_from_slice(&self.rate.to_le_bytes());
        v.extend_from_slice(&(self.rate * 2).to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        for s in &self.samples {
            v.extend_from_slice(&s.to_le_bytes());
        }
        v
    }

    pub fn from_wav(bytes: &[u8]) -> Result<Tape, String> {
        let bad = || "not a WAV file".to_string();
        if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(bad());
        }
        let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        let u32_at = |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        let mut i = 12;
        let mut fmt: Option<(u16, u16, u32, u16)> = None;
        let mut data: Option<&[u8]> = None;
        while i + 8 <= bytes.len() {
            let id = &bytes[i..i + 4];
            let len = u32_at(i + 4) as usize;
            let body = &bytes[i + 8..(i + 8 + len).min(bytes.len())];
            if id == b"fmt " && body.len() >= 16 {
                let mut format = u16::from_le_bytes([body[0], body[1]]);
                if format == 0xFFFE && body.len() >= 26 {
                    format = u16::from_le_bytes([body[24], body[25]]);
                }
                fmt = Some((
                    format,
                    u16::from_le_bytes([body[2], body[3]]),
                    u32::from_le_bytes([body[4], body[5], body[6], body[7]]),
                    u16::from_le_bytes([body[14], body[15]]),
                ));
            } else if id == b"data" {
                data = Some(body);
            }
            i += 8 + len + (len & 1);
        }
        let _ = u16_at;
        let (format, channels, rate, bits) = fmt.ok_or("WAV file has no format chunk")?;
        let data = data.ok_or("WAV file has no audio data")?;
        if channels == 0 || rate == 0 {
            return Err(bad());
        }
        let bps = (bits as usize).div_ceil(8);
        let frame = bps * channels as usize;
        let sample = |b: &[u8]| -> Result<f32, String> {
            Ok(match (format, bits) {
                (1, 8) => (b[0] as f32 - 128.0) * 256.0,
                (1, 16) => i16::from_le_bytes([b[0], b[1]]) as f32,
                (1, 24) => (i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 16) as f32,
                (1, 32) => (i32::from_le_bytes([b[0], b[1], b[2], b[3]]) >> 16) as f32,
                (3, 32) => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) * 32767.0,
                _ => return Err(format!("unsupported WAV format ({format}, {bits} bits)")),
            })
        };
        let mut samples = Vec::with_capacity(data.len() / frame.max(1));
        for f in data.chunks_exact(frame) {
            let mut acc = 0.0;
            for c in 0..channels as usize {
                acc += sample(&f[c * bps..])?;
            }
            samples.push((acc / channels as f32).clamp(-32767.0, 32767.0) as i16);
        }
        let mut t = Tape { samples, rate, ..Tape::default() };
        t.compute_envelope();
        Ok(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_roundtrip_and_envelope() {
        let mut t = Tape::default();
        t.record(0, false);
        // 3.4 kHz tone for 10 ms, then 10 ms silence
        let half = (CPU_HZ / 6800) as u64;
        let mut now = 0;
        let mut lvl = false;
        while now < CPU_HZ as u64 / 100 {
            now += half;
            lvl = !lvl;
            t.sod(lvl, now);
        }
        t.update(now + CPU_HZ as u64 / 100);
        t.stop(now + CPU_HZ as u64 / 100);
        let t2 = Tape::from_wav(&t.to_wav()).unwrap();
        assert_eq!(t2.samples, t.samples);
        let n = t2.samples.len();
        assert!(n > 800, "{n}");
        assert!(t2.env[200] && t2.env[400], "tone detected");
        assert!(!t2.env[800], "silence detected");
        assert!(!t2.env[n - 50], "silence after the stop click");
    }
}
