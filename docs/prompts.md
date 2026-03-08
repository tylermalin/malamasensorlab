# MĀLAMA PROTOCOL: Cursor Build Prompts

## "Odyssey of a Data Point" — Sensor to Multi-Chain Settlement

**Framework:** Each data point travels through 7 stages, mirroring the "Odyssey" narrative  
**Purpose:** Build production-ready Mālama MVP using story-driven prompts  
**Total Prompts:** 56 | **Estimated Build Time:** 20 weeks

---

## NARRATIVE FRAMEWORK: "ODYSSEY OF A DATA POINT"

| | |
|---|---|
| **The Hero** | A single data point (temperature reading, carbon removal, soil moisture) |
| **The Quest** | Birth → Identity → Batching → Consensus → Storage → Verification → Settlement |
| **The Stakes** | *"Trust by cryptography, not authority"* |

---

## MASTER PROMPT

```text
You are Cursor AI, building the Mālama Protocol as a story-driven application.

The narrative: A single data point (like a temperature reading) travels from a physical sensor, 
gets a cryptographic identity (DID), gets batched with others (Merkle tree), finds a home 
on-chain (multi-chain adapter), stays safe off-chain (IPFS + immutability proofs), and 
finally settles on multiple blockchains (Cardano/BASE/HEDERA/CELO).

Follow this sequence of 56 prompts, organized by the 7 stages of the Odyssey:

STAGE 1: BIRTH OF DATA & IDENTITY (Prompts 1–8)
STAGE 2: THE GATEWAY (Prompts 9–16)
STAGE 3: SATELLITE & CONSENSUS (Prompts 17–24)
STAGE 4: STORAGE PROVIDER (Prompts 25–32)
STAGE 5: SAFETY NET & INTEGRITY (Prompts 33–40)
STAGE 6: MULTI-CHAIN SETTLEMENT (Prompts 41–48)
STAGE 7: VERIFICATION & PROOF (Prompts 49–56)

For each prompt:
- Understand the narrative context (which stage of the journey?)
- Implement story-coherent code (function names should reflect the journey)
- Write tests that verify story assumptions (e.g., "data remains tamper-proof")
- Document with narrative language (not just technical)

Success = A data point can travel from sensor → blockchain → verified → settled, 
with a clear "chain of custody" visible at every stage.
```

---

## STAGE 1: BIRTH OF DATA & IDENTITY (Prompts 1–8)

### PROMPT 1: DID Generation & Self-Sovereign Sensor Identity

```text
[STAGE 1: BIRTH OF DATA & IDENTITY]
[THE HERO: A single data point needs a birth certificate]

Your task: Create a Sensor Identity System using Decentralized Identifiers (DIDs).

Narrative Context:
When our temperature sensor "wakes up" for the first time, it doesn't exist in any 
centralized database. Instead, it self-generates a cryptographic identity: a DID like 
"did:cardano:sensor:biochar-001". This is Self-Sovereign Identity—the sensor "owns" 
its identity independent of any authority.

Implementation Requirements:

1. DID Generation Engine (Rust/TypeScript)
   - Generate ECDSA keypair on sensor startup
   - Create W3C-compliant DID document
   - Store DID format: did:{blockchain}:sensor:{unique_identifier}
   - Return: {did, publicKey, privateKey (in secure element), createdAt}

2. DID Document Structure
   {
     "@context": "https://www.w3.org/ns/did/v1",
     "id": "did:cardano:sensor:biochar-001",
     "publicKey": [{
       "id": "did:cardano:sensor:biochar-001#key-1",
       "type": "EcdsaSecp256k1VerificationKey2019",
       "controller": "did:cardano:sensor:biochar-001",
       "publicKeyBase58": "0x82c4a7f2..."
     }],
     "authentication": ["did:cardano:sensor:biochar-001#key-1"],
     "created": "2025-03-05T12:00:00Z",
     "metadata": {
       "sensorType": "temperature",
       "location": {"latitude": 44.3, "longitude": -114.8},
       "manufacturer": "Tropic Square",
       "calibrationDate": "2025-02-01"
     }
   }

3. Self-Sovereign Identity Proof
   - Sensor proves ownership by signing a challenge
   - Challenge: Random nonce + timestamp
   - Signature = ECDSA(nonce || timestamp, privateKey)
   - Verifier recovers publicKey from signature → matches DID

4. Digital Twin Tracking
   - Map physical sensor → virtual DID identity
   - Track lifecycle: UNREGISTERED → REGISTERED → ACTIVE → OFFLINE → QUARANTINED → RETIRED
   - Immutable audit trail of identity state changes

5. Tests
   - Generate 100 DIDs, verify all unique
   - Prove ownership via signature challenge (success)
   - Tamper with signature, ownership fails (security test)
   - Verify DID format conforms to W3C standard

Narrative Success:
"Our data point is born with an unforgeable identity. No one can fake this sensor's 
readings because the sensor itself cryptographically proves its own existence."

Output: DID generation engine + DID document storage + ownership proof system + tests
```

### PROMPT 2: Sensor Birth Metadata & Provenance

