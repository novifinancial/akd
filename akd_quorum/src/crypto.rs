// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under both the MIT license found in the
// LICENSE-MIT file in the root directory of this source tree and the Apache
// License, Version 2.0 found in the LICENSE-APACHE file in the root directory
// of this source tree.

//! This module contains the cryptographic operations which need to be
//! performed, including storage & retrieval of private cryptographic operations
//!
//! NOTE: Instead of Shamir secret sharing, we may want to look into
//! threshold signatures (e.g. https://github.com/poanetwork/threshold_crypto)
//! which will avoid the need to ever reconstruct the private key while maintaining
//! a public key which can be used to verify the signatures from a consensus of the network
//! HOWEVER if we remain within a secure context when reconstructing the shards and generating
//! the signed commitment, then we should be safe from exploit. Moving to a public
//! participation might require an adjustment to this.
//!
//! Additionally it is unclear if threshold signatures can be adjusted after they are
//! initially created. Which is a requirement for mutation of the quorum set.

use crate::comms::Nonce;
use crate::storage::QuorumCommitment;
use crate::QuorumOperationError;

use async_trait::async_trait;
use shamirsecretsharing::{combine_shares, create_shares, DATA_SIZE, SHARE_SIZE};
use std::convert::TryInto;
use winter_crypto::Hasher;

// =====================================================
// Consts and Typedefs
// =====================================================

/// The multiplicitave factor of DATA_SIZE which denotes the size of the
/// quorum key. Probably should be a factor of 2
pub(crate) const QUORUM_KEY_NUM_PARTS: usize = 8;

/// The size of the quorum key private key in bytes.
/// NOTE: SSS's DATA_SIZE = 64 bytes, which the quorum key private key
/// need to be a multiple of
pub const QUORUM_KEY_SIZE: usize = QUORUM_KEY_NUM_PARTS * DATA_SIZE;

// =====================================================
// Structs
// =====================================================

/// Represents the node's "shard" of the secret quorum's private
/// signing key. A single shard cannot be utilized to reconstruct the
/// full quorum key.
///
/// Due to limitations of the Shamir's Secret Sharing lib, we are constrained
/// to break the secret information into batches of DATA_SIZE _exactly_ to generate
/// the shards. This means that to support a key bigger than DATA_SIZE, we need to
/// have multiple shards for each slice of the secret information.
pub struct QuorumKeyShard {
    pub(crate) components: [[u8; SHARE_SIZE]; QUORUM_KEY_NUM_PARTS],
}

impl Clone for QuorumKeyShard {
    fn clone(&self) -> Self {
        Self {
            components: self.components,
        }
    }
}

impl QuorumKeyShard {
    pub(crate) fn build_from_vec_vec_vec(
        data: Vec<Vec<Vec<u8>>>,
    ) -> Result<Vec<Self>, QuorumOperationError> {
        let mut results = vec![];

        for shards in data.into_iter() {
            let mut formatted_shards: Vec<[u8; SHARE_SIZE]> = vec![];
            for shard in shards.into_iter() {
                formatted_shards.push(shard.try_into().map_err(|_| {
                    QuorumOperationError::Sharding(format!(
                        "Unable to convert shard vec into array of len {}",
                        DATA_SIZE
                    ))
                })?)
            }
            let formatted_shard = Self {
                components: formatted_shards.try_into().map_err(|_| QuorumOperationError::Sharding(format!("Unable to format vector of shards into quorum key shard struct with {} components", QUORUM_KEY_NUM_PARTS)))?
            };
            results.push(formatted_shard);
        }

        Ok(results)
    }
}

// =====================================================
// Trait definitions
// =====================================================

/// Represents the cryptographic operations which the node needs to execute
/// within a secure context (e.g. HSM)
#[async_trait]
pub trait QuorumCryptographer {
    // ==================================================================
    // To be implemented
    // ==================================================================

    /// Retrieve the public key of this quorum node
    async fn retrieve_public_key(&self) -> Result<Vec<u8>, QuorumOperationError>;

    /// Retrieve the public key of the Quorum Key
    async fn retrieve_qk_public_key(&self) -> Result<Vec<u8>, QuorumOperationError>;

