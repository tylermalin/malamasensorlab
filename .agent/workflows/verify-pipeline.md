---
description: run end-to-end pipeline verification from sensor reading to LCO2 minting
---

1. Generate 100 mock sensor readings using the test harness in `src/stage1_birth_identity/`.
2. Confirm readings appear in Kafka topic `raw-sensor-readings`.
3. Wait for batch to form (either 100 readings or 1-hour timer elapsed).
4. Confirm Merkle root was created and logged.
5. Confirm Merkle root was submitted to all configured chains — check each adapter's response.
6. Confirm batch data was uploaded to IPFS — retrieve CID and verify content hash matches.
7. If confidence score > 80%: confirm LCO₂ minting transaction was submitted.
8. Open the dashboard in the browser and confirm:
   - Sensor status shows ACTIVE
   - Latest batch appears in the Data Journal
   - Credit estimate is non-zero (if minting occurred)
9. Output a summary report:
   - Kafka consumer lag (must be < 60s)
   - Blockchain tx hashes for all 4 chains
   - IPFS CID
   - LCO₂ amount minted (or reason it was skipped)
   - Pass / Fail for each step
