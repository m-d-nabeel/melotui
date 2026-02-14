use melotui_core::utils::SpectrumSource;
use std::sync::{Arc, RwLock};

/// shared spectrum buffer
#[derive(Clone)]
struct SharedState {
    data: Arc<RwLock<Vec<f32>>>,
}

/// producer (audio worker)
/// push updates
#[derive(Clone)]
pub struct SpectrumProducer {
    state: SharedState,
}

impl SpectrumProducer {
    pub fn push(&self, incoming: &[f32]) {
        // fast write lock
        if let Ok(mut guard) = self.state.data.write() {
            if guard.len() != incoming.len() {
                guard.resize(incoming.len(), 0.0);
            }
            // mem copy
            guard.copy_from_slice(incoming);
        }
    }
}

/// consumer (app state)
/// pull latest state
pub struct SpectrumConsumer {
    state: SharedState,
}

impl SpectrumSource for SpectrumConsumer {
    fn get_latest(&self, out: &mut [f32]) -> usize {
        if let Ok(guard) = self.state.data.read() {
            let len = guard.len().min(out.len());
            // shared -> ui copy
            out[..len].copy_from_slice(&guard[..len]);
            len
        } else {
            0
        }
    }
}

/// factory for bridge
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
