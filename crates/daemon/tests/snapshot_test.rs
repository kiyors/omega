use omega_daemon::{
    AUTO_SAVE_PREFIX, LATEST_POINTER, PaneSnapshot, SessionSnapshotRecord, Snapshot, SnapshotStore,
    WindowSnapshot, break_down_timestamp,
};
use std::path::PathBuf;
use tempfile::tempdir;

fn sample_snapshot(created_at: &str) -> Snapshot {
    Snapshot {
        version: 1,
        created_at: created_at.to_string(),
        sessions: vec![SessionSnapshotRecord {
            name: "omega".to_string(),
            path: PathBuf::from("/Users/dev/omega"),
            windows: vec![WindowSnapshot {
                index: 1,
                name: "editor".to_string(),
                layout: "bb62,159x48,0,0,1".to_string(),
                active: true,
                panes: vec![
                    PaneSnapshot {
                        index: 1,
                        path: PathBuf::from("/Users/dev/omega"),
                        command: "zsh".to_string(),
                        active: true,
                    },
                    PaneSnapshot {
                        index: 2,
                        path: PathBuf::from("/Users/dev/omega/crates/session"),
                        command: "cargo".to_string(),
                        active: false,
                    },
                ],
            }],
        }],
    }
}

#[test]
fn test_snapshot_model_serialization_roundtrip() {
    let snapshot = sample_snapshot("2026-10-13T07:30:00Z");

    let serialized = serde_json::to_string_pretty(&snapshot).expect("serialize snapshot");
    let deserialized: Snapshot = serde_json::from_str(&serialized).expect("deserialize snapshot");
    assert_eq!(snapshot, deserialized);
}

#[test]
fn test_store_save_named_and_load() {
    let tmp = tempdir().expect("tempdir");
    let store = SnapshotStore::new(tmp.path().to_path_buf());
    let snapshot = sample_snapshot("2026-10-13T07:30:00Z");

    let saved_path = store
        .save_snapshot(&snapshot, Some("my-backup"))
        .expect("save named");
    assert!(saved_path.ends_with("my-backup.json"));
    assert!(saved_path.exists());

    let loaded = store.load_snapshot(Some("my-backup")).expect("load named");
    assert_eq!(loaded, snapshot);
}

#[test]
fn test_store_auto_save_creates_latest_pointer() {
    let tmp = tempdir().expect("tempdir");
    let store = SnapshotStore::new(tmp.path().to_path_buf());
    let snapshot = sample_snapshot("2026-10-13T07:30:00Z");

    let saved_path = store.save_snapshot(&snapshot, None).expect("save auto");
    assert!(saved_path.exists());

    let latest_path = tmp.path().join(LATEST_POINTER);
    assert!(latest_path.exists(), "latest.json should be created");

    let loaded = store.load_snapshot(None).expect("load latest");
    assert_eq!(loaded, snapshot);
}

#[test]
fn test_latest_pointer_resolves_to_newest_auto_save() {
    let tmp = tempdir().expect("tempdir");
    let store = SnapshotStore::new(tmp.path().to_path_buf());

    store
        .save_snapshot(&sample_snapshot("first"), None)
        .expect("first save");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let second = store
        .save_snapshot(&sample_snapshot("second"), None)
        .expect("second save");

    let latest = tmp.path().join(LATEST_POINTER);
    #[cfg(unix)]
    {
        let resolved = std::fs::read_link(&latest).unwrap_or_else(|_| latest.clone());
        assert_eq!(
            resolved.file_name(),
            second.file_name(),
            "latest.json must atomically point at the newest auto-save"
        );
    }
    #[cfg(not(unix))]
    {
        assert!(latest.exists(), "latest.json must exist");
        let loaded = store.load_snapshot(None).expect("load latest");
        assert_eq!(
            loaded,
            sample_snapshot("second"),
            "latest.json must resolve to the newest auto-save"
        );
    }
}