```text
[STAGE 1: BIRTH OF DATA & IDENTITY]
[ESTABLISHING PROVENANCE: The sensor records "where it came from"]

Your task: Create a Sensor Provenance System that captures immutable origin metadata.

Narrative Context:
Before our data point takes its first reading, we record the sensor's "birth certificate"—
metadata that can never be changed. This includes: who made the sensor, where, when it was 
activated, its calibration status, physical location, and chain of custody.

Implementation:

1. Provenance Record (Immutable from creation)
   {
     "sensorDID": "did:cardano:sensor:biochar-001",
     "provenance": {
       "manufacturer": "Tropic Square",
       "manufacturingDate": "2024-12-15",
       "serialNumber": "TSQ-20241215-001",
       "initialCalibration": {
         "date": "2024-12-20", "reference": "Calibration Lab, TU Prague",
         "accuracy": "±0.3°C", "trackedTo": "NIST Standard"
       }
     },
     "deployment": {
       "installedAt": "2025-02-01T10:00:00Z",
       "location": {"address": "Idaho City Biochar Farm, Boise County, ID",
                    "coordinates": {"latitude": 43.8, "longitude": -115.9}},
       "deployedBy": "Jeffrey Wise (Mālama COO)"
     },
     "chainOfCustody": [
       {"timestamp": "2024-12-15", "custodian": "Tropic Square", "action": "manufactured"},
       {"timestamp": "2024-12-20", "custodian": "TU Prague Lab", "action": "calibrated"},
       {"timestamp": "2025-01-15", "custodian": "Mālama Labs", "action": "received"},
       {"timestamp": "2025-02-01", "custodian": "Idaho Farm", "action": "installed"}
     ],
     "createdHash": "0x7a4f..."
   }

2. Immutability Guarantee
   - Once created, provenance cannot be modified
   - Timestamp via blockchain (multi-chain anchoring)
   - Defense against: "Manufacturer's sticker replaced" attacks

3. Chain of Custody Tracking
   - Immutable log of who had the sensor and when
   - Each handoff signed by both current + new custodian
   - GPS location recorded at each step

4. Tests
   - Provenance record created, anchored to blockchain
   - Attempt to modify provenance → fails
   - Chain of custody signature verification
   - Location audit: GPS matches claimed location (within 100m)

Narrative Success:
"The sensor's entire history is locked in time. Auditors can verify: 'This sensor was 
manufactured in Prague on Dec 15, 2024, calibrated by TU Prague, installed Feb 1.'"

Output: Provenance recording system + custody signing + immutability proofs + tests
```

### PROMPT 3: Sensor Reading Signing & Cryptographic Data Fingerprint

```text
[STAGE 1: BIRTH OF DATA & IDENTITY]
[THE FIRST MEASUREMENT: The sensor's inaugural signature]

Your task: Create the Sensor Reading Signing System.

Narrative Context:
Our sensor takes its first reading: 23.4°C at 2025-03-05T12:30:00Z. But this number 
is useless without cryptographic proof that it came from the real sensor. The sensor 
signs each reading using its ECDSA private key (stored in the secure element).

Implementation:

1. Reading Data Structure
   {
     "sensorDID": "did:cardano:sensor:biochar-001",
     "reading": 23.4, "unit": "Celsius",
     "timestamp": "2025-03-05T12:30:00Z",
     "sequenceNumber": 1,
     "nonce": "0x82c4a7f2..." (random, prevents replay attacks),
     "location": {"latitude": 43.8123, "longitude": -115.9456, "altitude": 1234},
     "batteryVoltage": 4.2,
     "uncertaintyBounds": {"lower": 23.1, "upper": 23.7, "confidence": 0.95}
   }

2. Signing Process
   - Serialize reading as JSON (deterministic order)
   - Hash with SHA256: readingHash = SHA256(json)
   - Sign: signature = ECDSA_SIGN(readingHash, privateKey)
   - Append signature to reading

3. Signed Payload
   {
     "reading": {...},
     "signature": "0xd4c9e1a3b2f5...",
     "publicKey": "0x82c4a7f2..."
   }

4. Verification Chain
   - VERIFY(readingHash, signature, publicKey) == True
   - Recover publicKey from signature → match against DID document
   - Match: Data came from sensor | No match: Tampering detected

5. Replay Attack Prevention
   - Each reading has unique nonce
   - Redis cache tracks seen nonces (24h TTL)
   - Duplicate nonce → reject immediately

6. Tests
   - Sign reading, verify signature (happy path)
   - Tamper with reading value → signature fails
   - Replay attack: resend same nonce → rejected
   - Sequence numbers are monotonic (no gaps)

Narrative Success:
"Every data point carries its own 'birth certificate'—a cryptographic proof signed 
by the sensor itself. Anyone can verify: 'This reading came from sensor biochar-001, 
taken at 12:30 UTC on March 5, and has never been modified.'"

Output: Reading signing engine + signature verification + replay attack prevention + tests
```

### PROMPT 4: Sensor Reputation & Trust Scoring

