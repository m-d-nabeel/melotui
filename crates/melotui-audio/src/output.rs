//! The audio output device: which one, opening it, and knowing when it has
//! gone.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use rodio::cpal::{
    self, ErrorKind,
    traits::{DeviceTrait, HostTrait},
};

/// An open output, and what it is called.
pub struct Output {
    pub sink: rodio::MixerDeviceSink,
    pub name: String,
}

/// The period to ask cpal for so the whole buffer comes out at `frames`.
/// cpal 0.18 takes `BufferSize::Fixed` as the period and allocates two of
/// them; handing it the whole buffer doubled it and had the device pull a
/// buffer's worth at a time, every 171 ms at 8192 frames, which the
/// visualizer drew as a burst followed by a freeze.
fn period_for(frames: u32) -> u32 {
    (frames / 2).max(1)
}

/// Errors after which the stream is gone and has to be opened again. A
/// rerouted default device (`DeviceChanged`) carries on by itself, and an
/// underrun is a glitch, not a loss.
fn is_loss(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::DeviceNotAvailable | ErrorKind::StreamInvalidated | ErrorKind::HostUnavailable
    )
}

fn name_of(device: &cpal::Device) -> String {
    let name = device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "unknown device".to_string());
    // ALSA joins a card and its device with ", ", leaving a dangling comma
    // for a device with no name of its own.
    name.trim_end_matches([',', ' ']).to_string()
}

/// Whether a device is one only plumbing would want: ALSA's null device,
/// which plays nothing.
fn is_plumbing(name: &str) -> bool {
    name.starts_with("Discard all samples")
}

/// The names of the output devices there are now, each once.
pub fn device_names() -> Vec<String> {
    let host = cpal::default_host();
    let mut names: Vec<String> = Vec::new();
    for name in host
        .output_devices()
        .map(|devices| devices.map(|d| name_of(&d)).collect::<Vec<_>>())
        .unwrap_or_default()
    {
        if !is_plumbing(&name) && !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// The output device whose name is `wanted`, or failing that the only one
/// whose name contains it, ignoring case.
fn find_device(wanted: &str) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    let devices: Vec<cpal::Device> = host
        .output_devices()
        .map_err(|e| format!("cannot list output devices: {e}"))?
        .collect();
    let named: Vec<(String, cpal::Device)> = devices
        .into_iter()
        .map(|d| (name_of(&d), d))
        .filter(|(name, _)| !is_plumbing(name))
        .collect();
    if let Some((_, device)) = named.iter().find(|(name, _)| name == wanted) {
        return Ok(device.clone());
    }
    let lower = wanted.to_lowercase();
    let mut matching = named
        .into_iter()
        .filter(|(name, _)| name.to_lowercase().contains(&lower));
    match (matching.next(), matching.next()) {
        (Some((_, device)), None) => Ok(device),
        (Some(_), Some(_)) => Err(format!("more than one output device matches '{wanted}'")),
        (None, _) => Err(format!("no output device matches '{wanted}'")),
    }
}

/// Opens `device`, or the system default with none, at the configured rate
/// and buffer size. `lost` is set if the device later goes away — unplugged,
/// disconnected, its sound server stopped.
pub fn open(
    device: Option<&str>,
    sample_rate: Option<u32>,
    buffer_size: Option<u32>,
    lost: Arc<AtomicBool>,
) -> Result<Output, String> {
    let (builder, name) = match device {
        Some(wanted) => {
            let device = find_device(wanted)?;
            let name = name_of(&device);
            let builder =
                rodio::DeviceSinkBuilder::from_device(device).map_err(|e| e.to_string())?;
            (builder, name)
        }
        None => {
            let name = cpal::default_host()
                .default_output_device()
                .map(|d| name_of(&d))
                .unwrap_or_else(|| "default".to_string());
            let builder =
                rodio::DeviceSinkBuilder::from_default_device().map_err(|e| e.to_string())?;
            (builder, name)
        }
    };
    let mut builder = builder;
    if let Some(rate) = sample_rate.and_then(rodio::SampleRate::new) {
        builder = builder.with_sample_rate(rate);
    }
    if let Some(frames) = buffer_size {
        builder = builder.with_buffer_size(cpal::BufferSize::Fixed(period_for(frames)));
    }
    let builder = builder.with_error_callback(move |error: cpal::Error| {
        if is_loss(error.kind()) {
            tracing::warn!("Audio output lost: {error}");
            lost.store(true, Ordering::SeqCst);
        } else {
            tracing::debug!("Audio output: {error}");
        }
    });
    let mut sink = builder.open_sink_or_fallback().map_err(|e| e.to_string())?;
    sink.log_on_drop(false);
    tracing::info!(
        "Audio output opened: {name}, {} Hz, {} ch, buffer {:?}",
        sink.config().sample_rate(),
        sink.config().channel_count(),
        sink.config().buffer_size(),
    );
    Ok(Output { sink, name })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_device_that_is_gone_means_reopening() {
        assert!(is_loss(ErrorKind::DeviceNotAvailable));
        assert!(is_loss(ErrorKind::StreamInvalidated));
        assert!(is_loss(ErrorKind::HostUnavailable));
        assert!(!is_loss(ErrorKind::Xrun), "an underrun is a glitch");
        assert!(
            !is_loss(ErrorKind::DeviceChanged),
            "a rerouted default carries on"
        );
    }

    #[test]
    fn the_configured_size_is_the_whole_buffer_not_the_period() {
        assert_eq!(period_for(8192), 4096);
        assert_eq!(period_for(1), 1);
    }

    #[test]
    fn plumbing_is_not_offered() {
        assert!(is_plumbing(
            "Discard all samples (playback) or generate zero samples (capture)"
        ));
        assert!(!is_plumbing("PipeWire Sound Server"));
    }

    #[test]
    fn a_device_that_does_not_exist_is_an_error_not_a_panic() {
        let lost = Arc::new(AtomicBool::new(false));
        let error = open(Some("no such device, surely"), None, None, lost).err();
        assert!(error.is_some());
    }
}
