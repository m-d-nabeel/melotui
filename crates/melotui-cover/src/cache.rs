use image::DynamicImage;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

/// Maximum dimension for cached covers.
/// 600px is sufficient for most terminal display sizes while keeping
/// memory usage reasonable (~1.4 MB RGBA per image).
const MAX_COVER_DIM: u32 = 600;

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
        let result = if img.width() > MAX_COVER_DIM || img.height() > MAX_COVER_DIM {
            let resized = img.resize(
                MAX_COVER_DIM,
                MAX_COVER_DIM,
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

        let mut cache = CoverCache::new();
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
}
