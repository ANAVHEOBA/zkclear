# ZK-Clear Contracts (Sepolia)

This folder contains the smart contracts for:
- `AccessController`
- `PolicyManager`
- `Verifier`
- `SignalBinding`
- `ReplayProtection`
- `SettlementRegistry`

## Network Target

- Chain: `Ethereum Sepolia`
- Chain ID: `11155111`

## Environment Variables

Use `.env` in this folder. Start from `.env.example`.

Required:
- `PRIVATE_KEY`
- `ETH_SEPOLIA_RPC_URL`

Recommended:
- `DEPLOYER_ADDRESS`
- `ETH_SEPOLIA_CHAIN_ID=11155111`
- `WORKFLOW_PUBLISHER`
- `VERIFIER_ID`
- `DOMAIN_SEPARATOR`
- `INITIAL_POLICY_VERSION`
- `INITIAL_POLICY_HASH`
- `INITIAL_POLICY_METADATA_HASH`

### Rename From Old Lisk Naming

If you previously used:
- `LISK_TESTNET_RPC_URL`
- `LISK_TESTNET_CHAIN_ID`

Switch to:
- `ETH_SEPOLIA_RPC_URL`
- `ETH_SEPOLIA_CHAIN_ID=11155111`

Example:
```bash
ETH_SEPOLIA_CHAIN_ID=11155111
ETH_SEPOLIA_RPC_URL=https://sepolia.infura.io/v3/<KEY>
```

`https://rpc.sepolia-api.lisk.com` is for Lisk Sepolia, not Ethereum Sepolia.

## Build and Test

```bash
forge build
forge test
```

## Deploy All Contracts

Script:
- `script/DeployZKClear.s.sol:DeployZKClearScript`

Dry run:
```bash
source .env
forge script script/DeployZKClear.s.sol:DeployZKClearScript --rpc-url $ETH_SEPOLIA_RPC_URL
```

Broadcast to Sepolia:
```bash
source .env
forge script script/DeployZKClear.s.sol:DeployZKClearScript \
  --rpc-url $ETH_SEPOLIA_RPC_URL \
  --broadcast \
  --verify
```

If you do not want verification in the same command, remove `--verify`.

## What the Deploy Script Does

1. Deploys `AccessController`
2. Deploys `PolicyManager`
3. Deploys `Verifier`
4. Deploys `SignalBinding`
5. Deploys `ReplayProtection`
6. Deploys `SettlementRegistry`
7. Grants workflow publisher role
8. Authorizes `SettlementRegistry` in `ReplayProtection`
9. Commits and activates initial policy

## Post-Deploy Checklist

1. Save deployed addresses and tx hashes
2. Confirm policy is active:
```bash
cast call <POLICY_MANAGER> "activePolicyVersion()(uint64)" --rpc-url $ETH_SEPOLIA_RPC_URL
```
3. Confirm workflow publisher:
```bash
cast call <ACCESS_CONTROLLER> "isWorkflowPublisher(address)(bool)" <WORKFLOW_PUBLISHER> --rpc-url $ETH_SEPOLIA_RPC_URL
```
4. Confirm replay authorization:
```bash
cast call <REPLAY_PROTECTION> "authorizedCallers(address)(bool)" <SETTLEMENT_REGISTRY> --rpc-url $ETH_SEPOLIA_RPC_URL
```

## Deployed Addresses

- Network: `Ethereum Sepolia (11155111)`
- `AccessController`: `0x67f9aa6f37fc36482c9a0b5f65e1ee28e3ce4409`
- `PolicyManager`: `0x49728d5c119c0497c2478cd54c63097ed47ce9e1`
- `Verifier`: `0xe866e60522ba58da0f65956d417402bc35a5d04b`
- `SettlementValidGroth16Verifier`: `0x91d60b0e89874c8371290443fb4967ff1ff23d55`
- `SignalBinding`: `0x753eaac5674e92631161a3b66b38f9cee2432d2a`
- `ReplayProtection`: `0x0d9c1384a207c2b8c8ef9a5b9cccf5eca7a82737`
- `SettlementRegistry`: `0x3e3a14f46d13e156daa99bf234224a57b1c79da5`

Deployment tx hashes:
- `AccessController`: `0x2d4da8f3cde9fbd8685f94d591275b582d7d2e6d7d15ca07ce6c69971f6d78a5`
- `PolicyManager`: `0x6c0ae6369c88190889404ec33245e875ff33272a8eb1419a0ddf0fad11153745`
- `Verifier`: `0x333aaac39359e2e6af752def7d0afefb32ad2a01c4020eb5e9ac9c930942af41`
- `SettlementValidGroth16Verifier`: `0x11a1244c1b9d7c89b03963de795f5d3af2f6f77c99117f81e291354d3743feef`
- `SignalBinding`: `0xb69b996cc0f265bab3409fb4184091eb54dacebd63e20286187e9fc3de5f0221`
- `ReplayProtection`: `0x19d694130af8e4ee8fcf5ec236f0ae93be6b2366a5e153563df186cdf8cbd95e`
- `SettlementRegistry`: `0x0eb43924edb1202171caee62756be6c1100467c84f2586846f32d1e63f804fda`
- `Verifier.setGroth16Verifier`: `0x88c5477bd0e8a32c9426b7992a0c6cededfa2a2ceaea39158accb7e22c59b768`
- `AccessController.setWorkflowPublisher`: `0x363885f50c5dac72e2171af4b2581c0b7b2e9ea60d8a6775e1170ea89fa372b3`
- `ReplayProtection.setAuthorizedCaller`: `0xa55d04e476af8bf9d3df66d2c12adec008330cdad57877c0b36a13b03293241d`
- `PolicyManager.commitPolicy`: `0x4bf597234ee9d69fd7ca7219f2c8b815dbdb88070b49b0585a9d46737844f747`
- `PolicyManager.activatePolicy`: `0x4823f2bc5215120ad0d53c6187eb1a7f1d77eb26da0302a4ab64882bf9e9c980`

Broadcast artifact:
- `broadcast/DeployZKClear.s.sol/11155111/run-latest.json`

## Security Note

Rotate any private key that has been shared in plain text and fund a fresh deployer key before main deployment.
