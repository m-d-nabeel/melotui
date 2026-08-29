//! Behaviour under test: a track's play history is durable and correct when
//! the app is closed and reopened — not an implementation detail of how
//! `Library` happens to represent "most played" or "recently played" in
//! memory, which is exactly what made this easy to get wrong before: the two
//! were computed once at load and never touched again as plays came in
//! in-process. This test only cares about what a real restart would produce,
//! driven through the actual worker entry point (`melotui_db::run`) over the
//! same command/event channels the engine uses in production — not by
//! calling an internal loader function directly.

use melotui_bus::{channel::inprocess, command::DatabaseCommand, event::IpcEvent};
use melotui_core::domain::TrackId;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Owns the temp directory so it's cleaned up on drop; the caller only needs
/// the path.
fn temp_db_path() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("library.db");
    (dir, path)
}

/// Seeds a fresh database with two tracks, the way a real scan would have
/// left it — via the crate's real migration runner, not a hand-copied schema.
fn seed_two_tracks(db_path: &std::path::Path) {
    let conn = rusqlite::Connection::open(db_path).expect("open db");
    melotui_db::run_migrations(&conn).expect("run migrations");
    conn.execute(
        "INSERT INTO tracks (id, file_path, title, duration_ms, format, file_size_bytes, file_modified_at)
         VALUES (1, '/music/a.flac', 'Song A', 200000, 'flac', 1000, 0)",
        [],
    )
    .expect("seed track 1");
    conn.execute(
        "INSERT INTO tracks (id, file_path, title, duration_ms, format, file_size_bytes, file_modified_at)
         VALUES (2, '/music/b.flac', 'Song B', 200000, 'flac', 1000, 0)",
        [],
    )
    .expect("seed track 2");
}

fn spawn_worker(
    db_path: PathBuf,
) -> (
    std::sync::mpsc::Sender<DatabaseCommand>,
    std::sync::mpsc::Receiver<IpcEvent>,
) {
    let (bus_rx, bus_tx) = inprocess::hub();
    let (cmd_tx, worker) = inprocess::join_worker::<DatabaseCommand>(bus_tx);
    thread::spawn(move || melotui_db::run(worker, db_path));
    (cmd_tx, bus_rx)
}

fn expect_library_loaded(
    rx: &std::sync::mpsc::Receiver<IpcEvent>,
) -> melotui_core::library::Library {
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(IpcEvent::LibraryLoaded { snapshot }) => *snapshot,
        Ok(other) => panic!("expected LibraryLoaded, got {other:?}"),
        Err(e) => panic!("no LibraryLoaded event arrived: {e}"),
    }
}

#[test]
fn play_counts_and_recency_are_on_disk_and_reload_correctly() {
    let (_tmp, db_path) = temp_db_path();
    seed_two_tracks(&db_path);
    let (cmd_tx, evt_rx) = spawn_worker(db_path);

    // A listening session: track 1 gets played three times, track 2 once.
    for _ in 0..3 {
        cmd_tx
            .send(DatabaseCommand::IncrementPlayCount { track_id: 1 })
            .unwrap();
    }
    cmd_tx
        .send(DatabaseCommand::IncrementPlayCount { track_id: 2 })
        .unwrap();

    // Simulate the app being closed and reopened: nothing but a fresh
    // LoadLibrary against the same file. If play tracking were still
    // in-memory-only, or if the reload path didn't actually persist it,
    // this would come back at zero.
    cmd_tx
        .send(DatabaseCommand::LoadLibrary { roots: vec![] })
        .unwrap();
    let snapshot = expect_library_loaded(&evt_rx);

    let track1 = snapshot.tracks.get(&TrackId(1)).expect("track 1 present");
    let track2 = snapshot.tracks.get(&TrackId(2)).expect("track 2 present");
    assert_eq!(track1.play_count, 3, "three plays must survive a reload");
    assert_eq!(track2.play_count, 1);
    assert!(
        track1.last_played_at.is_some() && track2.last_played_at.is_some(),
        "a played track must carry a last-played timestamp after reload"
    );

    // Most-played ordering is unambiguous (3 plays vs 1) regardless of
    // timing, so this can be asserted precisely.
    let most_played = snapshot.most_played();
    assert_eq!(
        most_played.first(),
        Some(&TrackId(1)),
        "the track played three times must rank above the one played once"
    );

    // Recency, unlike play count, only has one-second resolution in SQLite's
    // `strftime('%s','now')`, and these two increments can legitimately land
    // in the same second. So: assert both are present (the actual behaviour
    // under test — a play is recorded and comes back at all) without
    // asserting which of two near-simultaneous plays sorts first, which
    // would make the test flaky for a reason that has nothing to do with the
    // code being tested.
    let recently_played = snapshot.recently_played();
    assert!(recently_played.contains(&TrackId(1)));
    assert!(recently_played.contains(&TrackId(2)));

    let _ = cmd_tx.send(DatabaseCommand::Shutdown);
}

#[test]
fn a_track_with_no_plays_never_appears_in_either_list() {
    let (_tmp, db_path) = temp_db_path();
    seed_two_tracks(&db_path);
    let (cmd_tx, evt_rx) = spawn_worker(db_path);

    cmd_tx
        .send(DatabaseCommand::IncrementPlayCount { track_id: 1 })
        .unwrap();
    cmd_tx
        .send(DatabaseCommand::LoadLibrary { roots: vec![] })
        .unwrap();
    let snapshot = expect_library_loaded(&evt_rx);

    assert!(!snapshot.most_played().contains(&TrackId(2)));
    assert!(!snapshot.recently_played().contains(&TrackId(2)));

    let _ = cmd_tx.send(DatabaseCommand::Shutdown);
}
