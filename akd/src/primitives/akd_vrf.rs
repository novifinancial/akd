// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under both the MIT license found in the
// LICENSE-MIT file in the root directory of this source tree and the Apache
// License, Version 2.0 found in the LICENSE-APACHE file in the root directory
// of this source tree.

//! Includes the trait and an implementation of it to access secure data for the VRF.
use vrf::{openssl::ECVRF, VRF};
use vrf::openssl::{Error, CipherSuite};

use crate::errors::{VRFStorageError, HardCodedVRFStorageError};
/// A trait to get public and secret key for the VRF
pub trait VRFKeyStorage {
    /// The type of the public key
    type PK: Clone;
    /// The type of the secret key
    type SK: Clone;
    /// Type of the VRF
    type VRF: VRF<Self::PK, Self::SK>;

    /// Gets the public key for the VRF
    fn get_public_key() -> Result<Self::PK, VRFStorageError>;

    /// Gets the secret key for the VRF
    fn get_secret_key() -> Result<Self::SK, VRFStorageError>;

    /// Generates the VRF proof
    fn prove(sk: Self::SK, alpha: &[u8]) -> Result<Vec<u8>, VRFStorageError>;
    
    /// Generates the VRF proof
    fn verify(y: Self::PK, pi: &[u8], alpha: &[u8]) -> Result<Vec<u8>, VRFStorageError>;

    /// Generates hash for a VRF
    fn vrf_to_hash(pi: &[u8], alpha: &[u8]) -> Result<Vec<u8>, VRFStorageError>;
    
}

pub struct NoLifetimeECVRF {
    vrf: ECVRF,
}

impl NoLifetimeECVRF {
    fn new() -> Result<Self, vrf::openssl::Error> {
        let vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI)?;
        Ok(Self {
            vrf
        })
    }

    fn derive_public_key(&mut self, secret_key: Vec<u8>) -> Result<Vec<u8>, vrf::openssl::Error> {
        self.vrf.derive_public_key(&secret_key)
    }

    fn proof_to_hash(&mut self, pi: &[u8]) -> Result<Vec<u8>, Error> {
        self.vrf.proof_to_hash(pi)   
    }

}

impl VRF<Vec<u8>, Vec<u8>> for NoLifetimeECVRF {
    type Error = Error;
    
    fn prove(&mut self, x: Vec<u8>, alpha: &[u8]) -> Result<Vec<u8>, Self::Error> {
        self.vrf.prove(x.as_slice(), alpha)
    }

    fn verify(&mut self, y: Vec<u8>, pi: &[u8], alpha: &[u8]) -> Result<Vec<u8>, Self::Error> {
        self.vrf.verify(y.as_slice(), pi, alpha)
    }
}

/// This is a version of VRFKeyStorage for testing purposes, which uses the example from the VRF crate.
pub struct HardCodedVRFKeyStorage {
    //const KEY_MATERIAL: &str = "c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721";
    
}

impl HardCodedVRFKeyStorage {
    fn get_secret_key_helper() -> Result<Vec<u8>, HardCodedVRFStorageError> {
        Ok(hex::decode("c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721")?)
    }

    fn get_public_key_helper() -> Result<Vec<u8>, HardCodedVRFStorageError> {
        let mut vrf = NoLifetimeECVRF::new()?;
        let sk = Self::get_secret_key_helper()?;
        Ok(vrf.derive_public_key(sk)?)
    }
}

impl VRFKeyStorage for HardCodedVRFKeyStorage {
    type PK = Vec<u8>;
    type SK = Vec<u8>;
    type VRF = NoLifetimeECVRF;
    
    fn get_secret_key() -> Result<Vec<u8>, VRFStorageError> {
        Ok(Self::get_secret_key_helper()?)
    }

    fn get_public_key() -> Result<Vec<u8>, VRFStorageError> {
        Ok(Self::get_public_key_helper()?)    
    }

    fn prove(sk: Self::SK, alpha: &[u8]) -> Result<Vec<u8>, VRFStorageError> {
        let mut vrf = NoLifetimeECVRF::new()?;
        Ok(vrf.prove(sk, alpha)?)
    }

    fn verify(pk: Self::PK, pi: &[u8], alpha: &[u8]) -> Result<Vec<u8>, VRFStorageError> {
        let mut vrf = NoLifetimeECVRF::new()?;
        Ok(vrf.verify(pk, pi, alpha)?)
    }

    fn vrf_to_hash(pi: &[u8], _alpha: &[u8]) -> Result<Vec<u8>, VRFStorageError> {
        let mut vrf = NoLifetimeECVRF::new()?;
        Ok(vrf.proof_to_hash(pi)?)
    }
    
}
