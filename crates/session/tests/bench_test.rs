use omega_session::{SessionCandidate, SessionCollector, SessionSource};
use std::fs;
use std::time::Instant;
use tempfile::tempdir;

#[test]
fn test_benchmark_candidate_aggregation_1000() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // Create 1,000 real directories on disk to benchmark genuine filesystem canonicalization
    let mut candidates = Vec::with_capacity(1000);
    for i in 0..1000 {
        let dir = root.join(format!("project-{:04}", i));
        fs::create_dir(&dir).unwrap();

        let src = match i % 5 {
            0 => SessionSource::Tmux,
            1 => SessionSource::Alias,
            2 => SessionSource::Config,
            3 => SessionSource::Git,
            _ => SessionSource::Zoxide,
        };
        candidates.push(SessionCandidate::new(format!("proj-{}", i), dir, src));
    }

    let collector = SessionCollector::new();
    let start = Instant::now();
    let result = collector.dedup_candidates(candidates, false);
    let duration = start.elapsed();

    assert_eq!(result.len(), 1000);
    assert!(
        duration.as_millis() < 30,
        "Canonical deduplication of 1,000 real filesystem directories took {:?}, expected < 30ms",
        duration
    );
}
