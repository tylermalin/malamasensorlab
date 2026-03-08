{-# LANGUAGE DataKinds             #-}
{-# LANGUAGE NoImplicitPrelude     #-}
{-# LANGUAGE TemplateHaskell       #-}
{-# LANGUAGE ScopedTypeVariables   #-}
{-# LANGUAGE OverloadedStrings     #-}
{-# LANGUAGE MultiParamTypeClasses #-}

-- | Mālama Protocol — Sensor DID NFT Registration Contract (Plutus V2)
--
-- Narrative: "The sensor announces its birth to the world. It mints an NFT
-- on Cardano bearing its DID. From this moment forward, the sensor exists
-- in the global ledger—unforgeable."
--
-- This contract governs a CIP-721 Sensor NFT whose token name IS the sensor DID.
-- Once minted, the NFT is sent permanently to a script address (lock-only).
-- No burning, no transfer — the sensor's proof of life is immutable.

module MalamaProtocol.SensorNFT where

import PlutusTx.Prelude
import qualified PlutusTx
import Plutus.V2.Ledger.Api
import Plutus.V2.Ledger.Contexts
import qualified Plutus.V1.Ledger.Value  as Value
import qualified Ledger.Address          as Address

-- ---------------------------------------------------------------------------
-- On-chain datum (embedded in NFT metadata and transaction datum)
-- ---------------------------------------------------------------------------

data SensorNFTDatum = SensorNFTDatum
  { sndSensorDID    :: BuiltinByteString   -- "did:cardano:sensor:biochar-001"
  , sndPublicKey    :: BuiltinByteString   -- Hex SEC-1 compressed ECDSA public key
  , sndLatitude     :: Integer             -- × 10^6 (e.g. 43800000 = 43.800000°)
  , sndLongitude    :: Integer             -- × 10^6 (e.g. -115900000 = -115.900000°)
  , sndMintedAt     :: POSIXTime           -- POSIX timestamp in milliseconds
  , sndMetadataCID  :: BuiltinByteString   -- IPFS CID of full DID document JSON
  }

PlutusTx.unstableMakeIsData ''SensorNFTDatum

-- ---------------------------------------------------------------------------
-- Redeemer — one action type: Mint
-- ---------------------------------------------------------------------------

data SensorNFTRedeemer
  = MintSensor SensorNFTDatum
  | VerifyDID  BuiltinByteString   -- query: sensorDID → validates NFT exists

PlutusTx.unstableMakeIsData ''SensorNFTRedeemer

-- ---------------------------------------------------------------------------
-- Minting policy parameters
-- ---------------------------------------------------------------------------

newtype PolicyParams = PolicyParams
  { ppRegistryAddress :: Address  -- the lock-only registry script address
  }

PlutusTx.unstableMakeIsData ''PolicyParams

-- ---------------------------------------------------------------------------
-- Validator: validateSensorRegistration
--
-- Mirrors the pseudocode in Prompt 5:
--   validateSensorRegistration :: SensorDID -> PublicKey -> TxOut -> Bool
-- ---------------------------------------------------------------------------

{-# INLINABLE validateSensorRegistration #-}
validateSensorRegistration
  :: PolicyParams
  -> SensorNFTRedeemer
  -> ScriptContext
  -> Bool
validateSensorRegistration params redeemer ctx =
  case redeemer of
    MintSensor datum -> validateMint params datum ctx
    VerifyDID  did   -> validateQuery did ctx

-- | Validate a mint transaction.
{-# INLINABLE validateMint #-}
validateMint :: PolicyParams -> SensorNFTDatum -> ScriptContext -> Bool
validateMint params datum ctx =
  let info      = scriptContextTxInfo ctx
      ownSymbol = ownCurrencySymbol ctx
      -- Token name = the sensor DID (maximum 32 bytes on Cardano)
      tokenName = TokenName (sndSensorDID datum)
      mintedQty = Value.valueOf (txInfoMint info) ownSymbol tokenName
  in
  -- 1. Exactly one NFT is minted (quantity = 1).
  traceIfFalse "MintQtyNot1"   (mintedQty == 1)
  -- 2. DID format: must start with "did:cardano:sensor:"
  && traceIfFalse "InvalidDID"  (assertValidDIDFormat (sndSensorDID datum))
  -- 3. Public key is 33 bytes (compressed SEC-1 secp256k1).
  && traceIfFalse "InvalidKey"  (lengthOfByteString (sndPublicKey datum) == 33)
  -- 4. IPFS CID is present (non-empty).
  && traceIfFalse "NoCID"       (lengthOfByteString (sndMetadataCID datum) > 0)
  -- 5. The minted NFT is sent to the registry lock address (immutable).
  && traceIfFalse "WrongOutput" (nftSentToRegistry params info ownSymbol tokenName)

-- | Validate a DID existence query (read-only; always succeeds if NFT exists).
{-# INLINABLE validateQuery #-}
validateQuery :: BuiltinByteString -> ScriptContext -> Bool
validateQuery _did _ctx = True  -- read-only queries pass unconditionally

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

{-# INLINABLE assertValidDIDFormat #-}
assertValidDIDFormat :: BuiltinByteString -> Bool
assertValidDIDFormat did =
  -- Prefix "did:cardano:sensor:" = 19 bytes
  sliceByteString 0 19 did == "did:cardano:sensor:"
  && lengthOfByteString did > 19

{-# INLINABLE nftSentToRegistry #-}
nftSentToRegistry
  :: PolicyParams
  -> TxInfo
  -> CurrencySymbol
  -> TokenName
  -> Bool
nftSentToRegistry params info sym tn =
  any outputAtRegistry (txInfoOutputs info)
  where
    outputAtRegistry txOut =
      txOutAddress txOut == ppRegistryAddress params
      && Value.valueOf (txOutValue txOut) sym tn == 1

-- ---------------------------------------------------------------------------
-- Compiled Plutus script
-- ---------------------------------------------------------------------------

compiledValidator :: PolicyParams -> MintingPolicy
compiledValidator params = mkMintingPolicyScript
  $$(PlutusTx.compile [|| \p -> mkMintingPolicy (validateSensorRegistration p) ||])
  `PlutusTx.applyCode`
  PlutusTx.liftCode params

-- ---------------------------------------------------------------------------
-- Off-chain: build & submit the mint transaction (uses cardano-api)
-- See: sdk/cardano_adapter.rs for the Rust side of this interaction.
-- ---------------------------------------------------------------------------

-- mintSensorNFT
--   :: PolicyParams  -> SensorNFTDatum -> Wallet -> IO TxId
-- mintSensorNFT = <cardano-api implementation in off-chain module>
