#[cfg(unix)]
use super::fixtures::Execution;
use super::fixtures::{REPORT, rejection};
use crate::artifacts::DeliveryRejection;
#[cfg(unix)]
use crate::runtime::Behavior;
use crate::runtime::Delivery;

fn with_data(path: &str, contents: &[u8]) -> Delivery {
    Delivery::new("approved")
        .output("report", "report.md", REPORT)
        .output("data", path, contents)
}

fn declaring_data(path: &str) -> Delivery {
    Delivery::new("approved")
        .output("report", "report.md", REPORT)
        .output_without_file("data", path)
}

#[tokio::test]
async fn paths_that_are_not_portable_relative_paths_are_rejected() {
    let unsafe_paths = [
        "",
        "../data.json",
        "out/../../data.json",
        "/tmp/data.json",
        "//server/share/data.json",
        "C:/data.json",
        "out\\data.json",
        "out/./data.json",
        "out//data.json",
        "out/data.json.",
        "con.json",
        "out/LPT1.json",
        "out/nul",
        "my data.json",
    ];

    for path in unsafe_paths {
        let rejection = rejection(declaring_data(path)).await;

        assert_eq!(
            rejection,
            DeliveryRejection::UnsafePath {
                path: path.to_owned()
            },
            "{path:?}"
        );
    }
}

#[tokio::test]
async fn a_declared_file_that_was_never_written_is_rejected() {
    let rejection = rejection(declaring_data("out/data.json")).await;

    assert_eq!(
        rejection,
        DeliveryRejection::MissingFile {
            path: "out/data.json".to_owned()
        }
    );
}

#[tokio::test]
async fn a_path_through_a_file_is_a_missing_file() {
    let rejection = rejection(declaring_data("report.md/data.json")).await;

    assert_eq!(
        rejection,
        DeliveryRejection::MissingFile {
            path: "report.md/data.json".to_owned()
        }
    );
}

#[tokio::test]
async fn a_directory_in_place_of_a_file_is_rejected() {
    let delivery = declaring_data("out/data.json").file("out/data.json/inner.json", "{}");

    let rejection = rejection(delivery).await;

    assert_eq!(
        rejection,
        DeliveryRejection::NotRegularFile {
            path: "out/data.json".to_owned()
        }
    );
}

#[tokio::test]
async fn an_empty_markdown_file_is_rejected() {
    let delivery = Delivery::new("approved")
        .output("report", "report.md", "")
        .output("data", "data.json", r#"{"count": 1}"#);

    let rejection = rejection(delivery).await;

    assert_eq!(
        rejection,
        DeliveryRejection::EmptyFile {
            path: "report.md".to_owned()
        }
    );
}

#[tokio::test]
async fn a_markdown_file_that_is_not_utf8_is_rejected() {
    let delivery = Delivery::new("approved")
        .output("report", "report.md", vec![0x23, 0x20, 0xff, 0xfe])
        .output("data", "data.json", r#"{"count": 1}"#);

    let rejection = rejection(delivery).await;

    assert_eq!(
        rejection,
        DeliveryRejection::NotUtf8 {
            path: "report.md".to_owned()
        }
    );
}

#[tokio::test]
async fn a_json_output_that_does_not_parse_is_rejected() {
    let rejection = rejection(with_data("data.json", br#"{"count": "#)).await;

    assert!(
        matches!(&rejection, DeliveryRejection::InvalidJson { path, .. } if path == "data.json"),
        "{rejection:?}"
    );
}

#[tokio::test]
async fn a_json_output_that_breaks_its_schema_is_rejected() {
    let rejection = rejection(with_data("data.json", br#"{"count": 0}"#)).await;

    assert!(
        matches!(&rejection, DeliveryRejection::SchemaViolation { path, .. } if path == "data.json"),
        "{rejection:?}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn a_symbolic_link_anywhere_in_the_path_is_rejected() {
    for (link, target, path) in [
        ("data.json", "../outside.json", "data.json"),
        ("out", "../elsewhere", "out/data.json"),
    ] {
        let execution = Execution::new();
        let root = execution.delivery_dir().join("..");
        std::fs::create_dir_all(root.join("elsewhere")).expect("a directory outside");
        std::fs::write(root.join("outside.json"), r#"{"count": 1}"#).expect("a file outside");
        std::fs::write(root.join("elsewhere/data.json"), r#"{"count": 1}"#)
            .expect("a file outside");
        let completion = execution
            .complete(Behavior::Deliver(declaring_data(path)))
            .await
            .expect("the fake completes");
        std::os::unix::fs::symlink(target, execution.delivery_dir().join(link))
            .expect("a symbolic link");

        let rejection = execution.rejection(&completion);

        assert_eq!(
            rejection,
            DeliveryRejection::Link {
                path: path.to_owned()
            }
        );
    }
}
