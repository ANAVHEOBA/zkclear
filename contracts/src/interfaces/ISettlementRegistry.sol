// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

interface ISettlementRegistry {
    enum SettlementStatus {
        NONE,
        ACCEPTED,
        REJECTED,
        SETTLED,
        FAILED
    }

    error Unauthorized();
    error InvalidPolicyVersion();
    error InvalidProof();
    error DuplicateWorkflowRun();
    error DuplicateReceiptHash();
    error InvalidStatus();
    error RegistryPaused();

    event WorkflowPublisherUpdated(address indexed account, bool allowed);
    event RegistryPausedStateChanged(bool paused);
    event ReceiptPublished(
        bytes32 indexed workflowRunId,
        bytes32 indexed receiptHash,
        uint64 indexed policyVersion,
        SettlementStatus status,
        bytes32 proofHash,
        uint256 timestamp
    );

    struct Receipt {
        bytes32 workflowRunId;
        bytes32 proofHash;
        uint64 policyVersion;
        SettlementStatus status;
        uint256 timestamp;
        bytes32 receiptHash;
    }

    struct PublishParams {
        bytes32 workflowRunId;
        bytes32 proofHash;
        uint64 policyVersion;
        SettlementStatus status;
        bytes32 receiptHash;
        bytes proof;
        uint256[] publicSignals;
    }

    function publishReceipt(PublishParams calldata params) external;
    function getReceipt(bytes32 workflowRunId) external view returns (Receipt memory);
    function receiptExists(bytes32 workflowRunId) external view returns (bool);
    function isReceiptHashUsed(bytes32 receiptHash) external view returns (bool);
    function setWorkflowPublisher(address account, bool allowed) external;
    function isWorkflowPublisher(address account) external view returns (bool);
    function setPaused(bool paused) external;
    function paused() external view returns (bool);
}
