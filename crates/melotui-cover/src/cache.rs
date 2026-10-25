use image::DynamicImage;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

/// The most the cover cache may hold on disk. Past it, the covers used
/// longest ago are deleted — each track's cover is cached on its own, and a
/// track re-tagged or moved leaves its old one behind, so without a limit the
/// cache only ever grew.
const MAX_CACHE_BYTES: u64 = 256 * 1024 * 1024;
/// What the cache is trimmed down to once over the limit, so it is not
/// trimmed again at the next start.
const TRIM_TO_BYTES: u64 = MAX_CACHE_BYTES / 10 * 8;

pub struct CoverCache {
    cache_dir: PathBuf,
    max_cover_dim: u32,
}

impl Default for CoverCache {
    fn default() -> Self {
        Self::new(600)
    }
}

impl CoverCache {
    pub fn new(max_cover_dim: u32) -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("melotui")
            .join("covers");

        if let Err(e) = fs::create_dir_all(&cache_dir) {
            tracing::error!("Failed to create cache dir: {}", e);
        }

        // Trimmed on a thread of its own: walking a large cache takes a moment,
        // and the first covers should not wait for it.
        let dir = cache_dir.clone();
        let _ = std::thread::Builder::new()
            .name("cover-cache-trim".into())
            .spawn(move || {
                let removed = trim(&dir, MAX_CACHE_BYTES, TRIM_TO_BYTES);
                if removed > 0 {
                    tracing::info!("Trimmed {removed} old covers from the cache");
                }
            });

        Self {
            cache_dir,
            max_cover_dim,
        }
    }

    /// Retrieve cover from cache, or extract from source if missing.
    pub fn get_or_load(&self, track_path: &Path) -> Option<Arc<DynamicImage>> {
        let key = self.compute_cache_key(track_path)?;
        let cache_path = self.get_cache_path(&key);

        if cache_path.exists() {
            if let Ok(img) = image::open(&cache_path) {
                // Marked as just used, so trimming takes it last.
                if let Ok(file) = fs::File::options().write(true).open(&cache_path) {
                    let _ = file.set_modified(SystemTime::now());
                }
                return Some(Arc::new(img));
            }
            let _ = fs::remove_file(&cache_path);
        }

        // Extract and process — carefully managing memory
        let resized = self.extract_and_resize(track_path)?;

        // best-effort cache write
        if let Err(e) = resized.save_with_format(&cache_path, image::ImageFormat::Jpeg) {
            tracing::warn!("Failed to write cache file {:?}: {}", cache_path, e);
        }

        // After heavy allocations, ask glibc to return freed pages to the OS.
        // This prevents the "arena" effect where RSS stays at peak watermark.
        release_memory_to_os();

        Some(Arc::new(resized))
    }

    /// Extract cover art and resize it, carefully staging allocations
    /// to minimize peak memory usage.
    fn extract_and_resize(&self, path: &Path) -> Option<DynamicImage> {
        // Stage 1: Extract JPEG bytes and immediately drop the tagged file.
        // This frees the tag/audio buffer before we allocate the decoded image.
        let picture_data = {
            use lofty::file::TaggedFileExt;

            let tagged_file = lofty::probe::Probe::open(path).ok()?.read().ok()?;
            let tag = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag())?;

            let pic = tag.pictures().first()?;
            pic.data().to_vec() // Copy bytes — allows dropping tagged_file
        };
        // tagged_file is now DROPPED — its memory can be reused for the decode

        // Stage 2: Check dimensions before full decode (no allocation)
        let dimensions = image_dimensions_from_bytes(&picture_data);
        let (src_w, src_h) = dimensions.unwrap_or((0, 0));

        tracing::debug!(
            "Cover art for {:?}: {}×{} ({} KB compressed)",
            path.file_name().unwrap_or_default(),
            src_w,
            src_h,
            picture_data.len() / 1024
        );

        // Stage 3: Decode the image
        let img = image::load_from_memory(&picture_data).ok()?;
        drop(picture_data); // Free compressed bytes before resize

        // Stage 4: Resize if needed — then immediately drop the original
        let result = if img.width() > self.max_cover_dim || img.height() > self.max_cover_dim {
            let resized = img.resize(
                self.max_cover_dim,
                self.max_cover_dim,
                image::imageops::FilterType::Triangle,
            );
            drop(img); // Explicitly drop the large decoded image ASAP
            resized
        } else {
            img
        };

        Some(result)
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

    /// Return the on-disk cache path for a track's cover art, if the key can be computed.
    /// Does NOT load or extract the image — just resolves the path.
    pub fn resolve_cache_path(&self, track_path: &Path) -> Option<PathBuf> {
        let key = self.compute_cache_key(track_path)?;
        let path = self.get_cache_path(&key);
        if path.exists() { Some(path) } else { None }
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
}

