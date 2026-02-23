// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Script} from "forge-std/Script.sol";
import {AccessController} from "../src/contracts/AccessController.sol";
import {PolicyManager} from "../src/contracts/PolicyManager.sol";
import {Verifier} from "../src/contracts/Verifier.sol";
import {SignalBinding} from "../src/contracts/SignalBinding.sol";
import {ReplayProtection} from "../src/contracts/ReplayProtection.sol";
import {SettlementRegistry} from "../src/contracts/SettlementRegistry.sol";
import {SettlementValidGroth16Verifier} from "../src/contracts/generated/SettlementValidGroth16Verifier.sol";

contract DeployZKClearScript is Script {
    function run() external {
        uint256 deployerKey = vm.envUint("PRIVATE_KEY");
        address deployer = vm.addr(deployerKey);

        address workflowPublisher = vm.envOr("WORKFLOW_PUBLISHER", deployer);
        bytes32 verifierId = vm.envOr("VERIFIER_ID", keccak256("zkclear-verifier-v1"));
        bytes32 initialDomainSeparator = vm.envOr("DOMAIN_SEPARATOR", keccak256("zkclear-sepolia-domain-v1"));
        uint64 initialPolicyVersion = uint64(vm.envOr("INITIAL_POLICY_VERSION", uint256(1)));
        bytes32 initialPolicyHash = vm.envOr("INITIAL_POLICY_HASH", keccak256("policy-v1"));
        bytes32 initialMetadataHash = vm.envOr("INITIAL_POLICY_METADATA_HASH", keccak256("policy-metadata-v1"));

        vm.startBroadcast(deployerKey);

        AccessController accessController = new AccessController(deployer);
        PolicyManager policyManager = new PolicyManager(address(accessController));
        Verifier verifier = new Verifier(address(accessController), verifierId);
        SettlementValidGroth16Verifier groth16Verifier = new SettlementValidGroth16Verifier();
        SignalBinding signalBinding = new SignalBinding(address(accessController), initialDomainSeparator);
        ReplayProtection replayProtection = new ReplayProtection(deployer);
        SettlementRegistry settlementRegistry = new SettlementRegistry(
            address(policyManager),
            address(verifier),
            address(signalBinding),
            address(replayProtection),
            address(accessController)
        );

        verifier.setGroth16Verifier(address(groth16Verifier));
        accessController.setWorkflowPublisher(workflowPublisher, true);
        replayProtection.setAuthorizedCaller(address(settlementRegistry), true);

        policyManager.commitPolicy(initialPolicyVersion, initialPolicyHash, initialMetadataHash);
        policyManager.activatePolicy(initialPolicyVersion);

        vm.stopBroadcast();
    }
}