    /// Retrieve this node's shard of the quorum key from persistent storage
    async fn retrieve_qk_shard(&self) -> Result<QuorumKeyShard, QuorumOperationError>;

    /// Save this node's shard of the quorum key to persistent (safe) storage
    async fn update_qk_shard(&self, shard: QuorumKeyShard) -> Result<(), QuorumOperationError>;

    /// Encrypt the given material using the provided public key, optionally with the provided nonce
    async fn encrypt_material(
        &self,
        public_key: Vec<u8>,
        material: Vec<u8>,
        nonce: Nonce,
    ) -> Result<Vec<u8>, QuorumOperationError>;

    /// Decrypt the specified material utilizing this node's private key
    /// and if a nonce is present, return it
    async fn decrypt_material(
        &self,
        material: Vec<u8>,
    ) -> Result<(Vec<u8>, Nonce), QuorumOperationError>;

    /// Generate a commitment on the epoch changes using the quorum key
    async fn generate_commitment<H: Hasher>(
        &self,
        quorum_key: Vec<u8>,
        epoch: u64,
        previous_hash: H::Digest,
        current_hash: H::Digest,
    ) -> Result<QuorumCommitment<H>, QuorumOperationError>;

    /// Validate the commitment applied on the specified epoch settings
    async fn validate_commitment<H: Hasher>(
        public_key: Vec<u8>,
        commitment: QuorumCommitment<H>,
    ) -> Result<bool, QuorumOperationError>;

    // ==================================================================
    // Common trait logic
    // ==================================================================

    /// Generate num_shards shards of the quorum key, and return the shard pieces.
    /// We take ownership of the quorum key here to help prevent leakage of the key.
    /// By taking ownership, someone needs to explicitely clone it to use it elsewhere
    fn generate_shards(
        quorum_key: [u8; QUORUM_KEY_SIZE],
        f: u8,
    ) -> Result<Vec<QuorumKeyShard>, QuorumOperationError> {
        let num_shards = 3 * f + 1;
        let num_approvals = 2 * f + 1;

        let mut parts = vec![vec![]; num_shards.into()];

        for i in 0..QUORUM_KEY_NUM_PARTS {
            let part: [u8; DATA_SIZE] = quorum_key[i * DATA_SIZE..(i + 1) * DATA_SIZE]
                .try_into()
                .map_err(|_| {
                QuorumOperationError::Sharding(format!(
                    "Unable to convert quorum key slice into SSS shardable component of len {}",
                    DATA_SIZE
                ))
            })?;
            let results = create_shares(&part, num_shards, num_approvals)?;
            for node_i in 0..num_shards {
                let idx: usize = node_i.into();
                match results.get(idx) {
                    None => {
                        return Err(QuorumOperationError::Sharding(format!(
                            "Resulting shards did not have an shard at entry {}",
                            node_i
                        )));
                    }
                    Some(part) => {
                        parts[idx].push(part.clone());
                    }
                }
            }
        }

        let formatted_shards = QuorumKeyShard::build_from_vec_vec_vec(parts)?;
        Ok(formatted_shards)
    }

    /// Reconstruct the quorum key from a specific collection of shards
    fn reconstruct_shards(
        shards: Vec<QuorumKeyShard>,
    ) -> Result<[u8; QUORUM_KEY_SIZE], QuorumOperationError> {
        let mut potential_result = [0u8; QUORUM_KEY_SIZE];
        // there should be QUORUM_KEY_NUM_PARTS in each shard
        for i in 0..QUORUM_KEY_NUM_PARTS {
            let part_i = shards
                .iter()
                .map(|shard| shard.components[i].to_vec())
                .collect::<Vec<_>>();
            let some_key = combine_shares(&part_i)?;
            if let Some(key) = some_key {
                let deconstructed_partial: [u8; DATA_SIZE] = key.try_into().map_err(|_| QuorumOperationError::Sharding(format!("Reconstructing the quorum key resulted in an invalid key length. It _MUST_ be of length {} bytes", DATA_SIZE)))?;
                potential_result[i * DATA_SIZE..(i + 1) * DATA_SIZE]
                    .clone_from_slice(&deconstructed_partial);
            } else {
                return Err(QuorumOperationError::Sharding(
                    "Sharding request to recombine shares resulted in no constructed quorum key"
                        .to_string(),
                ));
            }
        }
        Ok(potential_result)
    }
}

