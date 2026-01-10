use melotui_core::utils::SpectrumSource;
use std::sync::{Arc, RwLock};

/// The container for our single shared buffer.
#[derive(Clone)]
struct SharedState {
    data: Arc<RwLock<Vec<f32>>>,
}

/// The Producer: Held by the Audio Worker.
/// It "pushes" updates by overwriting the shared state.
#[derive(Clone)]
pub struct SpectrumProducer {
    state: SharedState,
}

impl SpectrumProducer {
    pub fn push(&self, incoming: &[f32]) {
        // Acquiring write lock is very fast (nanoseconds) unless UI is reading.
        if let Ok(mut guard) = self.state.data.write() {
            if guard.len() != incoming.len() {
                guard.resize(incoming.len(), 0.0);
            }
            // Direct memory copy - highly optimized by LLVM
            guard.copy_from_slice(incoming);
        }
    }
}

/// The Consumer: Held by the UI (AppState).
/// It "pulls" the latest state whenever the UI frame updates.
pub struct SpectrumConsumer {
    state: SharedState,
}

impl SpectrumSource for SpectrumConsumer {
    fn get_latest(&self, out: &mut [f32]) -> usize {
        if let Ok(guard) = self.state.data.read() {
            let len = guard.len().min(out.len());
            // Copy from shared memory to UI memory
            out[..len].copy_from_slice(&guard[..len]);
            len
        } else {
            0
        }
    }
}

/// Factory function to create the pair.
pub fn create_bridge() -> (SpectrumProducer, SpectrumConsumer) {
    let data = Arc::new(RwLock::new(Vec::new()));
    let state = SharedState { data };

    (
        SpectrumProducer {
            state: state.clone(),
        },
        SpectrumConsumer { state },
    )
}
