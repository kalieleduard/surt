use std::path::{Path, PathBuf};

use super::{parse_branches, parse_worktrees, worktree_path};

const PORCELAIN: &str = "worktree C:/dev/shop
HEAD 87b29f9aa11d0c1c3a1e5b2c3d4e5f6a7b8c9d0e
branch refs/heads/develop

worktree C:/dev/shop-worktrees/870-voice-kit
HEAD 302de82aa11d0c1c3a1e5b2c3d4e5f6a7b8c9d0e
branch refs/heads/chore/voice-test-kit

worktree C:/Users/dev/.surt/worktrees/shop/run-1
HEAD 1111111aa11d0c1c3a1e5b2c3d4e5f6a7b8c9d0e
branch refs/heads/surt/run-1

worktree C:/dev/shop-old
HEAD 2222222aa11d0c1c3a1e5b2c3d4e5f6a7b8c9d0e
detached
prunable gitdir file points to non-existent location

worktree C:/dev/shop-review
HEAD 3333333aa11d0c1c3a1e5b2c3d4e5f6a7b8c9d0e
detached
";

#[test]
fn worktrees_read_with_branch_short_head_and_the_first_as_main() {
    let worktrees = parse_worktrees(PORCELAIN, Some(Path::new("C:/Users/dev/.surt/worktrees")));

    let summary: Vec<_> = worktrees
        .iter()
        .map(|worktree| {
            (
                worktree.name.as_str(),
                worktree.branch.as_deref(),
                worktree.head.as_str(),
                worktree.main,
            )
        })
        .collect();
    assert_eq!(
        summary,
        vec![
            ("shop", Some("develop"), "87b29f9", true),
            (
                "870-voice-kit",
                Some("chore/voice-test-kit"),
                "302de82",
                false
            ),
            ("shop-review", None, "3333333", false),
        ]
    );
}

#[test]
fn branches_name_the_shown_worktree_they_are_checked_out_in() {
    let refs = "develop\u{1f}C:/dev/shop\nchore/voice-test-kit\u{1f}C:/dev/shop-worktrees/870-voice-kit\nsurt/run-1\u{1f}C:/Users/dev/.surt/worktrees/shop/run-1\nmain\u{1f}\n";

    let branches = parse_branches(
        refs,
        &["C:/dev/shop", "C:/dev/shop-worktrees/870-voice-kit"],
    );

    let summary: Vec<_> = branches
        .iter()
        .map(|branch| (branch.name.as_str(), branch.worktree.as_deref()))
        .collect();
    assert_eq!(
        summary,
        vec![
            ("develop", Some("C:/dev/shop")),
            (
                "chore/voice-test-kit",
                Some("C:/dev/shop-worktrees/870-voice-kit")
            ),
            ("surt/run-1", None),
            ("main", None),
        ]
    );
}

#[test]
fn a_new_worktree_lives_beside_the_repository_named_after_the_branch() {
    let root = PathBuf::from("C:/dev/shop");

    assert_eq!(
        worktree_path(&root, "feat/904 spanish run"),
        Some(
            PathBuf::from("C:/dev")
                .join("shop-worktrees")
                .join("904-spanish-run")
        )
    );
    assert_eq!(worktree_path(&root, "feat/..."), None);
}
