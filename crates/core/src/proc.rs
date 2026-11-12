//! Process trees for agents and terminals. Killing the tree kills every process the child
//! started, so nothing is left behind.
//!
//! - Windows: a Job Object with kill-on-close. If FocusIDE itself dies, Windows closes the job
//!   handle and the whole tree goes with it.
//! - Unix: the child leads a new process group and the tree is killed with `killpg`. On Linux the
//!   child also gets a parent-death signal.

use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::{Child, Command};

/// Builds a command with the platform flags a tree needs. Call before `spawn`.
pub fn prepare(cmd: &mut Command) {
    cmd.kill_on_drop(true);
    #[cfg(windows)]
    {
        // No console window for children of a GUI app.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    {
        cmd.process_group(0);
        #[cfg(target_os = "linux")]
        unsafe {
            cmd.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
                Ok(())
            });
        }
    }
}

pub struct ProcessTree {
    child: Child,
    pid: u32,
    #[cfg(windows)]
    job: win::Job,
}

impl ProcessTree {
    /// Spawns `cmd` with piped stdio as the root of a new tree.
    pub fn spawn_piped(mut cmd: Command) -> Result<Self> {
        prepare(&mut cmd);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        Self::spawn(cmd)
    }

    /// Spawns `cmd` as the root of a new tree. Stdio is whatever `cmd` has set.
    pub fn spawn(mut cmd: Command) -> Result<Self> {
        prepare(&mut cmd);
        #[cfg(windows)]
        let job = win::Job::new().context("create job object")?;
        let child = cmd.spawn().context("start process")?;
        let pid = child.id().unwrap_or(0);
        #[cfg(windows)]
        {
            // The child runs before it joins the job. A process it starts in that first moment
            // would escape the job; the adapter loads for far longer than this takes.
            if let Some(handle) = child.raw_handle() {
                job.assign(handle as _).context("add process to job")?;
            }
        }
        Ok(Self {
            child,
            pid,
            #[cfg(windows)]
            job,
        })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }

    /// Ids of every live process in the tree. Windows reads the job. Unix returns the group
    /// leader only (enough to confirm the leader is gone).
    pub fn pids(&self) -> Vec<u32> {
        #[cfg(windows)]
        {
            self.job.pids()
        }
        #[cfg(unix)]
        {
            if is_alive(self.pid) {
                vec![self.pid]
            } else {
                vec![]
            }
        }
    }

    /// Kills every process in the tree and waits for the root to exit.
    pub async fn kill(&mut self) {
        #[cfg(windows)]
        self.job.terminate();
        #[cfg(unix)]
        unsafe {
            libc::killpg(self.pid as i32, libc::SIGKILL);
        }
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
    }

    /// Exit status if the root has exited.
    pub fn try_wait(&mut self) -> Option<std::process::ExitStatus> {
        self.child.try_wait().ok().flatten()
    }
}

/// A tree rooted at a process someone else started (the terminal shell from a PTY library).
pub struct PidTree {
    #[allow(dead_code)]
    pid: u32,
    #[cfg(windows)]
    job: Option<win::Job>,
}

impl PidTree {
    /// Puts `pid` in a kill-on-close job (Windows). On Unix the process must lead its own
    /// group (PTY children do: they call setsid).
    pub fn attach(pid: u32) -> Self {
        #[cfg(windows)]
        {
            let job = win::Job::new().ok().filter(|j| j.assign_pid(pid).is_ok());
            Self { pid, job }
        }
        #[cfg(unix)]
        {
            Self { pid }
        }
    }

    pub fn kill(&self) {
        #[cfg(windows)]
        if let Some(j) = &self.job {
            j.terminate();
        }
        #[cfg(unix)]
        unsafe {
            libc::killpg(self.pid as i32, libc::SIGKILL);
        }
    }
}

/// True if a process with this id is running.
pub fn is_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        win::is_alive(pid)
    }
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, STILL_ACTIVE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicProcessIdList,
        JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
        TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    pub struct Job(HANDLE);

    // The handle is only used through thread-safe Win32 calls.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    impl Job {
        pub fn new() -> std::io::Result<Self> {
            unsafe {
                let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if h.is_null() {
                    return Err(std::io::Error::last_os_error());
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let ok = SetInformationJobObject(
                    h,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if ok == 0 {
                    let e = std::io::Error::last_os_error();
                    CloseHandle(h);
                    return Err(e);
                }
                Ok(Self(h))
            }
        }

        pub fn assign(&self, process: HANDLE) -> std::io::Result<()> {
            unsafe {
                if AssignProcessToJobObject(self.0, process) == 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        }

        pub fn assign_pid(&self, pid: u32) -> std::io::Result<()> {
            use windows_sys::Win32::System::Threading::{PROCESS_SET_QUOTA, PROCESS_TERMINATE};
            unsafe {
                let h = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
                if h.is_null() {
                    return Err(std::io::Error::last_os_error());
                }
                let r = self.assign(h);
                CloseHandle(h);
                r
            }
        }

        pub fn terminate(&self) {
            unsafe {
                TerminateJobObject(self.0, 1);
            }
        }

        pub fn pids(&self) -> Vec<u32> {
            // JOBOBJECT_BASIC_PROCESS_ID_LIST: two u32 counts, then usize ids.
            const MAX: usize = 1024;
            let mut buf = vec![0usize; 2 + MAX];
            unsafe {
                let ok = QueryInformationJobObject(
                    self.0,
                    JobObjectBasicProcessIdList,
                    buf.as_mut_ptr() as *mut c_void,
                    (buf.len() * std::mem::size_of::<usize>()) as u32,
                    std::ptr::null_mut(),
                );
                if ok == 0 {
                    return vec![];
                }
                let header = buf.as_ptr() as *const u32;
                let count = *header.add(1) as usize;
                let ids = (header as *const u8).add(8) as *const usize;
                (0..count.min(MAX)).map(|i| *ids.add(i) as u32).collect()
            }
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    pub fn is_alive(pid: u32) -> bool {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return false;
            }
            let mut code = 0u32;
            let ok = GetExitCodeProcess(h, &mut code);
            CloseHandle(h);
            ok != 0 && code == STILL_ACTIVE as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn kill_takes_the_whole_tree() {
        // A shell that starts a long-running grandchild.
        #[cfg(windows)]
        let cmd = {
            let mut c = Command::new("cmd.exe");
            c.args(["/c", "ping -n 30 127.0.0.1 > NUL"]);
            c
        };
        #[cfg(unix)]
        let cmd = {
            let mut c = Command::new("sh");
            c.args(["-c", "sleep 30 & sleep 30"]);
            c
        };
        let mut tree = ProcessTree::spawn_piped(cmd).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        let before = tree.pids();
        assert!(!before.is_empty());
        tree.kill().await;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        for pid in before {
            assert!(!is_alive(pid), "process {pid} is still running");
        }
    }
}
