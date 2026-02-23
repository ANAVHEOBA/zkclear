// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

interface IReplayProtection {
    error DuplicateWorkflowRun();
    error DuplicateReceiptHash();
    error Unauthorized();

    event WorkflowRunFinalized(bytes32 indexed workflowRunId, address indexed caller, uint256 timestamp);
    event ReceiptHashMarked(bytes32 indexed receiptHash, address indexed caller, uint256 timestamp);

    function markWorkflowRunFinalized(bytes32 workflowRunId) external;
    function markReceiptHashUsed(bytes32 receiptHash) external;

    function isWorkflowRunFinalized(bytes32 workflowRunId) external view returns (bool);
    function isReceiptHashUsed(bytes32 receiptHash) external view returns (bool);
}
