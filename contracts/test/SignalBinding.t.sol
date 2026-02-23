// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {AccessController} from "../src/contracts/AccessController.sol";
import {SignalBinding} from "../src/contracts/SignalBinding.sol";
import {ISignalBinding} from "../src/interfaces/ISignalBinding.sol";

contract SignalBindingTest is Test {
    AccessController internal accessController;
    SignalBinding internal signalBinding;
    address internal admin = address(this);
    address internal alice = address(0xA11CE);
    bytes32 internal initialDomain = keccak256("domain-v1");

    function setUp() public {
        accessController = new AccessController(admin);
        signalBinding = new SignalBinding(address(accessController), initialDomain);
    }

    function test_ValidateSignalBindingTrue() public view {
        ISignalBinding.BindingContext memory context = ISignalBinding.BindingContext({
            workflowRunId: keccak256("run"),
            policyVersion: 1,
            receiptHash: keccak256("receipt"),
            domainSeparator: initialDomain
        });
        bytes32 bindingHash = signalBinding.computeBindingHash(context);
        uint256[] memory signals = new uint256[](6);
        signals[0] = context.policyVersion;
        signals[1] = uint64(uint256(context.receiptHash));
        signals[2] = uint64(uint256(context.domainSeparator));
        signals[3] = uint64(uint256(context.workflowRunId));
        signals[4] = uint256(bindingHash);
        signals[5] = 1000;

        assertTrue(signalBinding.validateSignalBinding(signals, context));
    }

    function test_ValidateSignalBindingFalseForWrongDomain() public view {
        ISignalBinding.BindingContext memory context = ISignalBinding.BindingContext({
            workflowRunId: keccak256("run"),
            policyVersion: 1,
            receiptHash: keccak256("receipt"),
            domainSeparator: keccak256("wrong")
        });
        uint256[] memory signals = new uint256[](6);
        signals[0] = 1;

        assertFalse(signalBinding.validateSignalBinding(signals, context));
    }

    function test_SetDomainSeparator() public {
        bytes32 newDomain = keccak256("domain-v2");
        signalBinding.setDomainSeparator(newDomain);
        assertEq(signalBinding.domainSeparator(), newDomain);
    }

    function test_RevertWhen_NonVerifierAdminSetsDomainSeparator() public {
        vm.prank(alice);
        vm.expectRevert(ISignalBinding.InvalidDomainSeparator.selector);
        signalBinding.setDomainSeparator(keccak256("domain-v3"));
    }
}
