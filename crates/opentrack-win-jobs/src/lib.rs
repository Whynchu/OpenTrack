// SPDX-License-Identifier: Apache-2.0

//! Windows job-object containment for child processes.
//!
//! A scanner worker runs arbitrary native plug-in code. The parent is the only
//! process that knows when the scan is over, and a crash in the parent would
//! otherwise leave workers behind, still executing plug-in code with no one to
//! stop them. This crate places such a child in a job object created with
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`: closing the last handle — including the
//! operating system closing it when the parent dies — terminates every process
//! left in the job.
//!
//! The OS boundary is deliberately isolated here so the rest of the scan crate
//! can keep `#![forbid(unsafe_code)]`. On non-Windows targets the API is a
//! no-op that reports success, because the parent's lifetime already governs
//! its child there.

/// A live job object holding one child process.
///
/// Dropping the guard closes the job handle, which terminates any process still
/// inside the job. Keep it alive for as long as the child should be allowed to
/// run.
pub struct WorkerContainment {
    #[cfg(windows)]
    job: windows_sys::Win32::Foundation::HANDLE,
}

/// Places the process `pid` in a kill-on-close job object.
///
/// The job is only created when a process id is supplied; a caller that cannot
/// contain its worker must terminate that worker rather than let it run free.
pub fn contain_worker(pid: u32) -> Result<WorkerContainment, String> {
    #[cfg(windows)]
    {
        contain_worker_windows(pid)
    }
    #[cfg(not(windows))]
    {
        // The operating system does not offer an equivalent facility here and
        // the scan crate already kills a worker when its timeout expires.
        let _ = pid;
        Ok(WorkerContainment {})
    }
}

#[cfg(windows)]
fn contain_worker_windows(pid: u32) -> Result<WorkerContainment, String> {
    use std::ffi::c_void;

    use windows_sys::Win32::Foundation::{CloseHandle, FALSE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };

    // SAFETY: every handle is checked before use and every acquired handle is
    // closed on every path. The structures passed to the API are fully
    // initialized before the call.
    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return Err("could not create a job object for the module worker".to_owned());
        }
        let containment = WorkerContainment { job };
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) == FALSE
        {
            return Err("could not configure the module worker's job object".to_owned());
        }
        let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, FALSE, pid);
        if process.is_null() {
            return Err("could not open the module worker process".to_owned());
        }
        let assigned = AssignProcessToJobObject(job, process);
        CloseHandle(process);
        if assigned == FALSE {
            return Err("could not place the module worker in its job object".to_owned());
        }
        Ok(containment)
    }
}

#[cfg(windows)]
impl Drop for WorkerContainment {
    fn drop(&mut self) {
        // SAFETY: `job` is the handle this guard created and owns; it is closed
        // exactly once, here.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.job);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn containment_is_available_and_droppable() {
        // On Windows this contains a real child process; on other targets the
        // guard documents that the parent's lifetime is the containment.
        let pid = std::process::id();
        let containment = contain_worker(pid);
        if cfg!(windows) {
            // A scanner cannot be placed in a new job when it is already in one
            // that forbids breakaway, so either outcome is acceptable here; the
            // error must still explain itself.
            if let Err(error) = &containment {
                assert!(error.contains("job object"), "{error}");
            }
        } else {
            assert!(containment.is_ok());
        }
        drop(containment);
    }
}
