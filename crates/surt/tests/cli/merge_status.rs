use serde_json::{Value, json};

use crate::merge::{base_branch, commit_news, merging_project, planned, sha, short, surt};

#[test]
fn status_shows_the_fast_forward_of_a_merged_run() {
    let sandbox = merging_project();
    let name = planned(&sandbox);
    let main = base_branch(&sandbox);
    let base = sha(&sandbox, &main);
    sandbox.git(&sandbox.repo(), &["checkout", "--quiet", "--detach"]);
    surt(&sandbox, &["continue", &name]);
    let tip = sha(&sandbox, &format!("surt/{name}"));

    let detail = surt(&sandbox, &["status", &name]).stdout;
    let json = surt(&sandbox, &["status", &name, "--json"]).stdout;

    assert!(
        detail.contains(&format!(
            "\n  merge    merged into {main} (fast-forward {} → {})\n",
            short(&base),
            short(&tip)
        )),
        "{detail}"
    );
    let document: Value = serde_json::from_str(&json).expect("one JSON document");
    assert_eq!(
        document["merge"],
        json!({"outcome": "merged", "from": base, "to": tip})
    );
}

#[test]
fn status_shows_why_a_run_was_not_merged() {
    let sandbox = merging_project();
    let name = planned(&sandbox);
    let main = base_branch(&sandbox);
    let base = sha(&sandbox, &main);
    commit_news(&sandbox);
    let moved = sha(&sandbox, &main);
    sandbox.git(&sandbox.repo(), &["checkout", "--quiet", "--detach"]);
    surt(&sandbox, &["continue", &name]);

    let detail = surt(&sandbox, &["status", &name]).stdout;
    let json = surt(&sandbox, &["status", &name, "--json"]).stdout;

    assert!(
        detail.contains(&format!(
            "\n  merge    not merged: {main} moved since base ({} → {})\n",
            short(&base),
            short(&moved)
        )),
        "{detail}"
    );
    let document: Value = serde_json::from_str(&json).expect("one JSON document");
    assert_eq!(
        document["merge"],
        json!({"outcome": "skipped", "reason": "base_moved", "found": moved})
    );
}
