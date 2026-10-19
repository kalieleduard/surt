use surt_engine::project::{BASE_SCHEMA, CHECK_REPORT_SCHEMA, CONFLICT_SCHEMA, INTEGRATION_SCHEMA};

use super::{agent, feature, file};

fn assert_engine_schema(id: &str, (path, text): (&str, &str)) {
    assert_eq!(
        file(&format!("agents/{id}/{path}")),
        Some(text),
        "{id} {path}"
    );
}

#[test]
fn agents_reading_operation_artifacts_validate_them_with_the_engines_schemas() {
    let readers = [
        ("builder", "conflict", CONFLICT_SCHEMA),
        ("reviewer", "conflict", CONFLICT_SCHEMA),
        ("shipper", "integration", INTEGRATION_SCHEMA),
    ];

    for (id, input, schema) in readers {
        assert_eq!(
            agent(id)["inputs"][input]["json_schema"].as_str(),
            Some(schema.0),
            "{id}"
        );
        assert_engine_schema(id, schema);
    }
    assert_engine_schema("shipper", BASE_SCHEMA);
}

#[test]
fn agents_reading_a_check_report_validate_it_with_the_engines_schema() {
    for id in ["builder", "reviewer"] {
        assert_eq!(
            agent(id)["inputs"]["check-report"]["json_schema"].as_str(),
            Some(CHECK_REPORT_SCHEMA.0),
            "{id}"
        );
        assert_engine_schema(id, CHECK_REPORT_SCHEMA);
    }
}

#[test]
fn every_template_schema_is_a_draft_07_json_schema() {
    let schemas = feature()
        .files()
        .into_iter()
        .filter(|(path, _)| path.ends_with(".schema.json"));

    for (path, text) in schemas {
        let schema: serde_json::Value =
            serde_json::from_str(text).unwrap_or_else(|error| panic!("{path}: {error}"));

        assert_eq!(
            schema["$schema"].as_str(),
            Some("http://json-schema.org/draft-07/schema#"),
            "{path}"
        );
    }
}
