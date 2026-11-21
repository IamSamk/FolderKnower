//! M1.2 "done when": a hand-written graph round-trips and validates against the schema.

use ark_schema::{SystemArchitectureGraph, json_schema_string, to_canonical_json};

const SAMPLE: &str = include_str!("fixtures/sample.json");

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn validator() -> jsonschema::Validator {
    let schema: serde_json::Value = serde_json::from_str(&json_schema_string()).unwrap();
    jsonschema::validator_for(&schema).expect("generated schema compiles")
}

#[test]
fn sample_round_trips_byte_identically() {
    let graph: SystemArchitectureGraph = serde_json::from_str(SAMPLE).unwrap();
    let out = to_canonical_json(&graph).unwrap();
    assert_eq!(out, normalize(SAMPLE));

    let again: SystemArchitectureGraph = serde_json::from_str(&out).unwrap();
    assert_eq!(again, graph);
}

#[test]
fn sample_validates_against_generated_schema() {
    let instance: serde_json::Value = serde_json::from_str(SAMPLE).unwrap();
    let v = validator();
    let errors: Vec<String> = v.iter_errors(&instance).map(|e| e.to_string()).collect();
    assert!(errors.is_empty(), "schema errors: {errors:#?}");
}

#[test]
fn schema_rejects_bad_values() {
    let v = validator();
    let bad = [
        ("/schemaVersion", serde_json::json!("1.0")),
        ("/tiers/0/label", serde_json::json!("Database")),
        (
            "/tiers/1/clusters/0/files/0/imports/0/provenance",
            serde_json::json!("llm_guess"),
        ),
        (
            "/tiers/1/clusters/0/files/0/imports/0/confidence",
            serde_json::json!(1.5),
        ),
    ];
    for (ptr, value) in bad {
        let mut instance: serde_json::Value = serde_json::from_str(SAMPLE).unwrap();
        *instance.pointer_mut(ptr).unwrap() = value;
        assert!(!v.is_valid(&instance), "{ptr} should be rejected");
    }
}

#[test]
fn unknown_fields_are_rejected() {
    let mut instance: serde_json::Value = serde_json::from_str(SAMPLE).unwrap();
    instance["surprise"] = serde_json::json!(true);
    assert!(serde_json::from_value::<SystemArchitectureGraph>(instance.clone()).is_err());
    assert!(!validator().is_valid(&instance));
}
