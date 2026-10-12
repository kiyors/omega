use omega_session::{SessionCandidate, SessionCollector, SessionSource};
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn test_benchmark_candidate_aggregation_1000() {
    let mut candidates = Vec::with_capacity(1000);
    for i in 0..1000 {
        let path = PathBuf::from(format!("/mock/path/project-{}", i));
        let src = match i % 5 {
            0 => SessionSource::Tmux,
            1 => SessionSource::Alias,
            2 => SessionSource::Config,
            3 => SessionSource::Git,
            _ => SessionSource::Zoxide,
        };
        candidates.push(SessionCandidate::new(format!("proj-{}", i), path, src));
    }

    let collector = SessionCollector::new();
    let start = Instant::now();
    let result = collector.dedup_candidates(candidates, false);
    let duration = start.elapsed();

    assert_eq!(result.len(), 1000);
    assert!(
        duration.as_millis() < 30,
        "Deduplication of 1,000 candidates took {:?}, expected < 30ms",
        duration
    );
}
