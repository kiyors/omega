use omega_session::naming::NamingPipeline;
use std::path::Path;

#[test]
fn test_default_segment_naming() {
    let namer = NamingPipeline;
    let path = Path::new("/Users/gaurav/personal/projects/omega");
    let name = namer.resolve_name(path, 1, &[]);
    assert_eq!(name, "omega");

    let name_2_segments = namer.resolve_name(path, 2, &[]);
    assert_eq!(name_2_segments, "projects-omega");
}

#[test]
fn test_tmux_character_sanitization() {
    let namer = NamingPipeline;
    let path = Path::new("/Users/gaurav/.dotfiles");
    let name = namer.resolve_name(path, 1, &[]);
    assert_eq!(
        name, "_dotfiles",
        "Periods should be sanitized to underscores"
    );

    let path_with_colons = Path::new("/Users/gaurav/my:project");
    let name_colons = namer.resolve_name(path_with_colons, 1, &[]);
    assert_eq!(
        name_colons, "my_project",
        "Colons should be sanitized to underscores"
    );
}

#[test]
fn test_collision_resolution() {
    let namer = NamingPipeline;
    let path = Path::new("/Users/gaurav/work/omega");

    // Existing session "omega" points to a different path
    let existing = vec![("omega", Path::new("/Users/gaurav/personal/omega"))];

    let name = namer.resolve_name(path, 1, &existing);
    assert_eq!(
        name, "omega-2",
        "Should append -2 when session name is taken by different path"
    );

    // Existing sessions "omega" and "omega-2" both point to different paths
    let existing_2 = vec![
        ("omega", Path::new("/Users/gaurav/personal/omega")),
        ("omega-2", Path::new("/Users/gaurav/sandbox/omega")),
    ];

    let name_3 = namer.resolve_name(path, 1, &existing_2);
    assert_eq!(name_3, "omega-3", "Should append -3 when -2 is also taken");

    // If existing session points to the EXACT same path, no collision suffix needed
    let same_path = Path::new("/Users/gaurav/work/omega");
    let existing_same = vec![("omega", same_path)];
    let name_same = namer.resolve_name(path, 1, &existing_same);
    assert_eq!(name_same, "omega");
}
