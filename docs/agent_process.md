# Mālama Protocol: Antigravity Agent Process & Go-Live Recommendation

> **Using Antigravity to autonomously build, test, and deploy the Mālama Protocol end-to-end.**

---

## WHY ANTIGRAVITY FOR MĀLAMA

Antigravity is purpose-built for exactly this type of work:

- **Agentic execution** — runs long multi-step tasks without constant supervision
- **Full codebase context** — reads, writes, and coordinates across the entire `Malama SDK` workspace
- **Parallel tooling** — runs tests, commits code, reads browser output, and verifies results simultaneously
- **Persistent memory** — picks up where the last session ended via Knowledge Items and conversation artifacts
- **Browser verification** — can visually confirm the dashboard is working after each deployment

Each of the 56 Odyssey prompts maps directly to an Antigravity task boundary, making the entire build traceable and resumable.

---

## RECOMMENDED AGENT ARCHITECTURE

### Agent 1 — Core Protocol Builder (You Are Here)

**Scope:** Stages 1–5 (Rust SDK, Merkle trees, Chain adapters, IPFS, Integrity)
**Tool:** Antigravity in this workspace (`/Users/tylermalin/Malama SDK`)
**Current Status:** ✅ Stages 1–2 complete (DID, Gateway, Settlement skeleton)

### Agent 2 — Smart Contract Deployer

**Scope:** Stage 4 contracts (Cardano Plutus, BASE Solidity, HEDERA HCS/HTS, CELO)
**Separate workspace per chain** (prevents cross-contamination of chain-specific toolchains)
**Trigger:** After Stage 4 adapter interfaces pass all unit tests

### Agent 3 — AI Validation Engine

**Scope:** Stage 5 (LSH engine, XGBoost confidence model, SHAP explainability)
**Runs in:** Python environment with GPU access
**Output:** ONNX model file → loaded by Rust inference wrapper in Agent 1's workspace

### Agent 4 — Dashboard & Registry Bridge

**Scope:** Stage 7 dashboard (the existing `/dashboard` React app) + Verra/Gold Standard API adapters
**Trigger:** After Stage 6 settlement engine is live on testnet
**This agent already has a head start** — the Vite dashboard is scaffolded and verified

---

## GO-LIVE PROCESS: STAGE BY STAGE

### Phase 1 — Testnet Foundation (Weeks 1–6)

**Antigravity Tasks:**

```text
Task 1: Complete Stage 1 hardening
  - Run: cargo test --lib stage1_birth_identity
  - Fix any failing tests
  - Commit: "feat: stage1 hardened DID + provenance system"

Task 2: Complete Stage 2 batching
  - Implement full Merkle tree with LSH compression
  - Kafka topic schema (Avro + Schema Registry)
  - Commit: "feat: stage2 batching engine + kafka topics"

Task 3: Deploy testnet smart contracts
  - Cardano testnet via Blockfrost
  - BASE Goerli testnet via Alchemy
  - HEDERA testnet via HashScan
  - CELO Alfajores testnet
  - Commit: "deploy: all 4 chains on testnet"

Task 4: Wire Stage 6 settlement
  - LCO2/VCO2 token minting (confidence > 80%)
  - 2-of-3 validator quorum
  - Run integration test: full sensor → token mint flow
  - Commit: "feat: settlement engine integrated"
```

**Go / No-Go Criteria:**

- [ ] All 16+ unit tests pass (`cargo test`)
- [ ] Merkle root confirmed on all 4 testnets
- [ ] LCO₂ minted successfully on Cardano testnet
- [ ] Dashboard shows live sensor data

---

### Phase 2 — Pilot Deployment (Weeks 7–12)

**Antigravity Tasks:**

```text
Task 5: Real sensor onboarding
  - Use the dashboard to onboard 3 real sensors (Idaho biochar farm)
  - Verify: DID generated, NFT minted on Cardano testnet
  - Record proof-of-handshake video

Task 6: Live data pipeline validation
  - Run for 7 days with real sensors
  - Monitor Kafka lag < 1 min
  - Verify Merkle roots submitted every hour
  - Confirm IPFS CIDs pinned on Pinata

Task 7: AI model training
  - Train XGBoost on 7 days of real sensor data
  - Export ONNX model
  - Deploy inference endpoint
  - Verify confidence scores > 80% on clean data

Task 8: Verra API integration
  - Submit first "Proof of Journey" with pilot data
  - Monitor for approval status
  - Document any rejection reasons and fix
```

**Go / No-Go Criteria:**

- [ ] 7 days of uninterrupted real sensor data
- [ ] AI confidence consistently > 82% on clean readings
- [ ] Verra accepts evidence bundle (or provides actionable feedback)
- [ ] Zero tampering false-positives on clean data

---

### Phase 3 — Mainnet Launch (Weeks 13–20)

**Antigravity Tasks:**

