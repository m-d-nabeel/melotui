mod input_processor;
mod logging;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod mpris;
mod runtime;

fn main() {
    let log_guard = logging::init();

    std::panic::set_hook(Box::new(|panic| {
        let _ = melotui_tui::restore_terminal();
        tracing::error!("panic: {}", panic);
        eprintln!("panic: {}", panic);
    }));

    let res = runtime::run();

    if let Err(e) = res {
        eprintln!("Error: {}", e);
        // Flush the buffered log writer before bailing; `process::exit` does not
        // run destructors, so without this the lines explaining the failure are
        // exactly the ones that never get written.
        drop(log_guard);
        std::process::exit(1);
    }
}
