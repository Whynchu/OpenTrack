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
    last_system_ticks: Option<(u64, u64, u64)>,
    cores: f32,
    cpu_percent: f32,
    process_cpu_percent: f32,
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
                self.process_cpu_percent =
                    (cpu_seconds / elapsed / f64::from(self.cores.max(1.0)) * 100.0) as f32;
            }
            self.last_cpu_ticks = Some(ticks);
        }

        if let Some((idle, kernel, user)) = sample_system_cpu_ticks() {
            if let Some((previous_idle, previous_kernel, previous_user)) = self.last_system_ticks {
                let idle_delta = idle.wrapping_sub(previous_idle) as f64;
                let total_delta = kernel.wrapping_sub(previous_kernel) as f64
                    + user.wrapping_sub(previous_user) as f64;
                if total_delta > 0.0 {
                    self.cpu_percent =
                        ((1.0 - idle_delta / total_delta) * 100.0).clamp(0.0, 100.0) as f32;
                }
            }
            self.last_system_ticks = Some((idle, kernel, user));
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

    pub(crate) fn process_cpu_percent(&self) -> f32 {
        self.process_cpu_percent.clamp(0.0, 100.0)
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

#[cfg(target_os = "linux")]
fn sample_process_cpu_ticks() -> Option<u64> {
    parse_process_cpu_ticks(&std::fs::read_to_string("/proc/self/stat").ok()?)
}

/// `/proc` reports CPU time in `USER_HZ` (100 ticks per second); the sampler
/// works in the 100 ns units the Windows path produces.
#[cfg(target_os = "linux")]
const USER_HZ_TO_100NS: u64 = 100_000;

#[cfg(target_os = "linux")]
fn parse_process_cpu_ticks(stat: &str) -> Option<u64> {
    // The command name is parenthesised and may contain spaces and parentheses.
    let (_, fields) = stat.rsplit_once(')')?;
    let fields = fields.split_whitespace().collect::<Vec<_>>();
    // The remainder starts at field 3 (state); utime is 14 and stime 15.
    let utime = fields.get(11)?.parse::<u64>().ok()?;
    let stime = fields.get(12)?.parse::<u64>().ok()?;
    Some((utime + stime) * USER_HZ_TO_100NS)
}

#[cfg(not(any(windows, target_os = "linux")))]
fn sample_process_cpu_ticks() -> Option<u64> {
    None
}

#[cfg(windows)]
fn sample_system_cpu_ticks() -> Option<(u64, u64, u64)> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::GetSystemTimes;

    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut idle = zero;
    let mut kernel = zero;
    let mut user = zero;
    // SAFETY: all pointers reference valid locals for the duration of the call.
    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    if ok == 0 {
        return None;
    }
    let ticks =
        |value: FILETIME| (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime);
    Some((ticks(idle), ticks(kernel), ticks(user)))
}

#[cfg(target_os = "linux")]
fn sample_system_cpu_ticks() -> Option<(u64, u64, u64)> {
    parse_system_cpu_ticks(&std::fs::read_to_string("/proc/stat").ok()?)
}

/// Returns `(idle, kernel, user)` in 100 ns units, matching the Windows
/// `GetSystemTimes` shape where kernel time includes idle time.
#[cfg(target_os = "linux")]
fn parse_system_cpu_ticks(stat: &str) -> Option<(u64, u64, u64)> {
    let line = stat.lines().find(|line| line.starts_with("cpu "))?;
    let values = line
        .split_whitespace()
        .skip(1)
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let field = |index: usize| values.get(index).copied().unwrap_or(0);
    let user = field(0);
    let nice = field(1);
    let system = field(2);
    let idle = field(3) + field(4);
    let irq = field(5);
    let softirq = field(6);
    let steal = field(7);
    let kernel = nice + system + irq + softirq + steal + idle;
    Some((
        idle * USER_HZ_TO_100NS,
        kernel * USER_HZ_TO_100NS,
        user * USER_HZ_TO_100NS,
    ))
}

#[cfg(not(any(windows, target_os = "linux")))]
fn sample_system_cpu_ticks() -> Option<(u64, u64, u64)> {
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

#[cfg(target_os = "linux")]
fn sample_memory() -> Option<(u32, u64, u64, u64)> {
    parse_memory(
        &std::fs::read_to_string("/proc/meminfo").ok()?,
        &std::fs::read_to_string("/proc/self/status").ok()?,
    )
}

/// Returns `(load percent, total, used, process resident bytes)`.
#[cfg(target_os = "linux")]
fn parse_memory(meminfo: &str, status: &str) -> Option<(u32, u64, u64, u64)> {
    let meminfo_kib = |key: &str| -> Option<u64> {
        meminfo.lines().find_map(|line| {
            let rest = line.strip_prefix(key)?;
            rest.split_whitespace().next()?.parse::<u64>().ok()
        })
    };
    let total = meminfo_kib("MemTotal:")?.saturating_mul(1024);
    // MemAvailable is what the kernel considers reclaimable, which is the same
    // notion the Windows gauge uses (total minus available physical memory).
    let available = meminfo_kib("MemAvailable:")
        .or_else(|| meminfo_kib("MemFree:"))?
        .saturating_mul(1024);
    if total == 0 {
        return None;
    }
    let used = total.saturating_sub(available);
    let load = (used.saturating_mul(100) / total) as u32;
    let process = status
        .lines()
        .find_map(|line| {
            let rest = line.strip_prefix("VmRSS:")?;
            rest.split_whitespace().next()?.parse::<u64>().ok()
        })
        .unwrap_or(0)
        .saturating_mul(1024);
    Some((load, total, used, process))
}

#[cfg(not(any(windows, target_os = "linux")))]
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

    #[cfg(target_os = "linux")]
    #[test]
    fn process_ticks_are_read_past_a_command_name_with_spaces() {
        // Field 14/15 are utime/stime: 120 + 30 ticks at 100 Hz.
        let stat = "1234 (Open Track (dev)) S 1 1234 1234 0 -1 4194304 100 0 0 0 120 30 0 0 20 0 12 0 0 0 0 0\n";
        assert_eq!(parse_process_cpu_ticks(stat), Some(150 * USER_HZ_TO_100NS));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn system_ticks_put_idle_inside_kernel_like_windows() {
        // user nice system idle iowait irq softirq steal
        let stat = "cpu  100 10 40 800 50 5 5 0 0 0\n";
        let (idle, kernel, user) = parse_system_cpu_ticks(stat).unwrap();
        assert_eq!(user, 100 * USER_HZ_TO_100NS);
        assert_eq!(idle, 850 * USER_HZ_TO_100NS);
        // Kernel time includes idle, as GetSystemTimes reports it.
        assert_eq!(kernel, 910 * USER_HZ_TO_100NS);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_samplers_read_the_running_system() {
        assert!(sample_process_cpu_ticks().is_some());
        assert!(sample_system_cpu_ticks().is_some());
        let (load, total, used, process) = sample_memory().expect("/proc is readable");
        assert!(total > 0);
        assert!(used <= total);
        assert!(load <= 100);
        assert!(process > 0, "the test process has a resident set");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn memory_uses_available_and_the_process_resident_set() {
        let meminfo = "MemTotal:       16000000 kB\nMemFree:         1000000 kB\nMemAvailable:    4000000 kB\n";
        let status = "Name:\tOpenTrack\nVmRSS:\t  204800 kB\n";
        let (load, total, used, process) = parse_memory(meminfo, status).unwrap();
        assert_eq!(total, 16_000_000 * 1024);
        assert_eq!(used, 12_000_000 * 1024);
        assert_eq!(load, 75);
        assert_eq!(process, 204_800 * 1024);
    }
}
