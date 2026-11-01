//! Process CPU and memory sampling for the transport meters.
//!
//! Sampling is throttled and control-thread only. CPU is OpenTrack's own process
//! usage normalised across logical cores (all cores busy = 100%), which is what
//! a DAW user wants to watch; memory reports the process working set, with the
//! system-wide load available for the gauge.

use std::time::{Duration, Instant};

const SAMPLE_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Default)]
pub(crate) struct SystemStats {
    last_sample: Option<Instant>,
    last_cpu_ticks: Option<u64>,
    cores: f32,
    cpu_percent: f32,
    ram_load_percent: f32,
    ram_used_bytes: u64,
    ram_total_bytes: u64,
    process_bytes: u64,
}

impl SystemStats {
    pub(crate) fn refresh(&mut self) {
        let now = Instant::now();
        if self
            .last_sample
            .is_some_and(|last| now.duration_since(last) < SAMPLE_INTERVAL)
        {
            return;
        }
        let elapsed = self
            .last_sample
            .map(|last| now.duration_since(last).as_secs_f64())
            .unwrap_or(0.0);
        self.last_sample = Some(now);
        if self.cores <= 0.0 {
            self.cores = std::thread::available_parallelism()
                .map(|count| count.get() as f32)
                .unwrap_or(1.0);
        }

        if let Some(ticks) = sample_process_cpu_ticks() {
            if let Some(previous) = self.last_cpu_ticks
                && elapsed > 0.0
            {
                let delta = ticks.saturating_sub(previous) as f64;
                // FILETIME ticks are 100 ns; one CPU-second per 1e7 ticks.
                let cpu_seconds = delta / 1.0e7;
                self.cpu_percent =
                    (cpu_seconds / elapsed / f64::from(self.cores.max(1.0)) * 100.0) as f32;
            }
            self.last_cpu_ticks = Some(ticks);
        }

        if let Some((load, total, used, process)) = sample_memory() {
            self.ram_load_percent = load as f32;
            self.ram_total_bytes = total;
            self.ram_used_bytes = used;
            self.process_bytes = process;
        }
    }

    pub(crate) fn cpu_percent(&self) -> f32 {
        self.cpu_percent.clamp(0.0, 100.0)
    }

    pub(crate) fn ram_load_percent(&self) -> f32 {
        self.ram_load_percent.clamp(0.0, 100.0)
    }

    pub(crate) fn ram_used_bytes(&self) -> u64 {
        self.ram_used_bytes
    }

    pub(crate) fn ram_total_bytes(&self) -> u64 {
        self.ram_total_bytes
    }

    pub(crate) fn process_bytes(&self) -> u64 {
        self.process_bytes
    }
}

/// Compact byte count for the transport readout: MB below one gigabyte.
pub(crate) fn format_memory(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * MB;
    let bytes = bytes as f64;
    if bytes >= GB {
        format!("{:.1}G", bytes / GB)
    } else {
        format!("{:.0}M", bytes / MB)
    }
}

#[cfg(windows)]
fn sample_process_cpu_ticks() -> Option<u64> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    let mut creation = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit = creation;
    let mut kernel = creation;
    let mut user = creation;
    // SAFETY: all pointers reference valid locals for the duration of the call.
    let ok = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    if ok == 0 {
        return None;
    }
    let ticks =
        |value: FILETIME| (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime);
    Some(ticks(kernel) + ticks(user))
}

#[cfg(not(windows))]
fn sample_process_cpu_ticks() -> Option<u64> {
    None
}

#[cfg(windows)]
fn sample_memory() -> Option<(u32, u64, u64, u64)> {
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    // SAFETY: zero is a valid starting state for both plain-old-data structs;
    // their length/size fields are set immediately below.
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    // SAFETY: the struct is valid and its length field is set.
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return None;
    }

    // SAFETY: as above.
    let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    counters.cb = size;
    // SAFETY: the struct is valid and its size field is set.
    let process = if unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) } != 0
    {
        counters.WorkingSetSize as u64
    } else {
        0
    };

    let total = status.ullTotalPhys;
    let used = status.ullTotalPhys.saturating_sub(status.ullAvailPhys);
    Some((status.dwMemoryLoad, total, used, process))
}

#[cfg(not(windows))]
fn sample_memory() -> Option<(u32, u64, u64, u64)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_readout_switches_between_megabytes_and_gigabytes() {
        assert_eq!(format_memory(512 * 1024 * 1024), "512M");
        assert_eq!(format_memory(2 * 1024 * 1024 * 1024), "2.0G");
        assert_eq!(format_memory(1536 * 1024 * 1024), "1.5G");
        assert_eq!(format_memory(0), "0M");
    }

    #[test]
    fn a_fresh_sampler_reports_no_load() {
        let stats = SystemStats::default();
        assert_eq!(stats.cpu_percent(), 0.0);
        assert_eq!(stats.ram_load_percent(), 0.0);
    }
}
