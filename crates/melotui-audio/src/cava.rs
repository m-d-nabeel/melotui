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
        let config_path = create_cava_config(num_bars)?;

        let mut process = Command::new("cava")
            .arg("-p")
            .arg(config_path.to_str().unwrap())
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
                        // drop frame (backpressure)
                        // We already read it from stdout, so it's discarded.
                        // This ensures we don't block the reader, but we also don't fill the buffer with old data.
                        // Ideally we might want to discard *older* data in the ring, but for SPSC,
                        // keeping the consumer draining fast is the key.
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

fn create_cava_config(bars: usize) -> anyhow::Result<std::path::PathBuf> {
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("melotui_cava.conf");

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

    std::fs::write(&config_path, config)?;
    Ok(config_path)
}
