// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {AccessController} from "../src/contracts/AccessController.sol";
import {PolicyManager} from "../src/contracts/PolicyManager.sol";
import {Verifier} from "../src/contracts/Verifier.sol";
import {SignalBinding} from "../src/contracts/SignalBinding.sol";
import {ReplayProtection} from "../src/contracts/ReplayProtection.sol";
import {SettlementRegistry} from "../src/contracts/SettlementRegistry.sol";
import {SettlementValidGroth16Verifier} from "../src/contracts/generated/SettlementValidGroth16Verifier.sol";
import {ISettlementRegistry} from "../src/interfaces/ISettlementRegistry.sol";
import {ISignalBinding} from "../src/interfaces/ISignalBinding.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";

contract SettlementRegistryGroth16E2ETest is Test {
    AccessController internal accessController;
    PolicyManager internal policyManager;
    Verifier internal verifier;
    SignalBinding internal signalBinding;
    ReplayProtection internal replayProtection;
    SettlementRegistry internal settlementRegistry;
    SettlementValidGroth16Verifier internal groth16;
    bytes internal proof;
    uint256[] internal publicSignals;
    uint64 internal policyVersion;
    bytes32 internal runId;
    bytes32 internal receiptHash;

    address internal admin = address(this);
    address internal publisher = address(0xBEEF);

    function setUp() public {
        string memory proofRaw = vm.readFile("../zk/artifacts/settlement_valid/settlement_valid.proof.json");
        string memory publicRaw = vm.readFile("../zk/artifacts/settlement_valid/settlement_valid.public.json");

        publicSignals = vm.parseJsonUintArray(publicRaw, ".");
        policyVersion = uint64(publicSignals[0]);
        bytes32 domain = bytes32(publicSignals[2]);
        runId = bytes32(publicSignals[3]);
        receiptHash = bytes32(publicSignals[1]);

        accessController = new AccessController(admin);
        policyManager = new PolicyManager(address(accessController));
        verifier = new Verifier(address(accessController), keccak256("verifier-v1"));
        signalBinding = new SignalBinding(address(accessController), domain);
        replayProtection = new ReplayProtection(admin);
        settlementRegistry = new SettlementRegistry(
            address(policyManager), address(verifier), address(signalBinding), address(replayProtection), address(accessController)
        );
        groth16 = new SettlementValidGroth16Verifier();

        verifier.setGroth16Verifier(address(groth16));
        accessController.setWorkflowPublisher(publisher, true);
        replayProtection.setAuthorizedCaller(address(settlementRegistry), true);

        policyManager.commitPolicy(policyVersion, keccak256("policy-v1"), keccak256("meta-v1"));
        policyManager.activatePolicy(policyVersion);

        uint256[] memory piA = vm.parseJsonUintArray(proofRaw, ".pi_a");
        uint256[] memory piB0 = vm.parseJsonUintArray(proofRaw, ".pi_b[0]");
        uint256[] memory piB1 = vm.parseJsonUintArray(proofRaw, ".pi_b[1]");
        uint256[] memory piC = vm.parseJsonUintArray(proofRaw, ".pi_c");

        uint256[2] memory pA = [piA[0], piA[1]];
        uint256[2][2] memory pB = [[piB0[1], piB0[0]], [piB1[1], piB1[0]]];
        uint256[2] memory pC = [piC[0], piC[1]];
        proof = abi.encode(pA, pB, pC);
    }

    function test_PublishReceiptWithRealGroth16Proof() public {
        (uint256[2] memory pA, uint256[2][2] memory pB, uint256[2] memory pC) =
            abi.decode(proof, (uint256[2], uint256[2][2], uint256[2]));
        uint256[6] memory pubFixed =
            [publicSignals[0], publicSignals[1], publicSignals[2], publicSignals[3], publicSignals[4], publicSignals[5]];
        assertTrue(groth16.verifyProof(pA, pB, pC, pubFixed), "groth16 verify failed");

        ISignalBinding.BindingContext memory context = ISignalBinding.BindingContext({
            workflowRunId: runId,
            policyVersion: policyVersion,
            receiptHash: receiptHash,
            domainSeparator: signalBinding.domainSeparator()
        });
        assertTrue(signalBinding.validateSignalBinding(publicSignals, context), "binding failed");

        IVerifier.VerifierInput memory vInput = IVerifier.VerifierInput({proof: proof, publicSignals: publicSignals});
        IVerifier.VerifierContext memory vContext = IVerifier.VerifierContext({
            statementHash: keccak256(abi.encode(proof, publicSignals)),
            policyVersion: policyVersion,
            domainSeparator: signalBinding.domainSeparator()
        });
        assertTrue(verifier.verifyProofWithContext(vInput, vContext), "verifier path failed");

        ISettlementRegistry.PublishParams memory params = ISettlementRegistry.PublishParams({
            workflowRunId: runId,
            proofHash: keccak256(proof),
            policyVersion: policyVersion,
            status: ISettlementRegistry.SettlementStatus.SETTLED,
            receiptHash: receiptHash,
            proof: proof,
            publicSignals: publicSignals
        });

        vm.prank(publisher);
        settlementRegistry.publishReceipt(params);
        assertTrue(settlementRegistry.receiptExists(runId));
    }
}
