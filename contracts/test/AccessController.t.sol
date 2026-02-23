// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {AccessController} from "../src/contracts/AccessController.sol";
import {IAccessController} from "../src/interfaces/IAccessController.sol";

contract AccessControllerTest is Test {
    AccessController internal accessController;
    address internal admin = address(this);
    address internal alice = address(0xA11CE);
    address internal bob = address(0xB0B);

    function setUp() public {
        accessController = new AccessController(admin);
    }

    function test_InitialRoles() public view {
        assertTrue(accessController.isPauser(admin));
        assertTrue(accessController.isPolicyAdmin(admin));
        assertTrue(accessController.isVerifierAdmin(admin));
        assertFalse(accessController.isWorkflowPublisher(admin));
        assertFalse(accessController.paused());
    }

    function test_SetWorkflowPublisher() public {
        accessController.setWorkflowPublisher(alice, true);
        assertTrue(accessController.isWorkflowPublisher(alice));
    }

    function test_RevertWhen_NonAdminSetsRole() public {
        vm.prank(alice);
        vm.expectRevert(IAccessController.Unauthorized.selector);
        accessController.setPolicyAdmin(bob, true);
    }

    function test_SetPausedByPauser() public {
        accessController.setPauser(alice, true);
        vm.prank(alice);
        accessController.setPaused(true);
        assertTrue(accessController.paused());
    }

    function test_RevertWhen_SetPausedAlreadySet() public {
        accessController.setPaused(true);
        vm.expectRevert(IAccessController.AlreadySet.selector);
        accessController.setPaused(true);
    }
}
