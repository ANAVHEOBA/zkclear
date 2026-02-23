pragma circom 2.1.6;

include "./components/arithmetic_checks.circom";

template ComplianceValid(nBits) {
    signal input compliance_pass_bit;
    signal input risk_score;
    signal input max_risk_score;

    signal input policy_version_private;
    signal input policy_version_public;

    signal input attestation_hash_private;
    signal input attestation_hash_public;

    signal input sanctions_commitment_private;
    signal input sanctions_commitment_public;

    signal input allowlist_commitment_private;
    signal input allowlist_commitment_public;

    compliance_pass_bit * (compliance_pass_bit - 1) === 0;
    compliance_pass_bit === 1;

    component risk_le = LessEq(nBits);
    risk_le.in[0] <== risk_score;
    risk_le.in[1] <== max_risk_score;
    risk_le.out === 1;

    policy_version_private === policy_version_public;
    attestation_hash_private === attestation_hash_public;
    sanctions_commitment_private === sanctions_commitment_public;
    allowlist_commitment_private === allowlist_commitment_public;
}

component main {public [
    policy_version_public,
    attestation_hash_public,
    sanctions_commitment_public,
    allowlist_commitment_public
]} = ComplianceValid(64);
