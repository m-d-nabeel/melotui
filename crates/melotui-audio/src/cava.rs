use std::{
    io::{BufReader, Read},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
};

pub struct Cava {
    process: Child,
    receiver: Receiver<Vec<f32>>,
    running: Arc<AtomicBool>,
}

impl Cava {
    pub fn is_available() -> bool {
        Command::new("cava").arg("--version").output().is_ok()
    }

    pub fn new() -> anyhow::Result<Self> {
        let num_bars = 200; // Fixed resolution, UI will scale it
        let config_path = create_cava_config(num_bars)?;

        let mut process = Command::new("cava")
            .arg("-p")
            .arg(config_path.to_str().unwrap())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()?;

        let stdout = process.stdout.take().ok_or(anyhow::anyhow!("No stdout"))?;
        let (tx, rx) = mpsc::channel();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        // Spawn a reader thread specifically for CAVA stdout
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            // CAVA binary format: 1 byte per bar (8-bit)
            let mut buffer = vec![0u8; num_bars];

            while running_clone.load(Ordering::Relaxed) {
                if reader.read_exact(&mut buffer).is_ok() {
                    let floats: Vec<f32> = buffer.iter().map(|&b| b as f32 / 255.0).collect();
                    if tx.send(floats).is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
        });

        Ok(Self {
            process,
            receiver: rx,
            running,
        })
    }

    pub fn next_spectrum(&self) -> Option<Vec<f32>> {
        // Drain channel to get the very latest frame
        let mut last_frame = None;
        loop {
            match self.receiver.try_recv() {
                Ok(frame) => last_frame = Some(frame),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return None,
            }
        }
        last_frame
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

    // Simple config: standard bars, raw binary output
    let config = format!(
        r#"
[general]
bars = {bars}
framerate = 60

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
