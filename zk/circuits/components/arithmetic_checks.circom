pragma circom 2.1.6;

template Num2Bits(n) {
    signal input in;
    signal output out[n];

    var i;
    var lc = 0;

    for (i = 0; i < n; i++) {
        out[i] <-- (in >> i) & 1;
        out[i] * (out[i] - 1) === 0;
        lc += out[i] * (1 << i);
    }

    lc === in;
}

template IsZero() {
    signal input in;
    signal output out;

    signal inv;
    inv <-- in != 0 ? 1 / in : 0;
    out <== 1 - in * inv;
    in * out === 0;
}

template LessThan(n) {
    assert(n <= 252);
    signal input in[2];
    signal output out;

    component n2b = Num2Bits(n + 1);
    n2b.in <== in[0] + (1 << n) - in[1];
    out <== 1 - n2b.out[n];
}

template LessEq(n) {
    signal input in[2];
    signal output out;

    component lt = LessThan(n);
    lt.in[0] <== in[0];
    lt.in[1] <== in[1] + 1;
    out <== lt.out;
}

template GreaterThanZero(n) {
    signal input in;
    signal output out;

    component isz = IsZero();
    isz.in <== in;

    component le = LessEq(n);
    le.in[0] <== 1;
    le.in[1] <== in;

    out <== (1 - isz.out) * le.out;
}
