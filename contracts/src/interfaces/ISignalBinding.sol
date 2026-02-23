// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

interface ISignalBinding {
    error InvalidSignalBinding();
    error InvalidDomainSeparator();
    error InvalidPolicyVersion();
    error InvalidReceiptHash();

    event DomainSeparatorUpdated(bytes32 indexed domainSeparator);
    event SignalBindingValidated(
        bytes32 indexed workflowRunId,
        uint64 indexed policyVersion,
        bytes32 indexed receiptHash,
        bytes32 bindingHash
    );

    struct BindingContext {
        bytes32 workflowRunId;
        uint64 policyVersion;
        bytes32 receiptHash;
        bytes32 domainSeparator;
    }

    function validateSignalBinding(
        uint256[] calldata publicSignals,
        BindingContext calldata context
    ) external view returns (bool);

    function computeBindingHash(BindingContext calldata context) external pure returns (bytes32);
    function setDomainSeparator(bytes32 newDomainSeparator) external;
    function domainSeparator() external view returns (bytes32);
}
