pragma circom 2.1.6;

include "./components/arithmetic_checks.circom";

template RebateValid(nBits) {
    signal input gross_fee;
    signal input rebate_amount;
    signal input protocol_fee;

    signal input bps_denom;
    signal input rebate_bps;

    signal input policy_version_private;
    signal input policy_version_public;

    signal input recipient_commitment_private;
    signal input recipient_commitment_public;

    signal input gross_fee_public;
    signal input rebate_amount_public;
    signal input protocol_fee_public;

    gross_fee === rebate_amount + protocol_fee;

    signal rebate_mul;
    rebate_mul <== rebate_amount * bps_denom;
    rebate_mul === gross_fee * rebate_bps;

    gross_fee === gross_fee_public;
    rebate_amount === rebate_amount_public;
    protocol_fee === protocol_fee_public;

    component gross_pos = GreaterThanZero(nBits);
    gross_pos.in <== gross_fee;
    gross_pos.out === 1;

    component rebate_pos = GreaterThanZero(nBits);
    rebate_pos.in <== rebate_amount;
    rebate_pos.out === 1;

    component proto_pos = GreaterThanZero(nBits);
    proto_pos.in <== protocol_fee;
    proto_pos.out === 1;

    component denom_pos = GreaterThanZero(nBits);
    denom_pos.in <== bps_denom;
    denom_pos.out === 1;

    component le_rebate = LessEq(nBits);
    le_rebate.in[0] <== rebate_amount;
    le_rebate.in[1] <== gross_fee;
    le_rebate.out === 1;

    component le_proto = LessEq(nBits);
    le_proto.in[0] <== protocol_fee;
    le_proto.in[1] <== gross_fee;
    le_proto.out === 1;

    component le_bps = LessEq(nBits);
    le_bps.in[0] <== rebate_bps;
    le_bps.in[1] <== bps_denom;
    le_bps.out === 1;

    policy_version_private === policy_version_public;
    recipient_commitment_private === recipient_commitment_public;
}

component main {public [
    policy_version_public,
    recipient_commitment_public,
    gross_fee_public,
    rebate_amount_public,
    protocol_fee_public
]} = RebateValid(64);
