// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {ISignalBinding} from "../interfaces/ISignalBinding.sol";
import {IAccessController} from "../interfaces/IAccessController.sol";

contract SignalBinding is ISignalBinding {
    IAccessController public accessController;
    bytes32 private _domainSeparator;

    constructor(address accessController_, bytes32 initialDomainSeparator) {
        accessController = IAccessController(accessController_);
        _domainSeparator = initialDomainSeparator;
    }

    function validateSignalBinding(
        uint256[] calldata publicSignals,
        BindingContext calldata context
    ) external view returns (bool) {
        if (context.domainSeparator != _domainSeparator) return false;
        if (context.policyVersion == 0) return false;
        if (context.workflowRunId == bytes32(0)) return false;
        if (context.receiptHash == bytes32(0)) return false;
        if (publicSignals.length < 6) return false;

        uint256 runProjection = _fieldProjection(context.workflowRunId);
        uint256 receiptProjection = _fieldProjection(context.receiptHash);
        uint256 domainProjection = _fieldProjection(context.domainSeparator);

        bytes32 bindingHash = computeBindingHash(context);
        if (publicSignals[0] != uint256(context.policyVersion)) return false;
        if (publicSignals[1] != receiptProjection) return false;
        if (publicSignals[2] != domainProjection) return false;
        if (publicSignals[3] != runProjection) return false;
        if (publicSignals[4] != uint256(bindingHash)) return false;

        return true;
    }

    function computeBindingHash(BindingContext calldata context) public pure returns (bytes32) {
        uint256 runProjection = _fieldProjection(context.workflowRunId);
        uint256 receiptProjection = _fieldProjection(context.receiptHash);
        uint256 domainProjection = _fieldProjection(context.domainSeparator);
        uint256 binding = runProjection * 23 + uint256(context.policyVersion) * 131 + receiptProjection * 17
            + domainProjection * 19;
        return bytes32(binding);
    }

    function setDomainSeparator(bytes32 newDomainSeparator) external {
        if (!accessController.isVerifierAdmin(msg.sender)) revert InvalidDomainSeparator();
        if (newDomainSeparator == bytes32(0)) revert InvalidDomainSeparator();
        _domainSeparator = newDomainSeparator;
        emit DomainSeparatorUpdated(newDomainSeparator);
    }

    function domainSeparator() external view returns (bytes32) {
        return _domainSeparator;
    }

    function _fieldProjection(bytes32 value) internal pure returns (uint256) {
        return uint256(uint64(uint256(value)));
    }
}
