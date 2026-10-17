use super::{GitVersion, MIN_VERSION};

fn version(major: u32, minor: u32, patch: u32) -> GitVersion {
    GitVersion {
        major,
        minor,
        patch,
    }
}

#[test]
fn parses_the_version_line_of_every_platform_build() {
    let cases = [
        ("git version 2.47.0\n", version(2, 47, 0)),
        ("git version 2.39.3 (Apple Git-146)\n", version(2, 39, 3)),
        ("git version 2.45.1.windows.1\n", version(2, 45, 1)),
        ("git version 2.38\n", version(2, 38, 0)),
    ];

    for (output, expected) in cases {
        assert_eq!(GitVersion::parse(output), Some(expected), "{output}");
    }
}

#[test]
fn rejects_output_that_is_not_a_version_line() {
    for output in ["", "git version", "git version x.y", "hub version 2.40.0"] {
        assert_eq!(GitVersion::parse(output), None, "{output}");
    }
}

#[test]
fn versions_before_2_38_are_below_the_minimum() {
    assert!(version(2, 37, 9) < MIN_VERSION);
    assert!(version(1, 99, 0) < MIN_VERSION);
    assert!(version(2, 38, 0) >= MIN_VERSION);
    assert!(version(3, 0, 0) >= MIN_VERSION);
}

#[test]
fn displays_as_major_minor_patch() {
    assert_eq!(version(2, 34, 1).to_string(), "2.34.1");
}
