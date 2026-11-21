//! RAM limit and priority for the game process (Windows). Other systems: no-op with a warning.
//!
//! soft   = hard working-set maximum: physical RAM stays under the cap, the rest is paged out
//!          (slower when the game needs more, but it keeps running).
//! strict = Job Object process-memory limit: allocations beyond the cap fail, which usually
//!          closes the game. The job also kills the game if Strata itself is killed.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Soft,
    Strict,
}

pub fn parse_mode(s: &str) -> Mode {
    if s.eq_ignore_ascii_case("strict") {
        Mode::Strict
    } else {
        Mode::Soft
    }
}

pub fn gb_to_bytes(gb: u32) -> usize {
    (gb as usize) * 1024 * 1024 * 1024
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;

    pub type Handle = *mut c_void;

    pub const QUOTA_LIMITS_HARDWS_MIN_DISABLE: u32 = 0x2;
    pub const QUOTA_LIMITS_HARDWS_MAX_ENABLE: u32 = 0x4;
    pub const JOB_OBJECT_LIMIT_PROCESS_MEMORY: u32 = 0x0000_0100;
    pub const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;
    pub const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: i32 = 9;
    pub const NORMAL_PRIORITY_CLASS: u32 = 0x0000_0020;
    pub const ABOVE_NORMAL_PRIORITY_CLASS: u32 = 0x0000_8000;
    pub const HIGH_PRIORITY_CLASS: u32 = 0x0000_0080;

    #[repr(C)]
    #[derive(Default)]
    pub struct BasicLimit {
        pub per_process_user_time_limit: i64,
        pub per_job_user_time_limit: i64,
        pub limit_flags: u32,
        pub minimum_working_set_size: usize,
        pub maximum_working_set_size: usize,
        pub active_process_limit: u32,
        pub affinity: usize,
        pub priority_class: u32,
        pub scheduling_class: u32,
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct IoCounters {
        pub counters: [u64; 6],
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct ExtendedLimit {
        pub basic: BasicLimit,
        pub io: IoCounters,
        pub process_memory_limit: usize,
        pub job_memory_limit: usize,
        pub peak_process_memory_used: usize,
        pub peak_job_memory_used: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn SetProcessWorkingSetSizeEx(process: Handle, min: usize, max: usize, flags: u32) -> i32;
        pub fn CreateJobObjectW(attrs: *mut c_void, name: *const u16) -> Handle;
        pub fn SetInformationJobObject(job: Handle, class: i32, info: *mut c_void, len: u32) -> i32;
        pub fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        pub fn SetPriorityClass(process: Handle, class: u32) -> i32;
        pub fn GetLastError() -> u32;
    }
}

/// Keeps the job handle alive for the life of the game (closing it would end the game).
pub struct Guard {
    #[cfg(windows)]
    job: Option<win::Handle>,
}

// SAFETY: the job handle is only used to keep the job object alive and is never shared.
unsafe impl Send for Guard {}

impl Drop for Guard {
    fn drop(&mut self) {
        // The handle is intentionally leaked: the OS closes it when Strata exits, which then
        // ends the game (kill-on-close). Closing it here would kill a game that is still running.
        #[cfg(windows)]
        let _ = self.job.take();
    }
}

#[cfg(windows)]
pub fn apply(child: &std::process::Child, limit_gb: Option<(u32, Mode)>, priority: &str) -> (Guard, Vec<String>) {
    use std::os::windows::io::AsRawHandle;
    use win::*;
    let mut notes = Vec::new();
    let process = child.as_raw_handle() as Handle;

    let class = match priority {
        "high" => HIGH_PRIORITY_CLASS,
        "above_normal" => ABOVE_NORMAL_PRIORITY_CLASS,
        _ => NORMAL_PRIORITY_CLASS,
    };
    if class != NORMAL_PRIORITY_CLASS {
        if unsafe { SetPriorityClass(process, class) } != 0 {
            notes.push(format!("Process priority set to {priority}."));
        } else {
            notes.push(format!("Could not set priority (error {}).", unsafe { GetLastError() }));
        }
    }

    let mut job = None;
    if let Some((gb, Mode::Strict)) = limit_gb {
      unsafe {
        let j = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
        if !j.is_null() {
            let mut info = ExtendedLimit::default();
            info.basic.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
            info.process_memory_limit = gb_to_bytes(gb);
            let ok = SetInformationJobObject(
                j,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
                &mut info as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<ExtendedLimit>() as u32,
            ) != 0
                && AssignProcessToJobObject(j, process) != 0;
            if ok {
                job = Some(j);
                notes.push(format!("Strict RAM limit: {gb} GB (the game closes if it needs more)."));
            } else {
                notes.push(format!("Strict RAM limit not applied (error {}).", GetLastError()));
            }
        }
      }
    }

    if let Some((gb, Mode::Soft)) = limit_gb {
        let max = gb_to_bytes(gb);
        let min = 64 * 1024 * 1024;
        let flags = QUOTA_LIMITS_HARDWS_MAX_ENABLE | QUOTA_LIMITS_HARDWS_MIN_DISABLE;
        if unsafe { SetProcessWorkingSetSizeEx(process, min, max, flags) } != 0 {
            notes.push(format!("Soft RAM limit: {gb} GB of physical memory (extra goes to the page file)."));
        } else {
            notes.push(format!(
                "Soft RAM limit could not be applied (error {}). Try running Strata as administrator.",
                unsafe { GetLastError() }
            ));
        }
    }
    (Guard { job }, notes)
}

#[cfg(not(windows))]
pub fn apply(_child: &std::process::Child, limit_gb: Option<(u32, Mode)>, priority: &str) -> (Guard, Vec<String>) {
    let mut notes = Vec::new();
    if limit_gb.is_some() {
        notes.push("RAM limit is only available on Windows.".into());
    }
    if priority != "normal" {
        notes.push("Priority control is only available on Windows.".into());
    }
    (Guard {}, notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_and_sizes() {
        assert_eq!(parse_mode("STRICT"), Mode::Strict);
        assert_eq!(parse_mode("whatever"), Mode::Soft);
        assert_eq!(gb_to_bytes(8), 8 * 1024 * 1024 * 1024);
    }

    #[cfg(windows)]
    #[test]
    fn struct_layout_matches_windows() {
        // JOBOBJECT_EXTENDED_LIMIT_INFORMATION is 144 bytes on x64.
        assert_eq!(std::mem::size_of::<win::ExtendedLimit>(), 144);
    }
}
