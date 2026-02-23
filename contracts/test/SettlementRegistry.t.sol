// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {AccessController} from "../src/contracts/AccessController.sol";
import {PolicyManager} from "../src/contracts/PolicyManager.sol";
import {Verifier} from "../src/contracts/Verifier.sol";
import {SignalBinding} from "../src/contracts/SignalBinding.sol";
import {ReplayProtection} from "../src/contracts/ReplayProtection.sol";
import {SettlementRegistry} from "../src/contracts/SettlementRegistry.sol";
import {ISettlementRegistry} from "../src/interfaces/ISettlementRegistry.sol";
import {ISignalBinding} from "../src/interfaces/ISignalBinding.sol";

contract SettlementRegistryTest is Test {
    AccessController internal accessController;
    PolicyManager internal policyManager;
    Verifier internal verifier;
    SignalBinding internal signalBinding;
    ReplayProtection internal replayProtection;
    SettlementRegistry internal settlementRegistry;

    address internal admin = address(this);
    address internal publisher = address(0xBEEF);
    bytes32 internal domain = keccak256("zkclear-domain");
    uint64 internal policyVersion = 1;

    function setUp() public {
        accessController = new AccessController(admin);
        policyManager = new PolicyManager(address(accessController));
        verifier = new Verifier(address(accessController), keccak256("verifier-v1"));
        signalBinding = new SignalBinding(address(accessController), domain);
        replayProtection = new ReplayProtection(admin);
        settlementRegistry = new SettlementRegistry(
            address(policyManager), address(verifier), address(signalBinding), address(replayProtection), address(accessController)
        );

        accessController.setWorkflowPublisher(publisher, true);
        replayProtection.setAuthorizedCaller(address(settlementRegistry), true);

        policyManager.commitPolicy(policyVersion, keccak256("policy-v1"), keccak256("meta-v1"));
        policyManager.activatePolicy(policyVersion);
    }

    function test_PublishReceiptHappyPath() public {
        ISettlementRegistry.PublishParams memory params = _buildValidParams(keccak256("run-1"), keccak256("receipt-1"));

        vm.prank(publisher);
        settlementRegistry.publishReceipt(params);

        ISettlementRegistry.Receipt memory receipt = settlementRegistry.getReceipt(params.workflowRunId);
        assertEq(receipt.workflowRunId, params.workflowRunId);
        assertEq(receipt.proofHash, params.proofHash);
        assertEq(receipt.policyVersion, params.policyVersion);
        assertEq(uint256(receipt.status), uint256(params.status));
        assertEq(receipt.receiptHash, params.receiptHash);
        assertTrue(settlementRegistry.receiptExists(params.workflowRunId));
        assertTrue(settlementRegistry.isReceiptHashUsed(params.receiptHash));
    }

    function test_RevertWhen_UnauthorizedPublisher() public {
        ISettlementRegistry.PublishParams memory params = _buildValidParams(keccak256("run-2"), keccak256("receipt-2"));

        vm.expectRevert(ISettlementRegistry.Unauthorized.selector);
        settlementRegistry.publishReceipt(params);
    }

    function test_RevertWhen_RegistryPaused() public {
        accessController.setPaused(true);
        ISettlementRegistry.PublishParams memory params = _buildValidParams(keccak256("run-3"), keccak256("receipt-3"));

        vm.prank(publisher);
        vm.expectRevert(ISettlementRegistry.RegistryPaused.selector);
        settlementRegistry.publishReceipt(params);
    }

    function test_RevertWhen_InvalidPolicyVersion() public {
        ISettlementRegistry.PublishParams memory params = _buildValidParams(keccak256("run-4"), keccak256("receipt-4"));
        params.policyVersion = 77;

        vm.prank(publisher);
        vm.expectRevert(ISettlementRegistry.InvalidPolicyVersion.selector);
        settlementRegistry.publishReceipt(params);
    }

    function test_RevertWhen_DuplicateWorkflowRun() public {
        ISettlementRegistry.PublishParams memory params = _buildValidParams(keccak256("run-5"), keccak256("receipt-5"));

        vm.prank(publisher);
        settlementRegistry.publishReceipt(params);

        vm.prank(publisher);
        vm.expectRevert(ISettlementRegistry.DuplicateWorkflowRun.selector);
        settlementRegistry.publishReceipt(params);
    }

    function test_RevertWhen_InvalidProofBinding() public {
        ISettlementRegistry.PublishParams memory params = _buildValidParams(keccak256("run-6"), keccak256("receipt-6"));
        params.publicSignals[0] = 12345;

        vm.prank(publisher);
        vm.expectRevert(ISettlementRegistry.InvalidProof.selector);
        settlementRegistry.publishReceipt(params);
    }

    function _buildValidParams(bytes32 runId, bytes32 receiptHash)
        internal
        returns (ISettlementRegistry.PublishParams memory params)
    {
        ISignalBinding.BindingContext memory context = ISignalBinding.BindingContext({
            workflowRunId: runId,
            policyVersion: policyVersion,
            receiptHash: receiptHash,
            domainSeparator: signalBinding.domainSeparator()
        });

        bytes32 bindingHash = signalBinding.computeBindingHash(context);
        uint256[] memory signals = new uint256[](6);
        signals[0] = policyVersion;
        signals[1] = uint64(uint256(receiptHash));
        signals[2] = uint64(uint256(signalBinding.domainSeparator()));
        signals[3] = uint64(uint256(runId));
        signals[4] = uint256(bindingHash);
        signals[5] = 1000;

        bytes memory proof = hex"c0ffee";
        bytes32 digest = keccak256(abi.encode(proof, signals));
        verifier.setApprovedProofDigest(digest, true);

        params = ISettlementRegistry.PublishParams({
            workflowRunId: runId,
            proofHash: keccak256(abi.encodePacked(proof, runId)),
            policyVersion: policyVersion,
            status: ISettlementRegistry.SettlementStatus.SETTLED,
            receiptHash: receiptHash,
            proof: proof,
            publicSignals: signals
        });
    }
}
