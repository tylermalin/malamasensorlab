pub mod lsh_engine;
pub mod custody;
pub mod verifier;

#[cfg(test)]
mod tests {
    use super::lsh_engine::*;
    use super::custody::*;

    #[test]
    fn test_lsh_near_duplicate() {
        let lsh = LshEngine::new(42);
        let data1 = b"temperature sensor reading: 22.5C. location: Boise.";
        let data2 = b"temperature sensor reading: 22.7C. location: Boise."; // Slightly different
        let data3 = b"totally different data point from a co2 sensor.";

        let fp1 = lsh.fingerprint(data1);
        let fp2 = lsh.fingerprint(data2);
        let fp3 = lsh.fingerprint(data3);

        let dist_near = LshEngine::hamming_distance(fp1, fp2);
        let dist_far = LshEngine::hamming_distance(fp1, fp3);

        assert!(dist_near < dist_far);
    }

    #[test]
    fn test_chain_of_custody() {
        let mut coc = ChainOfCustody::new();
        let private_key = k256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);

        coc.add_link("did:1".into(), "hash_0".into(), "hash_1".into(), &private_key);
        coc.add_link("did:2".into(), "hash_1".into(), "hash_2".into(), &private_key);
        
        assert!(coc.verify_chain());
        
        // Break the chain
        coc.links[1].input_hash = "tampered_hash".into();
        assert!(!coc.verify_chain());
    }
}
