// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {IReplayProtection} from "../interfaces/IReplayProtection.sol";

contract ReplayProtection is IReplayProtection {
    address public admin;
    mapping(address => bool) public authorizedCallers;
    mapping(bytes32 => bool) private _workflowRuns;
    mapping(bytes32 => bool) private _receiptHashes;

    constructor(address admin_) {
        admin = admin_;
        authorizedCallers[admin_] = true;
    }

    modifier onlyAuthorized() {
        _onlyAuthorized();
        _;
    }

    function _onlyAuthorized() internal view {
        if (!authorizedCallers[msg.sender]) revert Unauthorized();
    }

    function setAuthorizedCaller(address caller, bool allowed) external {
        if (msg.sender != admin) revert Unauthorized();
        authorizedCallers[caller] = allowed;
    }

    function markWorkflowRunFinalized(bytes32 workflowRunId) external onlyAuthorized {
        if (_workflowRuns[workflowRunId]) revert DuplicateWorkflowRun();
        _workflowRuns[workflowRunId] = true;
        emit WorkflowRunFinalized(workflowRunId, msg.sender, block.timestamp);
    }

    function markReceiptHashUsed(bytes32 receiptHash) external onlyAuthorized {
        if (_receiptHashes[receiptHash]) revert DuplicateReceiptHash();
        _receiptHashes[receiptHash] = true;
        emit ReceiptHashMarked(receiptHash, msg.sender, block.timestamp);
    }

    function isWorkflowRunFinalized(bytes32 workflowRunId) external view returns (bool) {
        return _workflowRuns[workflowRunId];
    }

    function isReceiptHashUsed(bytes32 receiptHash) external view returns (bool) {
        return _receiptHashes[receiptHash];
    }
}
