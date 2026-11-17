//! The case convention of binding (G-5): every key of a case of `contracts/fields.json` and `contracts/forms.json` is in
//! snake_case, because the generated tests read a parameter or a result of a member by its name in snake_case. The
//! groups that only the core's tests read follow the same convention, so that one form holds for every case.
mod common;

use common::{contract, id};

/// Whether a key is in snake_case: lowercase ASCII letters, digits and `_`, starting with a letter.
fn snake_case(key: &str) -> bool {
    let mut characters = key.chars();
    characters.next().is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_')
}

#[test]
fn every_case_key_is_in_snake_case() {
    let mut found = Vec::new();
    let mut read = 0_usize;
    for file in ["fields.json", "forms.json"] {
        let document = contract(file);
        let Some(groups) = document.as_object() else { panic!("contracts/{file} is no object") };
        for (group, cases) in groups {
            let Some(cases) = cases.as_array() else { panic!("contracts/{file}#{group} is no list") };
            for case in cases {
                let Some(keys) = case.as_object() else { panic!("contracts/{file}#{group}: a case is no object") };
                read += 1;
                for key in keys.keys().filter(|key| !snake_case(key)) {
                    found.push(format!("contracts/{file}#{group}/{}: the key {key} is not in snake_case", id(case)));
                }
            }
        }
    }
    assert!(read > 0, "the contracts hold no case");
    assert!(found.is_empty(), "{read} cases read, {} keys not in snake_case:\n{}", found.len(), found.join("\n"));
}

/// Whether a text is the id of a rule: capital ASCII letters, `-`, then numbers joined by `.`, such as `F-12.3`.
fn rule_id(text: &str) -> bool {
    let Some((prefix, numbers)) = text.split_once('-') else { return false };
    !prefix.is_empty()
        && prefix.chars().all(|character| character.is_ascii_uppercase())
        && numbers
            .split('.')
            .all(|number| !number.is_empty() && number.chars().all(|character| character.is_ascii_digit()))
}

/// The findings of the key `rules` of the cases of a contract file: a value that is no list or is empty, and an entry
/// that is no id of a rule. Gives the number of cases that hold the key too.
fn rules_findings(file: &str, document: &serde_json::Value) -> (usize, Vec<String>) {
    let mut found = Vec::new();
    let mut read = 0_usize;
    let Some(groups) = document.as_object() else { panic!("contracts/{file} is no object") };
    for (group, cases) in groups {
        let Some(cases) = cases.as_array() else { panic!("contracts/{file}#{group} is no list") };
        for case in cases {
            let Some(rules) = case.get("rules") else { continue };
            read += 1;
            match rules.as_array() {
                Some(list) if !list.is_empty() => {
                    for rule in list.iter().filter(|rule| !rule.as_str().is_some_and(rule_id)) {
                        found.push(format!("contracts/{file}#{group}/{}: {rule} is no id of a rule", id(case)));
                    }
                }
                _ => found.push(format!("contracts/{file}#{group}/{}: rules is no list of rules", id(case))),
            }
        }
    }
    (read, found)
}

/// The key `rules` of a case names the rules of the specifications that the case pins. No generated test reads it
/// (the rows of `packages/rust/interface.json` name it in `reads`), so this test holds its form.
#[test]
fn the_rules_of_every_case_are_ids_of_rules() {
    let mut found = Vec::new();
    let mut read = 0_usize;
    for file in ["fields.json", "forms.json"] {
        let (cases, findings) = rules_findings(file, &contract(file));
        read += cases;
        found.extend(findings);
    }
    assert!(read > 0, "no case names its rules");
    assert!(found.is_empty(), "{read} cases read, {} findings:\n{}", found.len(), found.join("\n"));
}

#[test]
fn rules_that_are_no_ids_of_rules_are_found() {
    let fixture = serde_json::json!({"group": [
        {"id": "known", "rules": ["F-12.3", "MT-1"]},
        {"id": "empty", "rules": []},
        {"id": "text", "rules": "F-1"},
        {"id": "unknown", "rules": ["F-", "f-1", 3]},
        {"id": "none"}
    ]});
    let (read, found) = rules_findings("fixture.json", &fixture);
    assert_eq!(read, 4);
    assert_eq!(
        found,
        [
            "contracts/fixture.json#group/empty: rules is no list of rules",
            "contracts/fixture.json#group/text: rules is no list of rules",
            "contracts/fixture.json#group/unknown: \"F-\" is no id of a rule",
            "contracts/fixture.json#group/unknown: \"f-1\" is no id of a rule",
            "contracts/fixture.json#group/unknown: 3 is no id of a rule",
        ]
    );
}