```text
[STAGE 1: BIRTH OF DATA & IDENTITY]
[THE SENSOR'S CHARACTER: Building a reputation ledger]

Your task: Create a Sensor Reputation Scoring System.

Narrative Context:
A sensor with perfect readings deserves higher trust than one with failures. 
We track reputation on-chain, starting each sensor at 50/100, rising or falling 
based on behavior.

Implementation:

1. Reputation Mechanics
   - New sensor: reputation = 50/100
   - Each valid reading: +1 point
   - Each failed signature: -10 points
   - Each tampering detection: -50 points
   - Offline day: -5 points/day
   - Bounds: 0–100

2. Reputation Levels
   - 0–20: "Blacklisted" (auto-quarantine)
   - 21–49: "Untrusted" (lower confidence weighting)
   - 50–79: "Neutral" (normal processing)
   - 80–100: "Trusted" (higher confidence weighting)

3. On-Chain Anchor
   - Updated every batch submission
   - Record: (sensorDID, score, change, reason, timestamp)
   - Immutable audit trail

4. Confidence Weighting
   - High-reputation sensor: confidence *= 1.0
   - Low-reputation sensor: confidence *= 0.7
   - Example: 90% model confidence × 0.85 reputation = 76.5% final

5. Tests
   - Gains reputation with valid readings
   - Drops with failures
   - Cannot exceed 100 or go below 0
   - Confidence weighting applied correctly

Narrative Success:
"A sensor with 3 years of perfect readings is trusted more than a new sensor. 
This reputation is recorded forever on the blockchain."

Output: Reputation calculation engine + on-chain recording + confidence weighting + tests
```

### PROMPT 5: Multi-Chain DID Registration (Cardano Primary)

```text
[STAGE 1: BIRTH OF DATA & IDENTITY]
[ANNOUNCING THE BIRTH: DID lives on Cardano]

Your task: Create DID Registration Smart Contract for Cardano (Plutus).

Narrative Context:
The sensor's DID is registered as an immutable NFT on Cardano blockchain. 
This creates a "digital proof of life" for the sensor.

Implementation:

1. Cardano Smart Contract (Plutus)
   - Mint Sensor NFT on registration
   - Embed DID document in NFT metadata (full doc on IPFS)
   - Immutable record

2. NFT Metadata (on-chain)
   {
     "nft_id": "sensor-biochar-001-nft",
     "sensor_did": "did:cardano:sensor:biochar-001",
     "public_key": "0x82c4a7f2...",
     "location": {"latitude": 43.8, "longitude": -115.9},
     "minted_at": 1704067200,
     "metadata_cid": "QmX7f8P3q2K9mN5..."
   }

3. Smart Contract Validator (Plutus pseudocode)
   validateSensorRegistration :: SensorDID -> PublicKey -> TxOut -> Bool
   validateSensorRegistration did pubKey utxo = do
     assertValidDIDFormat did
     assertValidECDSAKey pubKey
     assertDIDNotRegistered did
     true

4. Verification
   - Anyone can query: hasNFTForDID("did:cardano:sensor:biochar-001")

5. Tests
   - Register sensor → NFT minted successfully
   - Query NFT by DID → metadata correct
   - Duplicate registration → fails
   - NFT metadata immutable

Narrative Success:
"The sensor announces its birth to the world. It mints an NFT on Cardano bearing 
its DID. From this moment forward, the sensor exists in the global ledger—unforgeable."

Output: Cardano smart contract (Plutus) + NFT minting + verification queries + tests
```

### PROMPTS 6–8 (Summary)

```text
PROMPT 6: DID Metadata Hashing
- Deterministic JSON hashing (canonical serialization)
- SHA256 of DID document → stored as "document fingerprint"
- Detect any modification by re-hashing and comparing

PROMPT 7: DID Document IPFS Storage
- Upload full DID document to IPFS
- Get CID, pin to Pinata for redundancy
- Store CID on-chain in NFT metadata
- Retrieve and verify at any time

PROMPT 8: DID Lifecycle State Transitions
- Implement full state machine:
  UNREGISTERED → REGISTERED → ACTIVE → OFFLINE → QUARANTINED → RETIRED
- Guards, actions, and timeouts for each transition
- State changes signed by admin key + logged on-chain
```

---

## STAGE 2: THE GATEWAY (Prompts 9–16)

### PROMPT 9: Batching Engine & Merkle Tree Construction

```text
[STAGE 2: THE GATEWAY]
[GATHERING THE HERD: Many readings become one container]

Your task: Implement the Batching Engine that creates Merkle Trees.

Narrative Context:
The sensor takes 1,440 readings per day. Sending each to the blockchain is expensive. 
Instead, the Gateway batches ~100 readings, hashes each, and combines into a Merkle Tree. 
The single "root hash" represents the entire container.

Why Merkle Trees?
- Energy Efficient: One blockchain commit vs. 100
- Cost Efficient: One transaction fee (~$0.25) vs. 100
- Verifiable: Anyone can prove a reading is in the batch

Implementation:

1. Batching Scheduler
   - Time-based: Every 1 hour OR volume-based: 100 readings (whichever first)
   - Force batch: Sensor offline, critical error, manual trigger

2. Merkle Tree Construction
   Input: [R1, R2, ..., R100]
   Step 1: Hash each → [H1, H2, ..., H100]
   Step 2: If count is odd, duplicate last hash
   Step 3: Pair and hash → [H(H1+H2), H(H3+H4), ...]
   Step 4: Repeat until single root hash → 0x7a4f...

3. Batch Structure
   {
     "batchId": "batch-2025-03-05-1300-cardano-001",
     "sensorDIDs": ["did:cardano:sensor:biochar-001", ...],
     "readings": [...100 readings...],
     "readingCount": 100,
     "hashes": {
       "leaf_hashes": ["0x1a2b...", ...],
       "merkle_root": "0x7a4f..."
     },
     "statistics": {
       "average_reading": 23.4, "min": 22.1, "max": 24.8, "std_dev": 0.6
     },
     "ipfs_cid": "QmX7f8P3q2K9mN5..."
   }

4. Merkle Proof Verification
   - Prove reading is in batch without revealing entire batch
   - Proof = path from leaf to root
   - VERIFY(reading, merkle_proof, merkle_root) == True

5. Optimization: Locality-Sensitive Hashing (LSH)
   - For weather data: Hash statistical fingerprint {mean, std_dev, min, max}
   - 95% compression vs. hashing all 100 readings
   - Only for non-critical data (not biochar)

6. Tests
   - Create Merkle tree with 100 readings
   - Verify root is consistent
   - Generate + verify proof for reading 42
   - Tamper with reading → proof fails
   - LSH reduces proof size by 95%

Narrative Success:
"100 readings compressed into a single container—a Merkle Root. Inside is a 
cryptographic proof that guarantees no reading was lost or modified."

Output: Batching engine + Merkle tree constructor + proof verifier + tests
```

