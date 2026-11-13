//! Running code that may panic without the panic taking the app with it.
//!
//! A tag library handed a strange enough file can panic. Where that would
//! cost only the one file — a scan, tagging a download — the call is
//! [`guarded`]: the panic is caught, and the app's panic hook, which hands
//! the terminal back and ends the process for a panic nobody catches, is
//! told through [`is_guarded`] to leave this one alone.

use std::cell::Cell;
use std::panic::{UnwindSafe, catch_unwind};

thread_local! {
    static GUARDED: Cell<bool> = const { Cell::new(false) };
}

/// Runs `f`, catching a panic in it: `Err` with what it panicked with.
pub fn guarded<T>(f: impl FnOnce() -> T + UnwindSafe) -> std::thread::Result<T> {
    let before = GUARDED.replace(true);
    let result = catch_unwind(f);
    GUARDED.set(before);
    result
}

/// Whether the code running on this thread is inside [`guarded`], and a
/// panic in it will be caught.
pub fn is_guarded() -> bool {
    GUARDED.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_guarded_panic_is_caught_and_known_to_be() {
        assert!(!is_guarded());
        assert_eq!(guarded(is_guarded).ok(), Some(true));
        assert!(!is_guarded(), "the guard outlived the call");

        // Seen by a panic hook, as the app's sees it.
        let seen = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new({
            let seen = seen.clone();
            move |_| {
                // Only ever set: another test panicking meanwhile, on its
                // own thread and unguarded, must not undo it.
                seen.fetch_or(is_guarded(), std::sync::atomic::Ordering::SeqCst);
            }
        }));
        let result = guarded(|| panic!("an odd file"));
        std::panic::set_hook(previous);

        assert!(result.is_err());
        assert!(seen.load(std::sync::atomic::Ordering::SeqCst));
        assert!(!is_guarded(), "the guard outlived the panic");
    }

    #[test]
    fn a_guard_inside_a_guard_leaves_the_outer_one_standing() {
        let inside = guarded(|| {
            let _ = guarded(|| ());
            is_guarded()
        });
        assert_eq!(inside.ok(), Some(true));
    }
}
