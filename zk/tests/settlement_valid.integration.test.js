const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const { wasm: wasm_tester } = require("circom_tester");

describe("settlement_valid circuit", function () {
  this.timeout(180000);

  const circuitPath = path.join(__dirname, "..", "circuits", "settlement_valid.circom");
  const passVectorPath = path.join(__dirname, "..", "test-vectors", "settlement_valid.pass.json");
  const failVectorPath = path.join(__dirname, "..", "test-vectors", "settlement_valid.fail.json");

  let circuit;

  before(async function () {
    circuit = await wasm_tester(circuitPath);
  });

  it("accepts valid settlement constraints", async function () {
    const input = JSON.parse(fs.readFileSync(passVectorPath, "utf8"));
    const witness = await circuit.calculateWitness(input, true);
    await circuit.checkConstraints(witness);
    assert.ok(witness.length > 0);
  });

  it("rejects invalid settlement constraints", async function () {
    const input = JSON.parse(fs.readFileSync(failVectorPath, "utf8"));
    await assert.rejects(async () => {
      const witness = await circuit.calculateWitness(input, true);
      await circuit.checkConstraints(witness);
    });
  });
});
