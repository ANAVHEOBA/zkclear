// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

interface IVerifier {
    error InvalidProof();
    error InvalidPublicSignals();
    error InvalidContextBinding();

    /// @notice Canonical verifier input for proof validation.
    struct VerifierInput {
        bytes proof;
        uint256[] publicSignals;
    }

    /// @notice Optional context binding to prevent proof replay across domains/policies.
    struct VerifierContext {
        bytes32 statementHash;
        uint64 policyVersion;
        bytes32 domainSeparator;
    }

    /// @notice Stable identifier for the active verifier/circuit configuration.
    function verifierId() external view returns (bytes32);

    /// @notice Verifies Groth16 proof and public signals.
    function verifyProof(bytes calldata proof, uint256[] calldata publicSignals) external view returns (bool isValid);

    /// @notice Verifies proof while binding it to policy/domain context.
    function verifyProofWithContext(
        VerifierInput calldata input,
        VerifierContext calldata context
    ) external view returns (bool isValid);
}
