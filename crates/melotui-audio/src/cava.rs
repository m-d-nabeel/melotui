use ringbuf::{
    HeapRb,
    traits::{Observer, Producer, Split},
};
use std::{
    io::{BufReader, Read},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

pub type CavaConsumer = <HeapRb<f32> as Split>::Cons;

pub struct Cava {
    process: Child,
    running: Arc<AtomicBool>,
}

impl Cava {
    pub fn is_available() -> bool {
        melotui_core::state::visualizer::is_cava_available()
    }

    pub fn new() -> anyhow::Result<(Self, CavaConsumer)> {
        let num_bars = 200;
        let config_path =
            write_cava_config(&melotui_core::config::AppConfig::data_dir(), num_bars)?;

        let mut process = Command::new("cava")
            .arg("-p")
            .arg(&config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()?;

        let stdout = process.stdout.take().ok_or(anyhow::anyhow!("No stdout"))?;

        // capacity for ~5 frames
        let ring = HeapRb::<f32>::new(1024);
        let (mut producer, consumer) = ring.split();

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        // Spawn CAVA stdout reader thread
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            // cava: 1 byte per bar
            let mut buffer = vec![0u8; num_bars];
            // mapping buffer
            let mut float_buffer = vec![0.0f32; num_bars];

            while running_clone.load(Ordering::Relaxed) {
                if reader.read_exact(&mut buffer).is_ok() {
                    // check room for frame
                    if producer.vacant_len() >= num_bars {
                        // normalize 0-255 to 0-1
                        for (i, &b) in buffer.iter().enumerate() {
                            float_buffer[i] = b as f32 / 255.0;
                        }

                        // push frame
                        let _ = producer.push_slice(&float_buffer);
                    } else {
                        // drop frame for backpressure
                        // keeping consumer fast is key for SPSC
                    }
                } else {
                    break;
                }
            }
        });

        Ok((Self { process, running }, consumer))
    }
}

impl Drop for Cava {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Writes cava's configuration into `dir`, melotui's own folder.
///
/// It used to go to a fixed name in the system's temporary folder, which
/// everyone on the machine can write to: another user's file there, or a link
/// under that name to somewhere else, was written over or written through.
fn write_cava_config(dir: &std::path::Path, bars: usize) -> anyhow::Result<std::path::PathBuf> {
    let config_path = dir.join("cava.conf");

    // cava config
    let config = format!(
        r#"
[general]
bars = {bars}
framerate = 60
sleep_timer = 1

[output]
method = raw
raw_target = /dev/stdout
data_format = binary
bit_format = 8bit

[smoothing]
monstercat = 1
noise_reduction = 0.77
"#,
        bars = bars
    );

    melotui_core::utils::fs::write_atomically(&config_path, config.as_bytes())?;
    Ok(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_configuration_is_written_where_it_is_told_to_be() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_cava_config(&dir.path().join("melotui"), 200).unwrap();
        assert!(path.starts_with(dir.path()));
        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.contains("bars = 200"), "{text}");
        assert!(text.contains("method = raw"));
    }

    #[test]
    #[cfg(unix)]
    fn a_link_left_under_its_name_is_replaced_not_written_through() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = dir.path().join("someone-elses-file");
        std::fs::write(&elsewhere, "untouched").unwrap();
        std::os::unix::fs::symlink(&elsewhere, dir.path().join("cava.conf")).unwrap();

        write_cava_config(dir.path(), 200).unwrap();

        assert_eq!(std::fs::read_to_string(&elsewhere).unwrap(), "untouched");
    }
}
