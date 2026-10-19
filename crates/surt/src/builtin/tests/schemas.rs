use surt_engine::project::{BASE_SCHEMA, CHECK_REPORT_SCHEMA, CONFLICT_SCHEMA, INTEGRATION_SCHEMA};

use super::super::AGENTS;
use super::agent;

#[test]
fn agents_reading_operation_artifacts_validate_them_with_the_engines_schemas() {
    let readers = [
        ("builder", "conflict", CONFLICT_SCHEMA),
        ("reviewer", "conflict", CONFLICT_SCHEMA),
        ("shipper", "integration", INTEGRATION_SCHEMA),
    ];

    for (id, input, (path, text)) in readers {
        let agent = agent(id);
        let (_, assets) = AGENTS
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .expect("the agent");

        assert_eq!(
            agent["inputs"][input]["json_schema"].as_str(),
            Some(path),
            "{id}"
        );
        assert!(assets.schemas.contains(&(path, text)), "{id}");
    }
    let (_, shipper) = AGENTS
        .iter()
        .find(|(candidate, _)| *candidate == "shipper")
        .expect("the shipper");
    assert!(shipper.schemas.contains(&BASE_SCHEMA));
}

#[test]
fn agents_reading_a_check_report_validate_it_with_the_engines_schema() {
    let (path, text) = CHECK_REPORT_SCHEMA;

    for id in ["builder", "reviewer"] {
        let agent = agent(id);
        let (_, assets) = AGENTS
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .expect("the agent");

        assert_eq!(
            agent["inputs"]["check-report"]["json_schema"].as_str(),
            Some(path),
            "{id}"
        );
        assert!(assets.schemas.contains(&(path, text)), "{id}");
    }
}

#[test]
fn every_embedded_schema_is_a_draft_07_json_schema() {
    for (id, assets) in AGENTS {
        for (path, text) in assets.schemas {
            let schema: serde_json::Value =
                serde_json::from_str(text).unwrap_or_else(|error| panic!("{id} {path}: {error}"));

            assert_eq!(
                schema["$schema"].as_str(),
                Some("http://json-schema.org/draft-07/schema#"),
                "{id} {path}"
            );
        }
    }
}
