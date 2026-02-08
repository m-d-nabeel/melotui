mod input_processor;
mod ipc_bridge;
mod logging;
mod runtime;

fn main() -> anyhow::Result<()> {
    let _log_guard = logging::init();

    std::panic::set_hook(Box::new(|panic| {
        tracing::error!("panic: {}", panic);
    }));

    runtime::run()?;

    Ok(())
}
