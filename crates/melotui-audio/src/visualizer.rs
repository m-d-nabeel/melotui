use melotui_core::state::SpectrumStyle;
use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, Split},
};

use crate::{
    cava::{Cava, CavaConsumer},
    fft::SpectrumAnalyzer,
};

// helper alias
type AudioConsumer = <HeapRb<f32> as Split>::Cons;

enum Backend {
    Internal(SpectrumAnalyzer<AudioConsumer>),
    External(CavaConsumer),
    None,
}

pub struct VisualizerManager {
    backend: Backend,
    // idle consumers to drain unused streams
    idle_internal: Option<AudioConsumer>,
    idle_cava: Option<CavaConsumer>,
    cava_process: Option<Cava>,
    cava_scratch: Vec<f32>,
    enabled: bool,
    style: SpectrumStyle,
    fft_size: usize,
    sample_rate: u32,
}

impl VisualizerManager {
    pub fn new(fft_size: usize) -> Self {
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
            style: SpectrumStyle::default(),
            fft_size,
            sample_rate: 44100,
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

    pub fn attach_source(&mut self, consumer: AudioConsumer, sample_rate: u32) {
        self.take_active_to_idle();
        self.idle_internal = Some(consumer);
        self.sample_rate = sample_rate;
        self.reconcile();
    }

    /// After a seek: the analyser starts over on the audio from there.
    pub fn restart(&mut self) {
        if let Backend::Internal(analyzer) = &mut self.backend {
            analyzer.restart();
        }
    }

    pub fn detach_source(&mut self) {
        self.take_active_to_idle();
        self.idle_internal = None; // drop internal stream
        self.backend = Backend::None;
    }

    /// Returns spectrum data if available
    pub fn tick(&mut self) -> Option<Vec<f32>> {
        // drain idle internal stream
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
                // drain strategy: keep latest
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
                        self.backend = Backend::Internal(SpectrumAnalyzer::new(
                            cons,
                            self.fft_size,
                            self.sample_rate,
                        ));
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
                    // fallback to internal
                    if let Some(cons) = self.idle_internal.take() {
                        self.backend = Backend::Internal(SpectrumAnalyzer::new(
                            cons,
                            self.fft_size,
                            self.sample_rate,
                        ));
                    }
                }
            }
        }
    }
}
