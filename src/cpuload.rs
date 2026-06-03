//! Self-process CPU-load sampler.
//!
//! Reads `/proc/self/stat` and computes CPU time consumed by this process
//! between successive calls to [`CpuLoad::sample`], then divides by wall-clock
//! elapsed to get a percentage. 
//!
//! Linux-only (it reads `/proc`). Returns `None` on the very first sample
//! (there is no baseline yet) or if the stat file is unreadable.

use std::fs;
use std::io;
use std::time::Instant;

/// Samples process CPU time between calls and reports load as a percentage
/// of one CPU (example: `150.0` on a multi-core machine means the process is
/// driving 1.5 cores flat out).
#[derive(Debug)]
pub struct CpuLoad {
    clock_ticks_per_sec: u64,
    last: Option<(Instant, u64)>,
}

impl CpuLoad {
    /// Build a new sampler. Reads `_SC_CLK_TCK` once.
    pub fn new() -> Self {
        // SAFETY: sysconf with _SC_CLK_TCK is always safe to call.
        let hz = unsafe { libc_clk_tck() };
        Self {
            clock_ticks_per_sec: hz.max(1),
            last: None,
        }
    }

    /// Sample `/proc/self/stat`. Returns `Some(percent)` once a baseline has
    /// been established (i.e. on the second and subsequent calls), otherwise
    /// `None`. Percent is CPU-time-used / wall-clock-elapsed * 100, so it can
    /// exceed 100 on a multi-threaded workload.
    pub fn sample(&mut self) -> io::Result<Option<f32>> {
        let now = Instant::now();
        let ticks = read_self_ticks()?;
        let result = match self.last {
            Some((prev_t, prev_ticks)) => {
                let dt = now.duration_since(prev_t).as_secs_f32();
                if dt <= 0.0 {
                    None
                } else {
                    let dticks = ticks.saturating_sub(prev_ticks) as f32;
                    let cpu_secs = dticks / self.clock_ticks_per_sec as f32;
                    Some((cpu_secs / dt) * 100.0)
                }
            }
            None => None,
        };
        self.last = Some((now, ticks));
        Ok(result)
    }
}

impl Default for CpuLoad {
    fn default() -> Self {
        Self::new()
    }
}

/// Read `utime + stime` from `/proc/self/stat` in clock ticks.
///
/// The stat file format is well-defined: fields are space-separated, but the
/// second field (comm) is `(name)` and may contain spaces. Parse past the
/// closing `)` to sidestep that, then pick positions 14 (utime) and 15 (stime)
/// from the remainder - see `proc(5)`.
fn read_self_ticks() -> io::Result<u64> {
    let raw = fs::read_to_string("/proc/self/stat")?;
    let close = raw
        .rfind(')')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no ')' in /proc/self/stat"))?;
    let rest = &raw[close + 1..];
    let fields: Vec<&str> = rest.split_ascii_whitespace().collect();
    // After the comm close-paren, field indices shift: position 0 is `state`,
    // so utime is at index 11 and stime at index 12.
    let utime: u64 = fields
        .get(11)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "utime missing"))?;
    let stime: u64 = fields
        .get(12)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "stime missing"))?;
    Ok(utime + stime)
}

unsafe fn libc_clk_tck() -> u64 {
    // _SC_CLK_TCK is 2 on glibc/musl.
    unsafe extern "C" {
        fn sysconf(name: core::ffi::c_int) -> core::ffi::c_long;
    }
    const _SC_CLK_TCK: core::ffi::c_int = 2;
    let v = unsafe { sysconf(_SC_CLK_TCK) };
    if v <= 0 { 100 } else { v as u64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sample_returns_none() {
        let mut cpu = CpuLoad::new();
        assert!(cpu.sample().unwrap().is_none());
    }

    #[test]
    fn second_sample_returns_a_value() {
        let mut cpu = CpuLoad::new();
        let _ = cpu.sample().unwrap();
        // Spin briefly so the interval is non-zero.
        let start = Instant::now();
        while start.elapsed().as_millis() < 20 {
            std::hint::black_box(start.elapsed());
        }
        let pct = cpu.sample().unwrap();
        assert!(pct.is_some(), "expected Some(percent) after baseline");
        // Can be anywhere from near-0 to multi-hundred under load; just
        // sanity-check non-negative and finite.
        let v = pct.unwrap();
        assert!(v.is_finite() && v >= 0.0, "percent was {v}");
    }

    #[test]
    fn read_self_ticks_is_nondecreasing() {
        let a = read_self_ticks().unwrap();
        for _ in 0..1_000 {
            std::hint::black_box(());
        }
        let b = read_self_ticks().unwrap();
        assert!(b >= a, "ticks went backwards: {a} -> {b}");
    }
}
