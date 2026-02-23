// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {IPolicyManager} from "../interfaces/IPolicyManager.sol";
import {IAccessController} from "../interfaces/IAccessController.sol";

contract PolicyManager is IPolicyManager {
    IAccessController public accessController;
    uint64 private _activePolicyVersion;
    mapping(uint64 => Policy) private _policies;

    constructor(address accessController_) {
        accessController = IAccessController(accessController_);
    }

    modifier onlyPolicyAdmin() {
        _onlyPolicyAdmin();
        _;
    }

    function _onlyPolicyAdmin() internal view {
        if (!accessController.isPolicyAdmin(msg.sender)) revert Unauthorized();
    }

    function commitPolicy(uint64 version, bytes32 policyHash, bytes32 metadataHash) external onlyPolicyAdmin {
        if (version == 0 || policyHash == bytes32(0)) revert InvalidPolicyVersion();
        if (_policies[version].exists) revert PolicyAlreadyExists();
        _policies[version] = Policy({policyHash: policyHash, metadataHash: metadataHash, exists: true, active: false});
        emit PolicyCommitted(version, policyHash, metadataHash);
    }

    function activatePolicy(uint64 version) external onlyPolicyAdmin {
        Policy storage p = _policies[version];
        if (!p.exists) revert PolicyNotFound();
        if (p.active) revert PolicyAlreadyActive();

        if (_activePolicyVersion != 0) {
            Policy storage oldPolicy = _policies[_activePolicyVersion];
            oldPolicy.active = false;
            emit PolicyDeactivated(_activePolicyVersion);
        }

        p.active = true;
        _activePolicyVersion = version;
        emit PolicyActivated(version);
    }

    function deactivatePolicy(uint64 version) external onlyPolicyAdmin {
        Policy storage p = _policies[version];
        if (!p.exists) revert PolicyNotFound();
        if (!p.active) revert PolicyNotActive();
        p.active = false;
        if (_activePolicyVersion == version) {
            _activePolicyVersion = 0;
        }
        emit PolicyDeactivated(version);
    }

    function getPolicy(uint64 version) external view returns (Policy memory) {
        Policy memory p = _policies[version];
        if (!p.exists) revert PolicyNotFound();
        return p;
    }

    function getActivePolicy() external view returns (uint64 version, bytes32 policyHash, bytes32 metadataHash) {
        version = _activePolicyVersion;
        if (version == 0) return (0, bytes32(0), bytes32(0));
        Policy memory p = _policies[version];
        return (version, p.policyHash, p.metadataHash);
    }

    function isPolicyActive(uint64 version) external view returns (bool) {
        return _policies[version].active;
    }

    function policyHashOf(uint64 version) external view returns (bytes32) {
        Policy memory p = _policies[version];
        if (!p.exists) revert PolicyNotFound();
        return p.policyHash;
    }

    function activePolicyVersion() external view returns (uint64) {
        return _activePolicyVersion;
    }
}
