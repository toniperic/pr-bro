use std::sync::Mutex;

static BUFFER: Mutex<Option<Vec<String>>> = Mutex::new(None);

/// Activate buffering. While active, `warn!()` calls store messages
/// instead of printing to stderr.
pub fn activate() {
    *BUFFER.lock().unwrap() = Some(Vec::new());
}

/// Deactivate buffering and return all collected messages.
pub fn drain() -> Vec<String> {
    BUFFER.lock().unwrap().take().unwrap_or_default()
}

/// Write a warning message. If buffering is active the message is stored;
/// otherwise it is printed to stderr immediately.
pub fn warn(msg: String) {
    let mut guard = BUFFER.lock().unwrap();
    if let Some(buf) = guard.as_mut() {
        buf.push(msg);
    } else {
        drop(guard);
        eprintln!("{}", msg);
    }
}

/// Convenience macro that works like `eprintln!` but routes through the
/// stderr buffer when it is active.
#[macro_export]
macro_rules! buffered_eprintln {
    ($($arg:tt)*) => {
        $crate::stderr_buffer::warn(format!($($arg)*))
    };
}
