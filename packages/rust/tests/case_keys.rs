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
