// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {IVerifier} from "../interfaces/IVerifier.sol";
import {IAccessController} from "../interfaces/IAccessController.sol";
import {IGroth16SettlementVerifier} from "../interfaces/IGroth16SettlementVerifier.sol";

contract Verifier is IVerifier {
    IAccessController public accessController;
    bytes32 private _verifierId;
    address public groth16Verifier;
    mapping(bytes32 => bool) public approvedProofDigests;

    constructor(address accessController_, bytes32 verifierId_) {
        accessController = IAccessController(accessController_);
        _verifierId = verifierId_;
    }

    modifier onlyVerifierAdmin() {
        _onlyVerifierAdmin();
        _;
    }

    function _onlyVerifierAdmin() internal view {
        if (!accessController.isVerifierAdmin(msg.sender)) revert InvalidContextBinding();
    }

    function verifierId() external view returns (bytes32) {
        return _verifierId;
    }

    function setVerifierId(bytes32 verifierId_) external onlyVerifierAdmin {
        _verifierId = verifierId_;
    }

    function setApprovedProofDigest(bytes32 digest, bool allowed) external onlyVerifierAdmin {
        approvedProofDigests[digest] = allowed;
    }

    function setGroth16Verifier(address groth16Verifier_) external onlyVerifierAdmin {
        groth16Verifier = groth16Verifier_;
    }

    function verifyProof(bytes calldata proof, uint256[] calldata publicSignals) public view returns (bool isValid) {
        if (proof.length == 0) return false;
        if (publicSignals.length == 0) return false;

        if (groth16Verifier != address(0)) {
            if (proof.length != 256) return false;
            if (publicSignals.length != 6) return false;

            (uint256[2] memory pA, uint256[2][2] memory pB, uint256[2] memory pC) =
                abi.decode(proof, (uint256[2], uint256[2][2], uint256[2]));
            uint256[6] memory pubSignals;
            for (uint256 i = 0; i < 6; i++) {
                pubSignals[i] = publicSignals[i];
            }

            return IGroth16SettlementVerifier(groth16Verifier).verifyProof(pA, pB, pC, pubSignals);
        }

        bytes32 digest = _hashProofEnvelope(proof, publicSignals);
        return approvedProofDigests[digest];
    }

    function verifyProofWithContext(
        VerifierInput calldata input,
        VerifierContext calldata context
    ) external view returns (bool isValid) {
        if (context.statementHash == bytes32(0)) return false;
        if (context.policyVersion == 0) return false;
        if (context.domainSeparator == bytes32(0)) return false;

        bytes32 digest = _hashProofEnvelope(input.proof, input.publicSignals);
        if (digest != context.statementHash) return false;

        return verifyProof(input.proof, input.publicSignals);
    }

    function _hashProofEnvelope(bytes calldata proof, uint256[] calldata publicSignals) internal pure returns (bytes32 digest) {
        bytes memory encoded = abi.encode(proof, publicSignals);
        assembly {
            digest := keccak256(add(encoded, 0x20), mload(encoded))
        }
    }
}
