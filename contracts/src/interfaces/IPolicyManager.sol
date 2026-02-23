// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

interface IPolicyManager {
    error Unauthorized();
    error InvalidPolicyVersion();
    error PolicyAlreadyExists();
    error PolicyNotFound();
    error PolicyAlreadyActive();
    error PolicyNotActive();

    event PolicyCommitted(uint64 indexed version, bytes32 indexed policyHash, bytes32 metadataHash);
    event PolicyActivated(uint64 indexed version);
    event PolicyDeactivated(uint64 indexed version);

    struct Policy {
        bytes32 policyHash;
        bytes32 metadataHash;
        bool exists;
        bool active;
    }

    function commitPolicy(uint64 version, bytes32 policyHash, bytes32 metadataHash) external;
    function activatePolicy(uint64 version) external;
    function deactivatePolicy(uint64 version) external;
    function getPolicy(uint64 version) external view returns (Policy memory);
    function getActivePolicy() external view returns (uint64 version, bytes32 policyHash, bytes32 metadataHash);
    function isPolicyActive(uint64 version) external view returns (bool);
    function policyHashOf(uint64 version) external view returns (bytes32);
    function activePolicyVersion() external view returns (uint64);
}
