# Mālama Protocol SDK — Journey of a Data Point

The Mālama Protocol is a production-grade framework for secure sensor data ingestion, verifiable carbon credit settlement, and transparent environmental proofing. This SDK provides the building blocks for the "7 Stages of a Data Point," transitioning environmental telemetry from hardware birth to high-integrity financial settlement.

## 🚀 The 7 Stages

1. **Stage 1: Data Birth & Identity**: Cryptographic onboarding of sensors with Secp256k1 keys and Decentralized Identifiers (DIDs).
2. **Stage 2: Gateway & Aggregation**: Secure ingestion of signed readings, batching, and Write Ahead Logging (WAL) for ACID durability.
3. **Stage 3: Consensus**: Decentralized validation and voting on data batches.
4. **Stage 4: Storage & Anchoring**: Content-addressed storage on IPFS/Filecoin and anchoring Merkle roots across multiple L1/L2 chains.
5. **Stage 5: Integrity & Custody**: Probabilistic verification (LSH) and verifiable chain of custody for every data packet.
6. **Stage 6: Multi-Chain Settlement**: Parallel carbon credit minting and settlement across Hedera, EVM (Celo/Polygon), and Cardano.
7. **Stage 7: Verification & Proof of Journey**: Final certificate generation and explorer logic for end-to-end transparency.

## 🛠️ Developer Guide: Local Setup

### Prerequisites

- **Rust**: [Install via rustup](https://rustup.rs/) (latest stable)
- **Node.js**: [v18 or higher](https://nodejs.org/)
- **Git**: [Latest version](https://git-scm.com/)

### 🦀 Building the Core (Rust)

The core logic handles cryptographic identity, gateway aggregation, and settlement orchestration.

```bash
# Verify Rust installation
cargo --version

# Run the full test suite (16+ tests)
cargo test

# Build for development
cargo build
```

### 🖥️ Building the Dashboard (React)

The Malama Dashboard is a high-fidelity internal tool for sensor onboarding and system monitoring.

```bash
# Navigate to the dashboard directory
cd dashboard

# Install dependencies
npm install

# Start the development server (Hot Module Replacement enabled)
npm run dev

# Build for production
npm run build
```

The app will be available at `http://localhost:5173/`.

## 🏗️ Architecture

- **Core**: High-performance Rust engine for cryptographic verification and consensus logic.
- **SDK (TS)**: Developer-friendly TypeScript wrappers for easy integration into web and mobile applications.
- **Dashboard**: Internal application for sensor management and onboarding.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
