use std::fs;
use std::path::PathBuf;

use zk_circuits_tests::{validate_rebate_vector, RebateVector};

fn read_vector(file_name: &str) -> RebateVector {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test-vectors");
    path.push(file_name);
    let raw = fs::read_to_string(path).expect("read vector");
    serde_json::from_str(&raw).expect("parse vector")
}

#[test]
fn rebate_valid_pass_vector() {
    let pass = read_vector("rebate_valid.pass.json");
    let res = validate_rebate_vector(&pass);
    assert!(res.is_ok(), "expected pass vector to validate, got {res:?}");
}

#[test]
fn rebate_valid_fail_vector() {
    let fail = read_vector("rebate_valid.fail.json");
    let res = validate_rebate_vector(&fail);
    assert!(res.is_err(), "expected fail vector to fail");
}
