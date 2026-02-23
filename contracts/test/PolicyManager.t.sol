// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {AccessController} from "../src/contracts/AccessController.sol";
import {PolicyManager} from "../src/contracts/PolicyManager.sol";
import {IPolicyManager} from "../src/interfaces/IPolicyManager.sol";

contract PolicyManagerTest is Test {
    AccessController internal accessController;
    PolicyManager internal policyManager;
    address internal admin = address(this);
    address internal alice = address(0xA11CE);

    function setUp() public {
        accessController = new AccessController(admin);
        policyManager = new PolicyManager(address(accessController));
    }

    function test_CommitAndActivatePolicy() public {
        bytes32 policyHash = keccak256("policy-v1");
        bytes32 metadataHash = keccak256("meta-v1");

        policyManager.commitPolicy(1, policyHash, metadataHash);
        policyManager.activatePolicy(1);

        (uint64 version, bytes32 activePolicyHash, bytes32 activeMetadataHash) = policyManager.getActivePolicy();
        assertEq(version, 1);
        assertEq(activePolicyHash, policyHash);
        assertEq(activeMetadataHash, metadataHash);
        assertTrue(policyManager.isPolicyActive(1));
    }

    function test_ActivateSwitchesPreviousPolicyOff() public {
        policyManager.commitPolicy(1, keccak256("p1"), keccak256("m1"));
        policyManager.commitPolicy(2, keccak256("p2"), keccak256("m2"));
        policyManager.activatePolicy(1);
        policyManager.activatePolicy(2);

        assertFalse(policyManager.isPolicyActive(1));
        assertTrue(policyManager.isPolicyActive(2));
        assertEq(policyManager.activePolicyVersion(), 2);
    }

    function test_RevertWhen_NonPolicyAdminCommits() public {
        vm.prank(alice);
        vm.expectRevert(IPolicyManager.Unauthorized.selector);
        policyManager.commitPolicy(1, keccak256("p"), keccak256("m"));
    }

    function test_RevertWhen_InvalidPolicyVersion() public {
        vm.expectRevert(IPolicyManager.InvalidPolicyVersion.selector);
        policyManager.commitPolicy(0, keccak256("p"), keccak256("m"));
    }

    function test_RevertWhen_GetUnknownPolicy() public {
        vm.expectRevert(IPolicyManager.PolicyNotFound.selector);
        policyManager.getPolicy(999);
    }
}
