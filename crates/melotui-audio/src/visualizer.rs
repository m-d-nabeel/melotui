use melotui_core::state::SpectrumStyle;
use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, Split},
};

use crate::{
    cava::{Cava, CavaConsumer},
    fft::SpectrumAnalyzer,
};

// Helper alias
type AudioConsumer = <HeapRb<f32> as Split>::Cons;

enum Backend {
    Internal(SpectrumAnalyzer<AudioConsumer>),
    External(CavaConsumer),
    None,
}

pub struct VisualizerManager {
    backend: Backend,
    // Idle consumers needed to "drain" unused streams so buffers don't overflow
    idle_internal: Option<AudioConsumer>,
    idle_cava: Option<CavaConsumer>,
    cava_process: Option<Cava>,
    cava_scratch: Vec<f32>,
    enabled: bool,
    style: SpectrumStyle,
}

impl VisualizerManager {
    pub fn new() -> Self {
        let available = Cava::is_available();
        if available {
            tracing::info!("CAVA detected.");
        } else {
            tracing::info!("CAVA not found.");
        }

        Self {
            backend: Backend::None,
            idle_internal: None,
            idle_cava: None,
            cava_process: None,
            cava_scratch: vec![0.0; 200],
            enabled: false,
            style: if available {
                SpectrumStyle::Cava
            } else {
                SpectrumStyle::Default
            },
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.reconcile();
    }

    pub fn set_style(&mut self, style: SpectrumStyle) {
        self.style = style;
        self.enabled = true;
        self.reconcile();
    }

    pub fn attach_source(&mut self, consumer: AudioConsumer) {
        self.detach_source(); // Reset current
        self.idle_internal = Some(consumer);
        self.reconcile();
    }

    pub fn detach_source(&mut self) {
        self.take_active_to_idle();
        self.idle_internal = None; // Drop the internal stream completely
        self.backend = Backend::None;
    }

    /// Returns spectrum data if available
    pub fn tick(&mut self) -> Option<Vec<f32>> {
        // 1. Always drain idle internal stream
        if let Some(consumer) = &mut self.idle_internal {
            while consumer.try_pop().is_some() {}
        }

        if !self.enabled {
            return None;
        }

        match &mut self.backend {
            Backend::Internal(analyzer) => analyzer.next_spectrum(),
            Backend::External(consumer) => {
                let mut valid_frame = false;
                // Drain strategy: read everything, keep latest
                while consumer.occupied_len() >= 200 {
                    let _ = consumer.pop_slice(&mut self.cava_scratch);
                    valid_frame = true;
                }
                if valid_frame {
                    Some(self.cava_scratch.clone())
                } else {
                    None
                }
            }
            Backend::None => None,
        }
    }

    fn reconcile(&mut self) {
        if !self.enabled {
            self.take_active_to_idle();
            self.backend = Backend::None;
            return;
        }

        match self.style {
            SpectrumStyle::Cava => {
                if !matches!(self.backend, Backend::External(_)) {
                    tracing::info!("Switching Visualizer to CAVA");
                    self.take_active_to_idle();
                    self.ensure_cava_started();

                    if let Some(cons) = self.idle_cava.take() {
                        self.backend = Backend::External(cons);
                    }
                }
            }
            _ => {
                if !matches!(self.backend, Backend::Internal(_)) {
                    tracing::info!("Switching Visualizer to Internal");
                    self.take_active_to_idle();
                    if let Some(cons) = self.idle_internal.take() {
                        self.backend = Backend::Internal(SpectrumAnalyzer::new(cons));
                    } else {
                        self.backend = Backend::None;
                    }
                }
            }
        }
    }

    fn take_active_to_idle(&mut self) {
        let old = std::mem::replace(&mut self.backend, Backend::None);
        match old {
            Backend::Internal(analyzer) => {
                self.idle_internal = Some(analyzer.into_consumer());
            }
            Backend::External(consumer) => {
                self.idle_cava = Some(consumer);
            }
            _ => {}
        }
    }

    fn ensure_cava_started(&mut self) {
        if self.cava_process.is_none() {
            match Cava::new() {
                Ok((process, consumer)) => {
                    self.cava_process = Some(process);
                    self.idle_cava = Some(consumer);
                }
                Err(e) => {
                    tracing::error!("Failed to start Cava: {}", e);
                    // Fallback to internal immediately if possible
                    if let Some(cons) = self.idle_internal.take() {
                        self.backend = Backend::Internal(SpectrumAnalyzer::new(cons));
                    }
                }
            }
        }
    }
}