### PROMPTS 10–16 (Summary)

```text
PROMPT 10: Batch Serialization & Deterministic JSON
- Canonical JSON (sorted keys, deterministic floats)
- Same batch always produces same hash

PROMPT 11: Batching Scheduler
- 1h timer + 100-reading volume threshold
- Force-batch triggers (offline sensor, critical error)

PROMPT 12: Kafka Topic Architecture
- Topics: raw-sensor-readings, validated-readings, batch-pending, blockchain-confirmed, alerts
- Partitioned by sensorDID for ordering guarantees

PROMPT 13: Backpressure Handling & Rate Limiting
- Token bucket algorithm
- Jittered sending on sensor SDK (prevent thundering herd)
- Circuit breaker pattern

PROMPT 14: Event-Driven Message Ordering
- Sequence numbers per sensor
- Gap detection (reading 1 → 3, missing 2)
- Kafka partition key = sensorDID

PROMPT 15: Data Deduplication & Idempotency
- Dedup key = (sensorDID, timestamp, value_hash)
- Redis cache with 24h TTL
- Idempotent processing (same input → same output)

PROMPT 16: Batch Validation & QA Checks
- AI confidence scoring (>85% required)
- Outlier detection (sudden 20x spike)
- Blacklist check (quarantined sensors)
```

---

## STAGE 3: SATELLITE & CONSENSUS (Prompts 17–24)

### PROMPT 17: Multi-Validator Quorum & Consensus Logic

```text
[STAGE 3: SATELLITE & CONSENSUS]
[THE COUNCIL DECIDES: 3 validators vote on where data goes]

Your task: Implement Multi-Validator Quorum for Merkle Root Submission.

Narrative Context:
The batch is ready. No single "Big Boss" can approve it. Three independent validators 
vote. If 2-of-3 agree, the batch is accepted.

Implementation:

1. Validator Network
   - Validator 1: Mālama Labs
   - Validator 2: Verra Registry
   - Validator 3: Community/third-party
   - Each has own signing key (non-shared)

2. Submission Protocol
   - Gateway sends batch to all 3 validators in parallel
   - Each independently verifies:
     * Merkle root is valid
     * AI confidence > 85%
     * Sensor DIDs not blacklisted
   - Each signs: "I approve this batch"

3. Quorum Check (Rust)
   pub fn check_quorum(signatures: Vec<ValidatorSignature>) -> bool {
       let valid_sigs = signatures.iter()
           .filter(|sig| verify_signature(sig))
           .count();
       valid_sigs >= 2
   }

4. Smart Contract (Plutus)
   validateBatchSubmission :: MerkleRoot -> [ValidatorSig] -> Bool
   validateBatchSubmission root sigs = do
     validSigs <- filterValidSignatures sigs
     assertQuorum validSigs 2 3
     assertValidMerkleRoot root
     true

5. Reputation Scoring
   - Uptime: % of submissions signed within 5 min
   - Accuracy: % approved by Verra
   - Remove validator if reputation < 50%

6. Failure Handling
   - 2 approve, 1 rejects → batch accepted, disagreement logged
   - Only 1 signature after 1 hour → timeout, retry

7. Tests
   - 3 validators approve → accepted
   - 2 approve, 1 rejects → accepted
   - Only 1 approves → rejected
   - Invalid signature → fails
   - Validator reputation tracked correctly

Narrative Success:
"Three independent voices agree: this batch is legitimate. The consensus is recorded 
forever on-chain as proof the decision was democratic."

Output: Quorum logic + signature aggregation + reputation tracking + tests
```

### PROMPTS 18–24 (Summary)

