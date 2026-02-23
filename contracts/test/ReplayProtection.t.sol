// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {ReplayProtection} from "../src/contracts/ReplayProtection.sol";
import {IReplayProtection} from "../src/interfaces/IReplayProtection.sol";

contract ReplayProtectionTest is Test {
    ReplayProtection internal replayProtection;
    address internal admin = address(this);
    address internal alice = address(0xA11CE);
    bytes32 internal runId = keccak256("run-1");
    bytes32 internal receiptHash = keccak256("receipt-1");

    function setUp() public {
        replayProtection = new ReplayProtection(admin);
    }

    function test_MarkRunAndReceiptHash() public {
        replayProtection.markWorkflowRunFinalized(runId);
        replayProtection.markReceiptHashUsed(receiptHash);

        assertTrue(replayProtection.isWorkflowRunFinalized(runId));
        assertTrue(replayProtection.isReceiptHashUsed(receiptHash));
    }

    function test_RevertWhen_UnauthorizedCallerMarks() public {
        vm.prank(alice);
        vm.expectRevert(IReplayProtection.Unauthorized.selector);
        replayProtection.markWorkflowRunFinalized(runId);
    }

    function test_RevertWhen_DuplicateRun() public {
        replayProtection.markWorkflowRunFinalized(runId);
        vm.expectRevert(IReplayProtection.DuplicateWorkflowRun.selector);
        replayProtection.markWorkflowRunFinalized(runId);
    }

    function test_RevertWhen_DuplicateReceiptHash() public {
        replayProtection.markReceiptHashUsed(receiptHash);
        vm.expectRevert(IReplayProtection.DuplicateReceiptHash.selector);
        replayProtection.markReceiptHashUsed(receiptHash);
    }
}