#[cfg(test)]
mod crypto_tests {
    use super::{QuorumCryptographer, QuorumKeyShard, QUORUM_KEY_NUM_PARTS, QUORUM_KEY_SIZE};
    use crate::comms::Nonce;
    use crate::storage::QuorumCommitment;
    use crate::QuorumOperationError;

    use async_trait::async_trait;
    use rand::{seq::IteratorRandom, thread_rng};
    use shamirsecretsharing::SHARE_SIZE;
    use winter_crypto::Hasher;

    struct TestCryptographer;

    #[async_trait]
    impl QuorumCryptographer for TestCryptographer {
        /// Retrieve the public key of this quorum node
        async fn retrieve_public_key(&self) -> Result<Vec<u8>, QuorumOperationError> {
            Ok(vec![])
        }

        /// Retrieve the public key of the Quorum Key
        async fn retrieve_qk_public_key(&self) -> Result<Vec<u8>, QuorumOperationError> {
            Ok(vec![])
        }

        /// Retrieve this node's shard of the quorum key from persistent storage
        async fn retrieve_qk_shard(&self) -> Result<QuorumKeyShard, QuorumOperationError> {
            Ok(QuorumKeyShard {
                components: [[0u8; SHARE_SIZE]; QUORUM_KEY_NUM_PARTS],
            })
        }

        /// Save this node's shard of the quorum key to persistent (safe) storage
        async fn update_qk_shard(
            &self,
            _shard: QuorumKeyShard,
        ) -> Result<(), QuorumOperationError> {
            Ok(())
        }

        /// Encrypt the given material using the provided public key, optionally with the provided nonce
        async fn encrypt_material(
            &self,
            _public_key: Vec<u8>,
            _material: Vec<u8>,
            _nonce: Nonce,
        ) -> Result<Vec<u8>, QuorumOperationError> {
            Ok(vec![])
        }

        /// Decrypt the specified material utilizing this node's private key
        /// and if a nonce is present, return it
        async fn decrypt_material(
            &self,
            _material: Vec<u8>,
        ) -> Result<(Vec<u8>, Nonce), QuorumOperationError> {
            Ok((vec![], 0))
        }

        /// Generate a commitment on the epoch changes using the quorum key
        async fn generate_commitment<H: Hasher>(
            &self,
            _quorum_key: Vec<u8>,
            _epoch: u64,
            _previous_hash: H::Digest,
            _current_hash: H::Digest,
        ) -> Result<QuorumCommitment<H>, QuorumOperationError> {
            unimplemented!();
        }

        /// Validate the commitment applied on the specified epoch settings
        async fn validate_commitment<H: Hasher>(
            _public_key: Vec<u8>,
            _commitment: QuorumCommitment<H>,
        ) -> Result<bool, QuorumOperationError> {
            Ok(false)
        }
    }

    #[test]
    fn test_shard_generation_and_reconstruction() {
        let data: [u8; QUORUM_KEY_SIZE] = [42; QUORUM_KEY_SIZE];
        let shards = TestCryptographer::generate_shards(data.clone(), 2).unwrap();
        assert_eq!(7, shards.len());

        // all shards should be fine
        let construction_ok = TestCryptographer::reconstruct_shards(shards.to_vec());
        assert_eq!(Ok(data), construction_ok);

        // using 5 shards should be fine, given a factor of 2 in f
        let construction_ok = TestCryptographer::reconstruct_shards(shards[0..5].to_vec());
        assert_eq!(Ok(data), construction_ok);

        // using a random subset of shards of size <= 4 should fail
        let mut rng = thread_rng();
        for _ in 1..5 {
            let sample = shards.clone().into_iter().choose_multiple(&mut rng, 4);
            let construction_fail = TestCryptographer::reconstruct_shards(sample);
            assert!(construction_fail.is_err());
        }
    }
}