mod input_processor;
mod ipc_bridge;
mod logging;
mod runtime;

fn main() -> anyhow::Result<()> {
    let _log_guard = logging::init();

    std::panic::set_hook(Box::new(|panic| {
        let _ = melotui_tui::restore_terminal();
        tracing::error!("panic: {}", panic);
        eprintln!("panic: {}", panic);
    }));

    runtime::run()?;

    Ok(())
}
