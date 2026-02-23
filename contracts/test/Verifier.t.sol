// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {AccessController} from "../src/contracts/AccessController.sol";
import {Verifier} from "../src/contracts/Verifier.sol";
import {IVerifier} from "../src/interfaces/IVerifier.sol";

contract VerifierTest is Test {
    AccessController internal accessController;
    Verifier internal verifier;
    address internal admin = address(this);
    address internal alice = address(0xA11CE);

    function setUp() public {
        accessController = new AccessController(admin);
        verifier = new Verifier(address(accessController), keccak256("verifier-v1"));
    }

    function test_VerifyProofTrueWhenDigestApproved() public {
        bytes memory proof = hex"1234";
        uint256[] memory signals = new uint256[](2);
        signals[0] = 111;
        signals[1] = 222;

        bytes32 digest = keccak256(abi.encode(proof, signals));
        verifier.setApprovedProofDigest(digest, true);

        assertTrue(verifier.verifyProof(proof, signals));
    }

    function test_VerifyProofWithContextTrue() public {
        bytes memory proof = hex"aabbcc";
        uint256[] memory signals = new uint256[](1);
        signals[0] = 777;

        bytes32 digest = keccak256(abi.encode(proof, signals));
        verifier.setApprovedProofDigest(digest, true);

        IVerifier.VerifierInput memory input = IVerifier.VerifierInput({proof: proof, publicSignals: signals});
        IVerifier.VerifierContext memory context =
            IVerifier.VerifierContext({statementHash: digest, policyVersion: 1, domainSeparator: keccak256("domain")});

        assertTrue(verifier.verifyProofWithContext(input, context));
    }

    function test_VerifyProofFalseForEmptyInputs() public view {
        bytes memory proof;
        uint256[] memory signals = new uint256[](0);
        assertFalse(verifier.verifyProof(proof, signals));
    }

    function test_RevertWhen_NonVerifierAdminUpdatesConfig() public {
        vm.prank(alice);
        vm.expectRevert(IVerifier.InvalidContextBinding.selector);
        verifier.setVerifierId(bytes32(uint256(9)));
    }
}
