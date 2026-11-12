//! V-1: a contract file copied from another repository has the sha256 of `contracts/sources.json`.

mod common;

use std::fs;

use common::{contract, root, string};
use sha2::{Digest, Sha256};

#[test]
fn copies_have_the_sha256_of_their_source() {
    let sources = contract("sources.json");
    let Some(files) = sources["files"].as_array() else { panic!("sources.json: files") };
    assert!(!files.is_empty());
    for entry in files {
        let file = string(entry, "file");
        let bytes = match fs::read(root().join(file)) {
            Ok(bytes) => bytes,
            Err(error) => panic!("{file}: {error}"),
        };
        let digest: String = Sha256::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(digest, string(entry, "sha256"), "{file} differs from {} {}", string(entry, "repository"), string(entry, "path"));
    }
}
