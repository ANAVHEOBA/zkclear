// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {ISettlementRegistry} from "../interfaces/ISettlementRegistry.sol";
import {IPolicyManager} from "../interfaces/IPolicyManager.sol";
import {IVerifier} from "../interfaces/IVerifier.sol";
import {ISignalBinding} from "../interfaces/ISignalBinding.sol";
import {IReplayProtection} from "../interfaces/IReplayProtection.sol";
import {IAccessController} from "../interfaces/IAccessController.sol";

contract SettlementRegistry is ISettlementRegistry {
    IPolicyManager public policyManager;
    IVerifier public verifier;
    ISignalBinding public signalBinding;
    IReplayProtection public replayProtection;
    IAccessController public accessController;

    mapping(bytes32 => Receipt) private _receipts;
    mapping(bytes32 => bool) private _receiptExists;

    constructor(
        address policyManager_,
        address verifier_,
        address signalBinding_,
        address replayProtection_,
        address accessController_
    ) {
        policyManager = IPolicyManager(policyManager_);
        verifier = IVerifier(verifier_);
        signalBinding = ISignalBinding(signalBinding_);
        replayProtection = IReplayProtection(replayProtection_);
        accessController = IAccessController(accessController_);
    }

    function publishReceipt(PublishParams calldata params) external {
        if (accessController.paused()) revert RegistryPaused();
        if (!accessController.isWorkflowPublisher(msg.sender)) revert Unauthorized();
        if (params.status == SettlementStatus.NONE) revert InvalidStatus();
        if (_receiptExists[params.workflowRunId]) revert DuplicateWorkflowRun();
        if (replayProtection.isWorkflowRunFinalized(params.workflowRunId)) revert DuplicateWorkflowRun();
        if (replayProtection.isReceiptHashUsed(params.receiptHash)) revert DuplicateReceiptHash();
        if (!policyManager.isPolicyActive(params.policyVersion)) revert InvalidPolicyVersion();

        ISignalBinding.BindingContext memory context = ISignalBinding.BindingContext({
            workflowRunId: params.workflowRunId,
            policyVersion: params.policyVersion,
            receiptHash: params.receiptHash,
            domainSeparator: signalBinding.domainSeparator()
        });

        bool bindingOk = signalBinding.validateSignalBinding(params.publicSignals, context);
        if (!bindingOk) revert InvalidProof();

        IVerifier.VerifierInput memory input = IVerifier.VerifierInput({
            proof: params.proof,
            publicSignals: params.publicSignals
        });

        IVerifier.VerifierContext memory verifierContext = IVerifier.VerifierContext({
            statementHash: _hashProofEnvelope(params.proof, params.publicSignals),
            policyVersion: params.policyVersion,
            domainSeparator: context.domainSeparator
        });

        bool isValid = verifier.verifyProofWithContext(input, verifierContext);
        if (!isValid) revert InvalidProof();

        Receipt memory receipt = Receipt({
            workflowRunId: params.workflowRunId,
            proofHash: params.proofHash,
            policyVersion: params.policyVersion,
            status: params.status,
            timestamp: block.timestamp,
            receiptHash: params.receiptHash
        });

        _receipts[params.workflowRunId] = receipt;
        _receiptExists[params.workflowRunId] = true;

        replayProtection.markWorkflowRunFinalized(params.workflowRunId);
        replayProtection.markReceiptHashUsed(params.receiptHash);

        emit ReceiptPublished(
            params.workflowRunId,
            params.receiptHash,
            params.policyVersion,
            params.status,
            params.proofHash,
            block.timestamp
        );
    }

    function getReceipt(bytes32 workflowRunId) external view returns (Receipt memory) {
        return _receipts[workflowRunId];
    }

    function receiptExists(bytes32 workflowRunId) external view returns (bool) {
        return _receiptExists[workflowRunId];
    }

    function isReceiptHashUsed(bytes32 receiptHash) external view returns (bool) {
        return replayProtection.isReceiptHashUsed(receiptHash);
    }

    function setWorkflowPublisher(address account, bool allowed) external {
        accessController.setWorkflowPublisher(account, allowed);
        emit WorkflowPublisherUpdated(account, allowed);
    }

    function isWorkflowPublisher(address account) external view returns (bool) {
        return accessController.isWorkflowPublisher(account);
    }

    function setPaused(bool paused_) external {
        accessController.setPaused(paused_);
        emit RegistryPausedStateChanged(paused_);
    }

    function paused() external view returns (bool) {
        return accessController.paused();
    }

    function _hashProofEnvelope(bytes calldata proof, uint256[] calldata publicSignals) internal pure returns (bytes32 digest) {
        bytes memory encoded = abi.encode(proof, publicSignals);
        assembly {
            digest := keccak256(add(encoded, 0x20), mload(encoded))
        }
    }
}