/// Read image dimensions from compressed bytes without full decode.
/// Returns (width, height) or None if the format can't be determined.
/// When the cache in `dir` holds more than `limit` bytes, deletes the covers
/// used longest ago until it holds no more than `target`. Returns how many it
/// deleted.
fn trim(dir: &Path, limit: u64, target: u64) -> usize {
    let mut covers: Vec<(SystemTime, u64, PathBuf)> = Vec::new();
    for shard in fs::read_dir(dir).into_iter().flatten().flatten() {
        for entry in fs::read_dir(shard.path()).into_iter().flatten().flatten() {
            if let Ok(metadata) = entry.metadata()
                && metadata.is_file()
            {
                let used = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                covers.push((used, metadata.len(), entry.path()));
            }
        }
    }
    let mut total: u64 = covers.iter().map(|(_, size, _)| size).sum();
    if total <= limit {
        return 0;
    }
    covers.sort_by_key(|(used, _, _)| *used);
    let mut removed = 0;
    for (_, size, path) in covers {
        if total <= target {
            break;
        }
        if fs::remove_file(&path).is_ok() {
            total -= size;
            removed += 1;
        }
    }
    removed
}

fn image_dimensions_from_bytes(data: &[u8]) -> Option<(u32, u32)> {
    let reader = image::ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .ok()?;
    let (w, h) = reader.into_dimensions().ok()?;
    Some((w, h))
}

/// Ask the OS to reclaim freed memory pages.
///
/// glibc's malloc retains freed pages in its arena for future allocations,
/// which keeps RSS at the peak watermark. `malloc_trim(0)` tells it to
/// return those pages to the OS, reducing RSS after large temporary
/// allocations (like decoding a 3000×3000 cover art image).
fn release_memory_to_os() {
    #[cfg(target_os = "linux")]
    {
        // malloc_trim is a standard glibc function that returns freed pages to the OS.
        // It only affects the malloc arena's free list, not any active allocations.
        unsafe extern "C" {
            safe fn malloc_trim(pad: usize) -> i32;
        }
        let released = malloc_trim(0);
        if released != 0 {
            tracing::debug!("malloc_trim released memory to OS");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_compute_cache_key_and_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_track.mp3");

        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"fake mp3 data").unwrap();

        let mut cache = CoverCache::new(600);
        // Override cache dir for testing
        cache.cache_dir = dir.path().join("covers");

        let key = cache.compute_cache_key(&file_path);
        assert!(key.is_some());

        let key_str = key.unwrap();
        assert_eq!(key_str.len(), 64); // SHA-256 hex string

        let cache_path = cache.get_cache_path(&key_str);

        // Assert sharding logic (first two chars)
        let expected_shard = &key_str[0..2];
        let parent = cache_path.parent().unwrap();
        assert_eq!(parent.file_name().unwrap(), expected_shard);
        assert_eq!(
            cache_path.file_name().unwrap().to_str().unwrap(),
            format!("{}.jpg", key_str)
        );
    }

    #[test]
    fn a_cache_over_its_limit_loses_the_covers_used_longest_ago() {
        let dir = tempdir().unwrap();
        let shard = dir.path().join("ab");
        fs::create_dir(&shard).unwrap();
        let base = SystemTime::now() - std::time::Duration::from_secs(1000);
        for i in 0..10u64 {
            let path = shard.join(format!("{i}.jpg"));
            fs::write(&path, vec![0u8; 100]).unwrap();
            let file = File::options().write(true).open(&path).unwrap();
            file.set_modified(base + std::time::Duration::from_secs(i * 10))
                .unwrap();
        }
        // 1000 bytes against a limit of 600: down to 400, oldest first.
        assert_eq!(trim(dir.path(), 600, 400), 6);
        let mut left: Vec<String> = fs::read_dir(&shard)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, ["6.jpg", "7.jpg", "8.jpg", "9.jpg"]);

        // Under the limit, nothing goes.
        assert_eq!(trim(dir.path(), 600, 400), 0);
    }
}
