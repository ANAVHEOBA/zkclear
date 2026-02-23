pragma circom 2.1.6;

include "./components/arithmetic_checks.circom";

template SettlementValid(nBits) {
    signal input amount_in;
    signal input amount_out;
    signal input fee;

    signal input execution_size;
    signal input execution_price;
    signal input limit_price;
    signal input max_notional;

    signal input notional;

    signal input policy_version_private;
    signal input receipt_hash_private;
    signal input domain_separator_private;
    signal input workflow_run_id_private;

    signal input policy_version_public;
    signal input receipt_hash_public;
    signal input domain_separator_public;
    signal input workflow_run_id_public;
    signal input binding_hash_public;
    signal input notional_public;

    amount_in === amount_out + fee;
    notional === execution_size * execution_price;
    notional === notional_public;

    component in_pos = GreaterThanZero(nBits);
    in_pos.in <== amount_in;
    in_pos.out === 1;

    component out_pos = GreaterThanZero(nBits);
    out_pos.in <== amount_out;
    out_pos.out === 1;

    component fee_pos = GreaterThanZero(nBits);
    fee_pos.in <== fee;
    fee_pos.out === 1;

    component size_pos = GreaterThanZero(nBits);
    size_pos.in <== execution_size;
    size_pos.out === 1;

    component px_pos = GreaterThanZero(nBits);
    px_pos.in <== execution_price;
    px_pos.out === 1;

    component limit_pos = GreaterThanZero(nBits);
    limit_pos.in <== limit_price;
    limit_pos.out === 1;

    component notional_pos = GreaterThanZero(nBits);
    notional_pos.in <== notional;
    notional_pos.out === 1;

    component le_notional = LessEq(nBits);
    le_notional.in[0] <== notional;
    le_notional.in[1] <== max_notional;
    le_notional.out === 1;

    component le_limit = LessEq(nBits);
    le_limit.in[0] <== execution_price;
    le_limit.in[1] <== limit_price;
    le_limit.out === 1;

    policy_version_private === policy_version_public;
    receipt_hash_private === receipt_hash_public;
    domain_separator_private === domain_separator_public;
    workflow_run_id_private === workflow_run_id_public;

    signal binding_calc;
    binding_calc <== workflow_run_id_public * 23 + policy_version_public * 131 + receipt_hash_public * 17 + domain_separator_public * 19;
    binding_calc === binding_hash_public;
}

component main {public [
    binding_hash_public,
    workflow_run_id_public,
    receipt_hash_public,
    policy_version_public,
    domain_separator_public,
    notional_public
]} = SettlementValid(64);
