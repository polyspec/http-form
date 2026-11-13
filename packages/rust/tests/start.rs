//! The start of a body (F-9): a member of the interface contract that waits is no constructor and no property, because
//! a language whose constructor or property cannot be awaited has no form of one that waits (binding GN-6m), and
//! `RequestBody` is made by its static member `request_body.start`, which returns it.
use serde_json::{json, Value};

fn interface() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/interface.json");
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => panic!("{path}: {error}"),
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => panic!("{path}: {error}"),
    }
}

fn members(contract: &Value) -> Vec<&Value> {
    let Some(units) = contract["units"].as_array() else { panic!("the contract has no list units") };
    units.iter().filter_map(|unit| unit["members"].as_array()).flatten().collect()
}

#[test]
fn no_constructor_and_no_property_waits() {
    let contract = interface();
    let members = members(&contract);
    assert!(!members.is_empty(), "the contract has no member");
    let found: Vec<String> = members
        .iter()
        .filter(|member| member["wait"] != json!("none") && !member["wait"].is_null())
        .filter(|member| member["kind"] == json!("constructor") || member["kind"] == json!("property"))
        .map(|member| format!("{}: a {} with wait {}", member["id"], member["kind"], member["wait"]))
        .collect();
    assert!(found.is_empty(), "members that wait in a form that no language awaits (F-9, binding GN-6m): {found:#?}");
}

#[test]
fn a_body_starts_by_a_static_member_that_returns_it() {
    let contract = interface();
    let members = members(&contract);
    let of_body: Vec<&&Value> = members
        .iter()
        .filter(|member| member["id"].as_str().is_some_and(|id| id.starts_with("request_body.")))
        .collect();
    assert!(!of_body.is_empty(), "the contract has no member of request_body");
    assert!(
        of_body.iter().all(|member| member["kind"] != json!("constructor")),
        "request_body has a constructor (F-9)"
    );
    let Some(start) = of_body.iter().find(|member| member["id"] == json!("request_body.start")) else {
        panic!("the contract has no member request_body.start (F-9)")
    };
    assert_eq!(start["kind"], json!("static"), "request_body.start is a static member (F-9)");
    assert_eq!(start["returns"], json!({"unit": "request_body"}), "request_body.start returns the body (F-9)");
    assert_eq!(start["wait"], json!("blocking_io"), "request_body.start waits on the directory (F-10, F-17)");
    assert_eq!(
        start["core"]["path"],
        json!("polyspec_http_form::RequestBody::start"),
        "the core function of the start"
    );
}
