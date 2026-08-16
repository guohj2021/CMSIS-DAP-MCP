//! Minimal Ctrl-C handling for monitor loops.
//!
//! The handler only sets a flag; each monitor loop checks it and stops
//! cleanly (REPL returns to the prompt, one-shot commands exit 0).

use std::sync::atomic::{AtomicBool, Ordering};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
pub fn install() {
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;

    unsafe extern "system" fn handler(_event: u32) -> i32 {
        INTERRUPTED.store(true, Ordering::SeqCst);
        // Keep running; the poll loop observes the flag and stops.
        1
    }
    unsafe {
        let _ = SetConsoleCtrlHandler(Some(handler), 1);
    }
}

#[cfg(unix)]
pub fn install() {
    unsafe extern "C" fn handler(_sig: libc::c_int) {
        INTERRUPTED.store(true, Ordering::SeqCst);
    }
    unsafe {
        libc::signal(libc::SIGINT, handler as libc::sighandler_t);
    }
}

pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Reset the flag (e.g. before starting a new monitor from the REPL).
pub fn reset() {
    INTERRUPTED.store(false, Ordering::SeqCst);
}
