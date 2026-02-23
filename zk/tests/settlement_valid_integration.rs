use std::fs;
use std::path::PathBuf;

use zk_circuits_tests::{max_field_for_nbits, validate_settlement_vector, SettlementVector};

fn read_vector(file_name: &str) -> SettlementVector {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test-vectors");
    path.push(file_name);
    let raw = fs::read_to_string(path).expect("read vector");
    serde_json::from_str(&raw).expect("parse vector")
}

#[test]
fn settlement_valid_pass_vector() {
    let pass = read_vector("settlement_valid.pass.json");
    let res = validate_settlement_vector(&pass);
    assert!(res.is_ok(), "expected pass vector to validate, got {res:?}");
}

#[test]
fn settlement_valid_fail_vector() {
    let fail = read_vector("settlement_valid.fail.json");
    let res = validate_settlement_vector(&fail);
    assert!(res.is_err(), "expected fail vector to fail");
}

#[test]
fn max_field_helper_is_correct_for_64_bits() {
    let max = max_field_for_nbits(64);
    assert_eq!(max.to_string(), "18446744073709551615");
}
