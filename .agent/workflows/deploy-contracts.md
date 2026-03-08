---
description: deploy smart contracts to testnet or mainnet for any supported chain
---

// turbo-all

1. Confirm with user: which chain (cardano | base | hedera | celo) and which network (testnet | mainnet)?
2. Run contract unit tests for the specified chain.
3. Check that the relevant RPC keys and wallet keys are set in `.env` — do NOT proceed if missing.
4. Deploy the contracts using the appropriate tool:
   - Cardano: `cardano-cli` or Blockfrost + Plutus build output
   - BASE/CELO: `npx hardhat deploy --network {network}`
   - HEDERA: Hedera SDK deploy script
5. Confirm the contract is live on the chain's block explorer.
6. Update `docs/deployed_contracts.md` with: chain, network, address, tx hash, timestamp.
7. Commit: `deploy({chain}): contracts deployed to {network}`
8. Push to GitHub.
9. Report: contract addresses, explorer links, and gas cost.
