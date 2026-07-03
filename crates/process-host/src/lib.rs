use std::ffi::CStr;
use std::os::raw::c_char;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::io::IntoRawHandle;

// On Windows we must retain the process HANDLE, because:
// - WaitForSingleObject / TerminateProcess need a HANDLE, not just a PID
// - Once we close the handle the kernel may reuse the PID
#[cfg(windows)]
mod win {
    use std::collections::HashMap;
    use std::sync::{LazyLock, Mutex};

    pub static HANDLES: LazyLock<Mutex<HashMap<u32, usize>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
}

// --- helper -----------------------------------------------------------------

unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

// --- public API -------------------------------------------------------------

/// Spawn `executable` with a single argument `args`.
/// Returns the child PID on success, -1 on failure.
///
/// # Safety
/// `executable` and `args` must be valid null-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn process_host_spawn(executable: *const c_char, args: *const c_char) -> i32 {
    let exe = match cstr_to_str(executable) {
        Some(s) => s,
        None => return -1,
    };
    let args_str = cstr_to_str(args).unwrap_or("");

    let mut cmd = Command::new(exe);
    if !args_str.is_empty() {
        cmd.arg(args_str);
    }

    // Set working directory to the executable's directory so that
    // relative resource paths (e.g. CEF locales/) resolve correctly.
    if let Some(dir) = std::path::Path::new(exe).parent() {
        if dir != std::path::Path::new("") {
            cmd.current_dir(dir);
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    match cmd.spawn() {
        Ok(child) => {
            let pid = child.id();

            #[cfg(windows)]
            {
                // Extract the raw HANDLE before dropping Child so we can
                // wait / terminate by PID later without reopening the handle.
                let handle = child.into_raw_handle() as usize;
                if let Ok(mut map) = win::HANDLES.lock() {
                    map.insert(pid, handle);
                }
            }

            #[cfg(unix)]
            {
                // Drop Child — closes stdio pipes but leaves the process running.
                // The kernel tracks it by PID; we reap it with waitpid().
                drop(child);
            }

            pid as i32
        }
        Err(_) => -1,
    }
}

/// Block until the process exits.
/// Returns the exit code, or -1 on error.
#[no_mangle]
pub extern "C" fn process_host_wait(pid: i32) -> i32 {
    if pid <= 0 {
        return -1;
    }

    #[cfg(unix)]
    {
        let mut status: libc::c_int = 0;
        let r = unsafe { libc::waitpid(pid, &mut status, 0) };
        if r > 0 {
            // waitpid succeeded: process has exited, read the exit code.
            if libc::WIFEXITED(status) {
                return libc::WEXITSTATUS(status);
            }
            if libc::WIFSIGNALED(status) {
                // Killed by a signal; return 128 + signal (Unix convention).
                return 128 + libc::WTERMSIG(status);
            }
            return -1;
        }

        // waitpid returned -1 (ECHILD or EINTR).
        //
        // Unity on Linux typically sets SIGCHLD = SIG_IGN to suppress zombie
        // processes from its own internal child management. Under SIG_IGN the
        // kernel auto-reaps children without going through a zombie state, and
        // waitpid() returns ECHILD immediately — even while the child is still
        // running — because the parent-child wait relationship is never recorded.
        //
        // Fall back to kill(pid, 0) polling: it probes process existence without
        // sending a real signal and works regardless of SIGCHLD disposition.
        loop {
            let probe = unsafe { libc::kill(pid, 0) };
            if probe < 0 {
                // ESRCH (or any error): process no longer exists.
                return 0;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
        use windows_sys::Win32::System::Threading::{WaitForSingleObject, INFINITE};

        // Grab the handle but do NOT remove it yet; process_host_kill may need it
        // concurrently if Stop() races with natural process exit.
        let handle = match win::HANDLES.lock() {
            Ok(map) => map.get(&(pid as u32)).copied(),
            Err(_) => return -1,
        };
        let h = match handle {
            Some(h) => h,
            None => return -1,
        };

        // Wait without holding the Mutex so process_host_kill can proceed.
        let result = unsafe { WaitForSingleObject(h as _, INFINITE) };

        // Clean up: remove from map and close handle.
        if let Ok(mut map) = win::HANDLES.lock() {
            map.remove(&(pid as u32));
        }
        unsafe { CloseHandle(h as _) };

        if result == WAIT_OBJECT_0 {
            0
        } else {
            -1
        }
    }
}

/// Check whether the process is still running.
/// Returns 1 if running, 0 if not, -1 on error.
#[no_mangle]
pub extern "C" fn process_host_is_running(pid: i32) -> i32 {
    if pid <= 0 {
        return 0;
    }

    #[cfg(unix)]
    {
        // kill(pid, 0) probes existence without sending a real signal.
        let r = unsafe { libc::kill(pid, 0) };
        if r == 0 {
            1
        } else {
            0
        }
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::WAIT_TIMEOUT;
        use windows_sys::Win32::System::Threading::WaitForSingleObject;

        let handle = match win::HANDLES.lock() {
            Ok(map) => map.get(&(pid as u32)).copied(),
            Err(_) => return -1,
        };
        match handle {
            Some(h) => {
                let r = unsafe { WaitForSingleObject(h as _, 0) };
                // WAIT_TIMEOUT means still running; other values mean exited
                if r == WAIT_TIMEOUT {
                    1
                } else {
                    0
                }
            }
            None => 0,
        }
    }
}

/// Terminate the process.
/// `force = 0`: graceful (SIGTERM on Unix, TerminateProcess on Windows)
/// `force = 1`: forceful (SIGKILL on Unix, TerminateProcess on Windows)
/// Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn process_host_kill(pid: i32, force: i32) -> i32 {
    if pid <= 0 {
        return -1;
    }

    #[cfg(unix)]
    {
        let signal = if force != 0 {
            libc::SIGKILL
        } else {
            libc::SIGTERM
        };
        let r = unsafe { libc::kill(pid, signal) };
        if r == 0 {
            0
        } else {
            -1
        }
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::TerminateProcess;

        let _ = force; // Windows always uses TerminateProcess

        // Grab handle without removing; process_host_wait owns cleanup.
        let handle = match win::HANDLES.lock() {
            Ok(map) => map.get(&(pid as u32)).copied(),
            Err(_) => return -1,
        };
        match handle {
            Some(h) => {
                let r = unsafe { TerminateProcess(h as _, 1) };
                if r != 0 {
                    0
                } else {
                    -1
                }
            }
            None => 0, // already exited, process_host_wait cleaned up
        }
    }
}
