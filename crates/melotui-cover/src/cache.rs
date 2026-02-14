use image::DynamicImage;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

pub struct CoverCache {
    cache_dir: PathBuf,
}

impl Default for CoverCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CoverCache {
    pub fn new() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("melotui")
            .join("covers");

        if let Err(e) = fs::create_dir_all(&cache_dir) {
            tracing::error!("Failed to create cache dir: {}", e);
        }

        Self { cache_dir }
    }

    /// Retrieve cover from cache, or extract from source if missing.
    pub fn get_or_load(&self, track_path: &Path) -> Option<Arc<DynamicImage>> {
        let key = self.compute_cache_key(track_path)?;
        let cache_path = self.get_cache_path(&key);

        if cache_path.exists() {
            if let Ok(img) = image::open(&cache_path) {
                return Some(Arc::new(img));
            }
            let _ = fs::remove_file(&cache_path);
        }

        if let Some(img) = self.load_from_source(track_path) {
            let resized = img.resize(360, 360, image::imageops::FilterType::Triangle);

            // best-effort cache write
            if let Err(e) = resized.save_with_format(&cache_path, image::ImageFormat::Jpeg) {
                tracing::warn!("Failed to write cache file {:?}: {}", cache_path, e);
            }

            return Some(Arc::new(resized));
        }

        None
    }

    fn compute_cache_key(&self, path: &Path) -> Option<String> {
        let metadata = fs::metadata(path).ok()?;
        let size = metadata.len();
        let mtime = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut hasher = Sha256::new();
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update(size.to_le_bytes());
        hasher.update(mtime.to_le_bytes());

        Some(hex::encode(hasher.finalize()))
    }

    fn get_cache_path(&self, key: &str) -> PathBuf {
        // Sharding: covers/ab/ab1234...
        let shard = &key[0..2];
        let shard_dir = self.cache_dir.join(shard);

        if !shard_dir.exists() {
            let _ = fs::create_dir_all(&shard_dir);
        }

        shard_dir.join(format!("{}.jpg", key))
    }

    fn load_from_source(&self, path: &Path) -> Option<DynamicImage> {
        use lofty::{file::TaggedFileExt, probe::Probe};

        let tagged_file = Probe::open(path).ok()?.read().ok()?;
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())?;
        let picture = tag.pictures().first()?;

        image::load_from_memory(picture.data()).ok()
    }
}
