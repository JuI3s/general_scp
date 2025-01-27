use std::{cell::RefCell, hash::Hasher, rc::Rc};

use ct_merkle::{error::InclusionVerifError, inclusion::InclusionProof, CtMerkleTree, RootHash};
use sha2::{Digest, Sha256};

pub type MerkleHash = [u8; 32];
pub type MerkleRoot = RootHash<Sha256>;
pub type MerkleSiblingHashes = Vec<u8>;

pub type MerkleOpResult<T> = std::result::Result<T, MerkleOpError>;

#[derive(Debug, PartialEq)]
pub enum MerkleOpError {
    FailureGenerateInclusionProof,
    InvalidIndex,
    // From the ct-merkle crate.
    /// The proof is malformed, meaning it's either too big for the tree, or its
    /// length is not a multiple of the hash function's digest size.
    MalformedProof,
    /// This root hash does not match the proof's root hash w.r.t. the item
    VerificationFailure,
    InternalTreeError,
}

pub type HMerkleTree = Rc<RefCell<MerkleTree>>;

#[derive(Clone, Debug)]
pub struct MerkleTree {
    mktree: CtMerkleTree<Sha256, MerkleHash>,
    size: usize,
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self {
            mktree: Default::default(),
            size: Default::default(),
        }
    }
}

impl MerkleTree {
    pub fn len(&self) -> usize {
        self.size
    }

    pub fn update(&mut self, val: MerkleHash, idx: usize) -> MerkleOpResult<()> {
        if self.size <= idx {
            return Err(MerkleOpError::InvalidIndex);
        }

        match self.mktree.update(val, idx) {
            Ok(()) => Ok(()),
            Err(_) => Err(MerkleOpError::InternalTreeError),
        }
    }

    pub fn push(&mut self, val: MerkleHash) {
        self.size += 1;
        self.mktree.push(val)
    }

    pub fn root(&self) -> MerkleRoot {
        self.mktree.root()
    }

    pub fn gen_inclusion_proof(&self, idx: usize) -> MerkleOpResult<InclusionProof<Sha256>> {
        // TODO: more error checking maybe?
        if idx >= self.size {
            Err(MerkleOpError::InvalidIndex)
        } else {
            Ok(self.mktree.prove_inclusion(idx))
        }
    }

    pub fn veritfy_inclusion_proof(
        &self,
        val: &MerkleHash,
        idx: usize,
        proof: &InclusionProof<Sha256>,
    ) -> MerkleOpResult<()> {
        if idx >= self.size {
            Err(MerkleOpError::InvalidIndex)
        } else {
            match self.mktree.root().verify_inclusion(val, idx, proof) {
                Ok(()) => Ok(()),
                Err(inclusion_verification_error) => match inclusion_verification_error {
                    InclusionVerifError::MalformedProof => Err(MerkleOpError::MalformedProof),
                    InclusionVerifError::VerificationFailure => {
                        Err(MerkleOpError::VerificationFailure)
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ct_merkle::inclusion::InclusionProof;
    use ct_merkle::CtMerkleTree;

    use sha2::{digest, Sha256};

    #[test]
    fn merkle_tree_create_and_add() {
        let mut mktree = MerkleTree::default();

        let val1: [u8; 32] = [0; 32];
        let val2: [u8; 32] = [1; 32];
        let val3: [u8; 32] = [2; 32];
        mktree.push(val1);
        mktree.push(val2);

        let p_opt = mktree.gen_inclusion_proof(1);

        let p = p_opt.unwrap();
        assert!(mktree.veritfy_inclusion_proof(&val2, 1, &p).is_ok());
        assert!(mktree.veritfy_inclusion_proof(&val1, 1, &p).is_err());
        assert!(mktree.veritfy_inclusion_proof(&val1, 0, &p).is_err());

        mktree.push(val3);
        assert!(mktree.veritfy_inclusion_proof(&val2, 1, &p).is_err());
        let p2_opt = mktree.gen_inclusion_proof(1);
        let p2 = p2_opt.unwrap();
        assert!(mktree.veritfy_inclusion_proof(&val2, 1, &p2).is_ok());

        let bytes: Vec<u8> = p2.as_bytes().into();
        let p_from_bytes = InclusionProof::<Sha256>::from_bytes(bytes);
        assert!(mktree
            .veritfy_inclusion_proof(&val2, 1, &p_from_bytes)
            .is_ok());
    }

    #[test]
    fn merkle_tree_modify_entry() {
        let mut mktree = MerkleTree::default();
        let val1: [u8; 32] = [0; 32];
        let val2: [u8; 32] = [1; 32];
        let val3: [u8; 32] = [2; 32];
        mktree.push(val1);
        mktree.push(val2);

        let p1 = mktree.gen_inclusion_proof(1).unwrap();
        assert!(mktree.veritfy_inclusion_proof(&val2, 1, &p1).is_ok());

        assert!(mktree.update(val3, 1).is_ok());
        assert!(mktree
            .veritfy_inclusion_proof(&val2, 1, &p1)
            .is_err_and(|e| { e == MerkleOpError::VerificationFailure }));

        let p2 = mktree.gen_inclusion_proof(1).unwrap();
        assert!(mktree.veritfy_inclusion_proof(&val3, 1, &p2).is_ok());
    }

    #[test]
    fn ct_merkletree_lib() {
        let mut mktree: CtMerkleTree<Sha256, [u8; 32]> = Default::default();

        let val1: [u8; 32] = [0; 32];
        let val2: [u8; 32] = [1; 32];
        let val3: [u8; 32] = [2; 32];
        mktree.push(val1);
        mktree.push(val2);

        let p: InclusionProof<Sha256> = mktree.prove_inclusion(1);
        assert!(mktree.root().verify_inclusion(&val2, 1, &p).is_ok());
        assert!(mktree.root().verify_inclusion(&val1, 1, &p).is_err());
        assert!(mktree.root().verify_inclusion(&val1, 0, &p).is_err());

        mktree.push(val3);
        assert!(mktree.root().verify_inclusion(&val2, 1, &p).is_err());
        let p2 = mktree.prove_inclusion(1);
        assert!(mktree.root().verify_inclusion(&val2, 1, &p2).is_ok());

        let bytes: Vec<u8> = p2.as_bytes().into();
        let p_from_bytes = InclusionProof::<Sha256>::from_bytes(bytes);
        assert!(mktree
            .root()
            .verify_inclusion(&val2, 1, &p_from_bytes)
            .is_ok());
    }
}