```text
PROMPT 18: Graph Partitioning & Adaptive Packaging
- Which sensors go to which validator node
- Load balancing by geography + sensor count

PROMPT 19: Validator Selection & Load Balancing
- Route to fastest available validator
- Failover if primary validator is offline

PROMPT 20: Consensus Timeout & Retry Logic
- Exponential backoff: 1s → 2s → 4s → 8s
- Rotate to backup validator after 3 failures

PROMPT 21: Validator Reputation Calculation
- Weighted scoring: uptime, accuracy, speed
- Slashing mechanics for dishonest validators

PROMPT 22: Blockchain Proof Recording
- Record quorum decision on-chain: {merkleRoot, validatorSigs, timestamp}
- Immutable audit trail of every consensus decision

PROMPT 23: Dispute Resolution
- What happens when 2-of-3 approve but 1 rejects?
- Log dispute, investigate, escalate to manual review

PROMPT 24: Validator Network Monitoring & Health Checks
- Ping each validator every 5 minutes
- Alert if any validator goes offline
- Auto-rotate to backup validator
```

---

## STAGE 4: STORAGE PROVIDER (Prompts 25–32)

### PROMPT 25: Chain Adapter Pattern & Multi-Chain Routing

```text
[STAGE 4: STORAGE PROVIDER]
[CHOOSING THE BLOCKCHAIN HOME: Cardano vs. BASE vs. HEDERA vs. CELO]

Your task: Implement the Chain Adapter Pattern for blockchain-agnostic data.

Narrative Context:
The same batch registered on ALL FOUR chains simultaneously. Each chain serves different users:
- Cardano: Climate credibility (ISO 14064-5, formal verification)
- BASE: Enterprise adoption (Coinbase, Web2→Web3)
- HEDERA: Sovereign programs (Article 6.4, government validators)
- CELO: Mobile-first farmers (Africa/Asia, smallholder access)

Implementation:

1. Chain Adapter Trait (Rust)
   pub trait ChainAdapter: Send + Sync {
       async fn submit_merkle_root(
           &self, batch_id: String, merkle_root: [u8; 32],
           ipfs_cid: String, validator_sigs: Vec<Signature>,
       ) -> Result<TransactionHash>;
       
       async fn update_reputation(&self, sensor_did: &str, confidence: f64)
           -> Result<TransactionHash>;
       
       async fn mint_lco2(&self, project_id: String, amount: u128, confidence: f64)
           -> Result<TokenMintTx>;
       
       fn chain_id(&self) -> String;
       fn estimated_fee(&self) -> u128;
   }

2. Concrete Adapters
   - CardanoAdapter (Pallas/Blockfrost, Plutus contracts)
   - BaseAdapter (ethers-rs, Solidity ERC-721)
   - HederaAdapter (Hedera SDK, HCS + HTS)
   - CeloAdapter (celo-ethers, mobile-optimized)

3. Multi-Chain Submission
   async fn submit_to_all_chains(batch: &Batch) -> Result<MultiChainProof> {
       let adapters: Vec<Box<dyn ChainAdapter>> = vec![
           Box::new(CardanoAdapter::new()),
           Box::new(BaseAdapter::new()),
           Box::new(HederaAdapter::new()),
           Box::new(CeloAdapter::new()),
       ];
       
       let results = futures::future::join_all(
           adapters.iter().map(|a| a.submit_merkle_root(...))
       ).await;
       
       Ok(MultiChainProof { proofs: results })
   }

4. Cost Optimization
   - Cardano: ~$0.25 (climate credibility)
   - BASE: ~$0.001 (enterprise, cheap)
   - HEDERA: ~$0.0001 (sovereign, fixed)
   - CELO: ~$0.01 (mobile-first)

5. Tests
   - Submit to each chain testnet
   - Verify same data on all 4 chains
   - Cost estimation correct per chain
   - Parallel submission with fault isolation

Narrative Success:
"The batch is now recorded on FOUR blockchains simultaneously. A hospital can verify 
on Cardano. An enterprise on BASE. A government on HEDERA. A farmer on CELO."

Output: Chain adapter trait + all 4 adapters + multi-chain submission + tests
```

### PROMPTS 26–32 (Summary)

```text
PROMPT 26: IPFS Batch Upload & Pinata Integration
- Serialize batch → upload to IPFS → get CID
- Pin to Pinata + secondary node for redundancy
- Store CID in blockchain transaction

PROMPT 27: Cardano Plutus Contract Deployment
- SensorRegistry.hs, MerkleRootAnchor.hs, ReputationTracker.hs
- Deploy to Cardano testnet
- Integration test with real Cardano RPC

PROMPT 28: BASE Solidity Contract Deployment
- SensorRegistry.sol (ERC-721), MerkleRootAnchor.sol, CarbonTokens.sol
- Deploy with Hardhat to BASE testnet
- Verify on Basescan

PROMPT 29: HEDERA HCS & HTS Integration
- HCS topic for Merkle roots
- HTS tokens for LCO₂ and VCO₂
- Article 6.4 sovereign program support

PROMPT 30: CELO Mobile SDK Integration
- USSD gateway for SMS-based sensor reading submission
- cUSD stablecoin settlement
- Smallholder farmer onboarding flow

PROMPT 31: Transaction Fee Optimization & Cost Calculator
- Estimate fees across all chains before submitting
- Route critical data to Cardano, non-critical to HEDERA/BASE
- Aggregate savings report

PROMPT 32: Cross-Chain Consistency Verification
- Verify same Merkle root exists on all 4 chains
- Alert if any chain is missing the submission
- Automatic resubmission on failure
```

---

## STAGE 5: SAFETY NET & INTEGRITY (Prompts 33–40)