#[test]
fn test_store_rotation_keeps_last_ten_and_exempts_named() {
    let tmp = tempdir().expect("tempdir");
    let store = SnapshotStore::new(tmp.path().to_path_buf());

    let named = Snapshot {
        version: 1,
        created_at: "named".to_string(),
        sessions: vec![],
    };
    store
        .save_snapshot(&named, Some("preserved-tag"))
        .expect("save named");

    // 12 rapid auto-saves; same-second collisions get numeric suffixes.
    for i in 1..=12 {
        let snap = Snapshot {
            version: 1,
            created_at: format!("2026-10-13T07:{i:02}:00Z"),
            sessions: vec![],
        };
        store.save_snapshot(&snap, None).expect("save auto");
    }

    let entries: Vec<String> = std::fs::read_dir(tmp.path())
        .expect("read dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    let auto_saves: Vec<&String> = entries
        .iter()
        .filter(|name| name.starts_with(AUTO_SAVE_PREFIX) && name.ends_with(".json"))
        .collect();

    assert_eq!(
        auto_saves.len(),
        10,
        "rolling auto-saves must retain exactly 10 snapshots"
    );
    assert!(
        entries.contains(&"preserved-tag.json".to_string()),
        "named snapshots must be preserved indefinitely"
    );
    assert!(
        entries.contains(&LATEST_POINTER.to_string()),
        "latest.json pointer must exist"
    );
}

#[test]
fn test_store_list_snapshots_reports_kinds() {
    let tmp = tempdir().expect("tempdir");
    let store = SnapshotStore::new(tmp.path().to_path_buf());

    store
        .save_snapshot(&sample_snapshot("auto"), None)
        .expect("save auto");
    store
        .save_snapshot(&sample_snapshot("named"), Some("tagged"))
        .expect("save named");

    let summaries = store.list_snapshots().expect("list");
    assert_eq!(summaries.len(), 2);
    let auto = summaries
        .iter()
        .find(|s| s.is_auto)
        .expect("auto summary present");
    let named = summaries
        .iter()
        .find(|s| !s.is_auto)
        .expect("named summary present");
    assert!(auto.name.starts_with(AUTO_SAVE_PREFIX));
    assert_eq!(named.name, "tagged.json");
}

#[test]
fn test_load_missing_snapshot_reports_not_found() {
    let tmp = tempdir().expect("tempdir");
    let store = SnapshotStore::new(tmp.path().to_path_buf());

    let err = store
        .load_snapshot(Some("nope"))
        .expect_err("missing snapshot must error");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);

    let err = store
        .load_snapshot(None)
        .expect_err("missing latest must error");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn test_break_down_timestamp_known_epochs() {
    assert_eq!(break_down_timestamp(0), (1970, 1, 1, 0, 0, 0), "epoch");
    assert_eq!(
        break_down_timestamp(951_782_400),
        (2000, 2, 29, 0, 0, 0),
        "leap day"
    );
    assert_eq!(
        break_down_timestamp(1_735_689_600),
        (2025, 1, 1, 0, 0, 0),
        "new year"
    );
    assert_eq!(
        break_down_timestamp(1_700_000_000),
        (2023, 11, 14, 22, 13, 20),
        "arbitrary instant"
    );
}

#[test]
fn test_auto_save_filename_follows_spec_format() {
    let tmp = tempdir().expect("tempdir");
    let store = SnapshotStore::new(tmp.path().to_path_buf());

    let saved = store
        .save_snapshot(&sample_snapshot("2026-10-13T07:30:00Z"), None)
        .expect("save auto");
    let name = saved
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned();

    // snapshot-YYYY-MM-DD-HHMMSS.json
    let stamp = name
        .strip_prefix(AUTO_SAVE_PREFIX)
        .and_then(|s| s.strip_suffix(".json"))
        .expect("timestamp segment");
    assert_eq!(stamp.len(), 17, "expected YYYY-MM-DD-HHMMSS, got {stamp}");
    assert_eq!(stamp.chars().nth(4), Some('-'));
    assert_eq!(stamp.chars().nth(7), Some('-'));
    assert_eq!(stamp.chars().nth(10), Some('-'));
}

#[test]
fn test_tagged_snapshot_sanitizes_path_traversal() {
    let tmp = tempdir().expect("tempdir");
    let store = SnapshotStore::new(tmp.path().to_path_buf());
    let snapshot = sample_snapshot("2026-10-13T07:30:00Z");

    let saved = store
        .save_snapshot(&snapshot, Some("../../traversal-attack"))
        .expect("save sanitized");
    assert_eq!(saved.file_name().unwrap(), "traversal-attack.json");
    assert_eq!(saved.parent().unwrap(), tmp.path());
}