```text
Task 9: Security audit prep
  - Run: cargo audit (dependency vulnerabilities)
  - Penetration test: replay attack, GPS spoofing, validator collusion simulation
  - Fix all CRITICAL/HIGH findings before mainnet

Task 10: Mainnet contract deployment
  - Cardano mainnet (via Blockfrost mainnet API key)
  - BASE mainnet (via Alchemy mainnet)
  - HEDERA mainnet (production account)
  - CELO mainnet
  - Verify all contracts on respective block explorers

Task 11: Production infrastructure
  - Deploy Kafka cluster (AWS MSK or self-hosted)
  - Deploy Rust gateway (Docker on AWS ECS or Fly.io)
  - Deploy dashboard (Vercel or Netlify)
  - Set up monitoring (Datadog or Grafana Cloud)
  - Configure PagerDuty alerts for CRITICAL tampering events

Task 12: Farmer onboarding launch
  - QR code flow: scan → DID generated → sensor registered
  - Onboard first paying farm project
  - First LCO₂ minted on mainnet
```

**Go / No-Go Criteria:**

- [ ] Security audit clean (no CRITICAL/HIGH findings)
- [ ] All contracts verified on block explorers
- [ ] Infrastructure uptime > 99.5% for 2 weeks
- [ ] First real LCO₂ minted on mainnet
- [ ] First "Proof of Journey" submitted to Verra on mainnet

---

## ANTIGRAVITY WORKFLOW SETUP

### 1. Create Workflow Files

Save these as agent workflows so you can invoke them with slash commands:

**`.agent/workflows/build-stage.md`**

```markdown
---
description: Build and test a single Odyssey stage
---
1. Run cargo test for the specified stage module
2. Fix any failing tests
3. Run cargo clippy to catch warnings
4. Commit changes with a conventional commit message
5. Push to GitHub
```

**`.agent/workflows/deploy-contracts.md`**

```markdown
---
description: Deploy smart contracts to testnet or mainnet
---
1. Confirm network (testnet | mainnet) before proceeding
2. Run contract tests (unit + integration)
3. Deploy to specified chain using the chain's CLI/SDK
4. Verify contract on block explorer
5. Update .env with deployed contract addresses
6. Commit updated addresses
```

**`.agent/workflows/verify-pipeline.md`**

```markdown
---
description: End-to-end pipeline verification
---
1. Start a sensor simulation (generate 100 mock readings)
2. Confirm readings appear in Kafka raw-sensor-readings topic
3. Confirm batch created with Merkle root
4. Confirm Merkle root submitted to blockchain (all 4 chains)
5. Confirm IPFS CID is accessible
6. Confirm LCO₂ minted if confidence > 80%
7. Open dashboard and visually confirm sensor status is ACTIVE
8. Report: pass/fail with links to tx hashes
```

### 2. Recommended Session Structure

Each Antigravity session should target **one complete Odyssey stage**:

```
Session 1: "Complete Stage 1 — DID + Provenance + Signing + Reputation"
Session 2: "Complete Stage 2 — Batching + Merkle Trees + Kafka"
Session 3: "Complete Stage 3 — Multi-Validator Quorum + Consensus"
Session 4: "Deploy Stage 4 contracts to all 4 testnets"
Session 5: "Complete Stage 5 — LSH + Immutability + GDPR"
Session 6: "Complete Stage 6 — LCO₂/VCO₂ Settlement Engine"
Session 7: "Complete Stage 7 — E2E Verification API + Dashboard"
```

Each session ends with tests passing, committed, and pushed. Antigravity's Knowledge Items preserve context between sessions.

---

## MONITORING & OBSERVABILITY STACK

| Layer | Tool | What to Watch |
|-------|------|---------------|
| Sensors | Custom SDK metrics | Battery voltage, signal strength, uptime % |
| Pipeline | Kafka lag dashboard | Consumer lag < 60s per topic |
| Blockchain | Chain Explorer APIs | Tx confirmation time, gas costs |
| AI Model | MLflow or W&B | Confidence score distribution, drift |
| Application | Datadog APM | API latency, error rate, throughput |
| Alerts | PagerDuty | CRITICAL: tampering, validator offline |

---

## IMMEDIATE NEXT STEPS (TODAY)

1. **Tell Antigravity:** *"Complete Stage 2 — implement the full Merkle tree batching engine in `src/stage2_gateway/`"*
2. **Tell Antigravity:** *"Deploy Cardano testnet contracts — use the Blockfrost testnet API and Plutus contracts from `docs/prompts.md` Stage 4, Prompt 27"*
3. **Tell Antigravity:** *"Wire the AI confidence scoring to the settlement engine — confidence > 80% triggers LCO₂ minting in `src/stage6_settlement/`"*
4. **Create workflows** by asking Antigravity to write the 3 workflow files above into `.agent/workflows/`

---

*Built with Antigravity | Mālama Protocol | March 2026*