### PROMPT 33: Immutability Proofs & LSH Verification

```text
[STAGE 5: SAFETY NET & INTEGRITY]
[THE POSTCARD & THE PACKAGE: Merkle root on-chain, batch data off-chain]

Your task: Implement Immutability Verification using Locality-Sensitive Hashing (LSH).

Narrative Context:
Our 100-reading batch is split:
- "Postcard" (Merkle Root, 32 bytes) → blockchain, immutable forever
- "Package" (full batch data, ~32KB) → IPFS, permanent but verifiable

How do we prove the Package wasn't tampered with? Merkle proofs + LSH.

Implementation:

1. Verification Flow
   Auditor wants to verify a reading from March 2025:
   Step 1: Query blockchain for Merkle root from March
   Step 2: Fetch batch data from IPFS (via CID)
   Step 3: Reconstruct Merkle tree from batch
   Step 4: Verify Merkle root matches on-chain
   Result: "This batch has never been modified"

2. LSH Implementation (Rust)
   pub fn compute_lsh_fingerprint(readings: &[f64]) -> [u8; 32] {
       let mean = readings.iter().sum::<f64>() / readings.len() as f64;
       let variance = readings.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
           / readings.len() as f64;
       let std_dev = variance.sqrt();
       let min = readings.iter().cloned().fold(f64::INFINITY, f64::min);
       let max = readings.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
       let fingerprint = format!("{mean:.2}|{std_dev:.2}|{min:.2}|{max:.2}");
       sha256(fingerprint.as_bytes())
   }
   
   // Reduces proof from 320 bytes → 32 bytes (90% compression)

3. GDPR Compliance
   - Blockchain: immutable (Merkle root stays forever)
   - IPFS: batch data has TTL (deleted after 3 years for GDPR)
   - Proof remains: "Data existed and was verified on this date"

4. Tests
   - Batch uploaded to IPFS, CID returned
   - Merkle root matches on-chain proof
   - Tamper with batch on IPFS → verification fails
   - LSH compression reduces size by 90%
   - Off-chain data deletable, on-chain proof permanent

Narrative Success:
"The reading is locked in time. A Merkle root on Cardano proves it existed and was 
accurate. The full data can be deleted for GDPR, but the proof remains forever."

Output: Immutability verification + LSH compression + GDPR-compliant deletion + tests
```

### PROMPTS 34–40 (Summary)

```text
PROMPT 34: Merkle Proof Generation & Verification
- Generate path from leaf to root
- Single-file proof: {reading, siblings[], root}
- Anyone can verify without downloading full batch

PROMPT 35: IPFS Pinning, Redundancy & Monitoring
- Pin to Pinata + own IPFS node + Filecoin
- Monitor availability (periodic retrieval test)
- Alert if any pin goes offline

PROMPT 36: Content Addressing & CID Validation
- Verify CID matches content (re-hash and compare)
- Detect if IPFS content was modified (CID mismatch)

PROMPT 37: Blockchain Fork Handling & Reorg Protection
- Listen for reorgs (block depth > 6 = safe)
- Rollback indexer if chain reorganizes
- Never submit to blockchain during fork

PROMPT 38: Data Retention Policies & Archival
- Hot storage: Last 30 days (fast retrieval)
- Warm storage: 30 days - 3 years (slower, cheaper)
- Cold storage: 3+ years (Filecoin, archive)
- Automatic tier migration

PROMPT 39: Audit Trail Reconstruction
- Reconstruct full chain of custody from on-chain events
- "Prove this reading existed in this batch on this date"
- Export as PDF report for VVB auditors

PROMPT 40: Tamper Detection & Automatic Alerts
- Periodic spot-checks: re-retrieve and re-verify 1% of batches
- Alert if any tampering detected
- Quarantine affected sensor automatically
```

---

## STAGE 6: MULTI-CHAIN SETTLEMENT (Prompts 41–48)

### PROMPT 41: Smart Contract Automation & Token Issuance

