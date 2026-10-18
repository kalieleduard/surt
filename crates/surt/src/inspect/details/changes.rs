use super::super::fields::{Fields, joined};
use super::{LIST_SEPARATOR, moved};
use crate::symbols::DASH;

const AGENT_ORIGIN: &str = "execution";

pub(super) fn of(kind: &str, fields: Fields<'_>) -> Vec<Option<String>> {
    match kind {
        "ref_move.intended" | "intention.confirmed" | "intention.discarded" => {
            vec![intention(fields)]
        }
        "worktree.repaired" => vec![joined(texts(fields, ["repair", "path"]), " ")],
        "worktree.salvaged" => vec![fields.text("ref").map(str::to_owned)],
        "revision.published" => vec![revision(fields)],
        "working_copy.edit_detected" => vec![revision(fields), fields.labelled("by", "actor")],
        "working_copy.edit_refused" => vec![refused(fields), files(fields)],
        "approval.recorded" => vec![decision(fields), fields.labelled("by", "actor")],
        _ => Vec::new(),
    }
}

fn texts<const N: usize>(fields: Fields<'_>, keys: [&str; N]) -> [Option<String>; N] {
    keys.map(|key| fields.text(key).map(str::to_owned))
}

fn intention(fields: Fields<'_>) -> Option<String> {
    let [intention, reference] = texts(fields, ["intention", "ref"]);
    let at = fields.sha("sha").map(|sha| format!("@ {sha}"));
    joined([intention, reference, moved(fields), at], " ")
}

fn revision(fields: Fields<'_>) -> Option<String> {
    let origin = fields.text("origin").map(|origin| {
        let shown = if origin == AGENT_ORIGIN {
            "agent"
        } else {
            origin
        };
        format!("({shown})")
    });
    joined(
        [
            fields.text("artifact").map(str::to_owned),
            fields.labelled("rev", "revision"),
            origin,
        ],
        " ",
    )
}

fn decision(fields: Fields<'_>) -> Option<String> {
    let decided = fields.text("decision").map(|decision| match decision {
        "approve" => "approved".to_owned(),
        "reject" => "rejected".to_owned(),
        other => other.to_owned(),
    });
    let reason = fields
        .text("reason")
        .map(|reason| format!("{DASH} \"{reason}\""));
    joined(
        [
            fields.text("artifact").map(str::to_owned),
            fields.labelled("rev", "revision"),
            decided,
            reason,
        ],
        " ",
    )
}

fn refused(fields: Fields<'_>) -> Option<String> {
    let reason = match fields.text("reason") {
        Some("pinned") => joined(
            [
                Some("pinned".to_owned()),
                fields.labelled("rev", "revision"),
            ],
            " ",
        ),
        Some("invalid") => joined(
            [
                Some("invalid".to_owned()),
                fields.text("problem").map(str::to_owned),
            ],
            ": ",
        ),
        other => other.map(str::to_owned),
    };
    joined([fields.text("artifact").map(str::to_owned), reason], " ")
}

fn files(fields: Fields<'_>) -> Option<String> {
    let files = fields.list("files").iter().filter_map(|file| file.as_str());
    joined(files.map(|file| Some(file.to_owned())), LIST_SEPARATOR)
}
