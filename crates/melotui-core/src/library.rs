use std::fs;
use std::path::PathBuf;

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub path: PathBuf,
    pub tracks: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Library {
    pub root: PathBuf,
    pub playlists: Vec<Playlist>,
}

impl Library {
    pub fn scan(root: PathBuf) -> Result<Self> {
        if !root.exists() {
            fs::create_dir_all(&root)?;
        }

        let mut playlists = Vec::new();
        let mut untitled_tracks = Vec::new();

        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                // Files directly under root → Untitled
                untitled_tracks.push(path);
                continue;
            }

            if !path.is_dir() {
                continue;
            }

            let name = match path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => continue,
            };

            let mut tracks = Vec::new();

            for file in fs::read_dir(&path)? {
                let file = file?;
                let file_path = file.path();

                if file_path.is_file() {
                    tracks.push(file_path);
                }
            }

            playlists.push(Playlist { name, path, tracks });
        }

        // Add Untitled playlist if needed
        if !untitled_tracks.is_empty() {
            playlists.insert(
                0,
                Playlist {
                    name: "Untitled".into(),
                    path: root.clone(),
                    tracks: untitled_tracks,
                },
            );
        }

        Ok(Self { root, playlists })
    }
}