```text
[STAGE 6: MULTI-CHAIN SETTLEMENT]
[THE MOMENT OF TRUTH: Automatic LCO₂ minting when confidence > 80%]

Your task: Implement Smart Contract Logic for Automatic Carbon Token Issuance.

Narrative Context:
A farmer plants biochar. Sensors stream removal data. AI calculates 87% confidence 
that 1,050 tCO₂e was removed. The smart contract automatically mints 1,050 LCO₂ 
tokens. The farmer sells these immediately for ~$189k, without waiting 12 months.

Implementation:

1. LCO₂ Token (ERC-20 / Cardano Native)
   - Pre-finance: Issued before VVB approval
   - Amount = carbon_removed × pre_finance_ratio (35%)
   - Gated: Only mint if confidence_score > 0.80
   - Burnable: Burned 1:1 when VCO₂ is issued

2. Confidence-Gated Minting Logic
   pub fn should_mint_lco2(confidence: f64, carbon_removed: f64) -> Option<u128> {
       if confidence >= 0.80 {
           let amount = (carbon_removed * 1e6 as f64) as u128; // micro-tokens
           Some(amount)
       } else {
           None // Not enough confidence, wait for more data
       }
   }

3. Settlement Flow
   [Sensor Data] → [AI Confidence 87%] → [Smart Contract checks > 80%]
   → [Mint 1,050 LCO₂ to farmer wallet]
   → [Farmer sells LCO₂ on secondary market: $189k]
   → [6 months later: VVB approves]
   → [Burn 1,050 LCO₂, Mint 1,050 VCO₂]
   → [Farmer sells VCO₂ for $250–300k]

4. Smart Contract (Solidity/Plutus)
   function mintLCO2(
       string memory projectId,
       uint256 carbonRemoved,
       uint256 confidence, // 0-100
       bytes32 merkleRoot,
       bytes memory validatorSigs
   ) external {
       require(confidence >= 80, "Confidence too low");
       require(verifyQuorum(merkleRoot, validatorSigs), "Quorum not met");
       _mint(farmerWallet, carbonRemoved);
       emit LCO2Minted(projectId, carbonRemoved, confidence);
   }

5. Tests
   - Confidence 87% → LCO₂ minted correctly
   - Confidence 70% → no minting, event logged
   - Quorum not met → minting blocked
   - LCO₂ burned correctly when VCO₂ issued

Narrative Success:
"The data point's journey culminates in financial value. A sensor reading taken 70 
days ago just triggered $189,000 in carbon pre-finance for a farmer in Idaho."

Output: LCO₂/VCO₂ smart contracts + confidence-gated minting + settlement engine + tests
```

### PROMPTS 42–48 (Summary)

```text
PROMPT 42: LCO₂ Token Design (ERC-20 + Cardano Native)
- Pre-finance carbon tokens
- Metadata: projectId, methodology, confidence, timestamp
- Transfer and burn restrictions

PROMPT 43: VCO₂ Token Design (Verified Credits)
- Post-verification carbon credits
- Minted only after VVB approval
- Retirement tracking (cannot be un-retired)

PROMPT 44: Confidence-Gated Minting Engine
- Real-time confidence monitoring
- Auto-mint when 80% threshold crossed
- Partial minting for borderline cases (80–85%)

PROMPT 45: Prediction Market Settlement
- Auto-settle if AI confidence > 90%
- Distribute escrow funds to winning side
- Partial settlement for borderline cases

PROMPT 46: DePIN Reward Distribution
- Reward sensor operators for uptime
- Bonus for high reputation sensors
- Slashing for tampering or extended offline

PROMPT 47: Registry Settlement (Verra/Gold Standard)
- Submit evidence bundle to registry API
- Poll for approval status
- Trigger VCO₂ minting on approval

PROMPT 48: Slashing Mechanism
- Penalize dishonest sensors (fabricated readings)
- Automatic detection: LSH similarity < 0.5
- Stakes slashed proportionally
```

---

## STAGE 7: VERIFICATION & PROOF (Prompts 49–56)

### PROMPT 49: End-to-End Verification Workflow

```text
[STAGE 7: VERIFICATION & PROOF]
[THE FINAL CHAPTER: Complete traceability from sensor to credit]

Your task: Build the Complete End-to-End Verification Workflow.

Narrative Context:
A hospital receives medicine monitored by a temperature sensor. They want to verify: 
"Did this medicine really stay at -18°C the entire journey?"

They follow the Odyssey backward:
1. Find Merkle root on blockchain (Cardano)
2. Fetch batch from IPFS (verify CID matches content)
3. Reconstruct Merkle tree (prove medicine reading is in batch)
4. Verify sensor signature (prove sensor is real)
5. Check sensor reputation (87/100, excellent)
6. Query sensor DID on blockchain (sensor exists, not spoofed)
Result: "Medicine stayed at -18°C, confirmed by tamper-proof system"

Implementation:

1. Verification API
   GET /verify/reading?sensorDID=...&timestamp=...&value=23.4
   
   Response:
   {
     "verified": true,
     "confidence": 0.97,
     "chain_of_proof": {
       "step1_blockchain": {
         "merkle_root": "0x7a4f...",
         "tx_hash": "abc123...",
         "chain": "cardano",
         "block": 8501234
       },
       "step2_ipfs": {
         "cid": "QmX7f8P3q2K9mN5...",
         "batch_hash_verified": true
       },
       "step3_merkle_proof": {
         "reading_in_batch": true,
         "proof_path": ["0x1a2b...", "0x3c4d...", ...]
       },
       "step4_signature": {
         "signature_valid": true,
         "public_key": "0x82c4a7f2..."
       },
       "step5_reputation": {
         "score": 87,
         "level": "trusted"
       },
       "step6_did": {
         "registered_on_chain": true,
         "nft_id": "sensor-biochar-001-nft"
       }
     }
   }

2. Export Formats
   - JSON (machine-readable)
   - PDF (auditor-readable, with charts)
   - QR Code (scan to verify on mobile)

3. Verification Speed
   - Target: <5 seconds end-to-end
   - Cache blockchain state (10 min TTL)
   - Pre-index Merkle proofs for fast lookup

4. Tests
   - Verify valid reading: all 6 steps pass
   - Tampered reading: step 3 fails (Merkle proof invalid)
   - Spoofed sensor: step 6 fails (DID not on blockchain)
   - Full audit trail exported as PDF

Narrative Success:
"The Odyssey is complete. A single data point born at 12:30 UTC on March 5 
can be proven tamper-proof by anyone in the world, at any time, forever."

Output: End-to-end verification API + PDF export + QR code + tests
```

### PROMPTS 50–56 (Summary)

```text
PROMPT 50: Explorer Dashboard
- Visual blockchain explorer for Mālama
- Search by: sensorDID, project, date, batch
- Real-time sensor status map

PROMPT 51: Audit Trail Visualization
- Interactive timeline of data point's journey
- Clickable nodes: each stage of the Odyssey
- Export as regulatory compliance report

PROMPT 52: Stakeholder-Specific Views
- Farmer view: my sensors, my credits, my revenue
- Hospital view: verify product integrity
- Regulator view: full audit trail, compliance docs
- Enterprise buyer view: credit portfolio

PROMPT 53: Integration Tests (Complete E2E Workflows)
- Test full Odyssey: sensor birth → blockchain settlement
- Simulate 1,000 sensors over 30 days
- Verify all credits issued correctly

PROMPT 54: Load Testing & Performance Benchmarks
- Test with 10,000 simultaneous sensors
- Verify Kafka lag stays < 1 min
- Measure blockchain submission latency by chain

PROMPT 55: Security Testing & Penetration Testing
- Simulate replay attacks (rejected correctly)
- Simulate validator collusion (detected)
- Simulate GPS spoofing (caught by multi-method verification)
- Simulate Sybil attack (reputation system defense)

PROMPT 56: Deployment & Mainnet Launch
- Deploy Plutus contracts to Cardano mainnet
- Deploy Solidity contracts to BASE mainnet
- Deploy HCS/HTS to HEDERA mainnet
- Deploy Solidity contracts to CELO mainnet
- Launch farmer onboarding (QR code scan)
```

---

## CODE ORGANIZATION

```text
src/
├── stage1_birth_identity/
│   ├── did_generator.rs
│   ├── provenance_recorder.rs
│   ├── reading_signer.rs
│   ├── reputation_scorer.rs
│   └── cardano_did_registration.rs
├── stage2_gateway/
│   ├── batching_engine.rs
│   ├── merkle_tree.rs
│   ├── kafka_producer.rs
│   ├── backpressure_handler.rs
│   └── deduplication.rs
├── stage3_consensus/
│   ├── multi_validator.rs
│   ├── quorum_logic.rs
│   ├── graph_partitioning.rs
│   └── consensus_proof.rs
├── stage4_storage/
│   ├── chain_adapter.rs
│   ├── cardano_adapter.rs
│   ├── base_adapter.rs
│   ├── hedera_adapter.rs
│   ├── celo_adapter.rs
│   └── ipfs_client.rs
├── stage5_integrity/
│   ├── immutability_verifier.rs
│   ├── lsh_compression.rs
│   └── gdpr_compliance.rs
├── stage6_settlement/
│   ├── smart_contracts/
│   │   ├── lco2_token.sol
│   │   ├── vco2_token.sol
│   │   └── settlement_contract.hs
│   └── settlement_engine.rs
└── stage7_verification/
    ├── end_to_end_verifier.rs
    ├── explorer_api.rs
    └── audit_report.rs
```

---

## SUCCESS CRITERIA

The Odyssey is complete when:

- ✅ Data point born with DID identity (Stage 1)
- ✅ Reading signed with ECDSA cryptographic proof (Stage 1)
- ✅ Batch created with Merkle tree (Stage 2)
- ✅ Consensus achieved by 2-of-3 validators (Stage 3)
- ✅ Batch anchored on 4 blockchains simultaneously (Stage 4)
- ✅ Data provably immutable + verifiable (Stage 5)
- ✅ LCO₂ tokens auto-minted when confidence > 80% (Stage 6)
- ✅ End-to-end verification in < 5 seconds (Stage 7)
- ✅ 80%+ test coverage, all tests passing
- ✅ MVP deployed to Cardano/BASE/HEDERA/CELO testnets

---

## TIMELINE OF A SINGLE DATA POINT

| Time | Event |
|------|-------|
| T=0:00 | Sensor reads 23.4°C, signs with ECDSA |
| T=0:01–1:00 | Reading queued locally (offline-first) |
| T=1:00 | 100 readings batched into Merkle tree |
| T=1:05 | Batch sent to 3 validators for voting |
| T=1:06 | Validators approve (2-of-3 quorum) |
| T=1:07 | Batch uploaded to IPFS (CID: QmX7f8...) |
| T=1:08 | Merkle root → Cardano ($0.25) |
| T=1:09 | Merkle root → BASE ($0.001) |
| T=1:10 | Merkle root → HEDERA ($0.0001) |
| T=1:11 | Merkle root → CELO ($0.01) |
| T=1:12 | Smart contract: 87% confidence × 1,050 tCO₂e |
| T=1:13 | Auto-mint 1,050 LCO₂ tokens to farmer wallet |
| T=1:14 | Farmer sells LCO₂ for $189k (doesn't wait 12 months) |
| T=6 months | VVB approves → 1,050 VCO₂ minted (burns LCO₂ 1:1) |
| T=∞ | Reading remains immutable—hospital verifies at any time |

---

*Document Version: 2.0 — Odyssey Framework*  
*Status: Ready for Cursor Implementation*  
*Total Prompts: 56 | Estimated Build Time: 20 weeks*
