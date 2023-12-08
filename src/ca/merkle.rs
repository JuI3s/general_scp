use std::{collections::hash_map::DefaultHasher, fmt, hash::Hasher};

use ct_merkle::{CtMerkleTree, inclusion::InclusionProof, error::InclusionVerifError};
use sha2::{Digest, Sha256};

use super::cell::Cell;

pub type MerkleHash = [u8; 32];

pub type MerkleOpResult<T> = std::result::Result<T, MerkleOpError>;

#[derive(Debug)]
pub enum MerkleOpError {
    FailureGenerateInclusionProof,
    InvalidIndex,
    // From the ct-merkle crate.

    /// The proof is malformed, meaning it's either too big for the tree, or its length is not a
    /// multiple of the hash function's digest size.
    MalformedProof,
    /// This root hash does not match the proof's root hash w.r.t. the item
    VerificationFailure,    

}

pub struct MerkleTree{
    mktree: CtMerkleTree<Sha256, MerkleHash>,
    size: usize,
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self { mktree: Default::default(), size: Default::default() }
    }
}

impl MerkleTree {
    pub fn len(&self) -> usize {
        self.size
    }

    pub fn push(&mut self, val: MerkleHash) {
        self.size += 1;
        self.mktree.push(val)
    }

    pub fn gen_inclusion_proof(&self, idx: usize) -> MerkleOpResult< InclusionProof<Sha256>> {    
        // TODO: more error checking maybe?
        if idx >= self.size {
            Err(MerkleOpError::InvalidIndex)
        } else {
            Ok(self.mktree.prove_inclusion(idx))
        }
    }

    pub fn veritfy_inclusion_proof(&self, val: &MerkleHash, idx: usize, proof: &InclusionProof<Sha256>)  -> MerkleOpResult<()> {
        if idx >= self.size {
            Err(MerkleOpError::InvalidIndex)
        } else {
            match self.mktree.root().verify_inclusion(val, idx, proof) {
                Ok(()) => Ok(()),
                Err(inclusion_verification_error) => {
                    match inclusion_verification_error  {
                        InclusionVerifError::MalformedProof => Err(MerkleOpError::MalformedProof),
                        InclusionVerifError::VerificationFailure => Err(MerkleOpError::VerificationFailure),
                    }
                }

            }
        }
    }
    
    
}


#[cfg(test)]
mod tests {
    use super::*;

    use ct_merkle::inclusion::InclusionProof;
    use sha2::digest::core_api::{CtVariableCoreWrapper, CoreWrapper};
    use sha2::{Sha256, digest};
    use ct_merkle::CtMerkleTree;
    use typenum::U0;
    use digest::consts::{U28, U32, U48, U64};

    use crate::ca::cell::Cell;
    use crate::utils::hash::to_hash_value;


    #[test]
    fn merkle_tree() {
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
        assert!(mktree.veritfy_inclusion_proof(&val2, 1, &p_from_bytes).is_ok());

    }

    #[test]
    fn ct_merkletree_lib() {
        let mut mktree: CtMerkleTree<Sha256, [u8; 32]>  = Default::default();

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
        assert!(mktree.root().verify_inclusion(&val2, 1, &p_from_bytes).is_ok());


    }
    

    #[test]
    fn merkletree_lib() {
        let cell1 = Cell::new_value_cell("value1");
        let cell2 = Cell::new_value_cell("value2");
        let cell3 = Cell::new_value_cell("value3");
        let cell4 = Cell::new_value_cell("value4");
        let cell5 = Cell::new_value_cell("value5");
        let indices = vec![to_hash_value(cell4.to_owned())];
        // let leaves: Vec<u64> = vec![cell1, cell2, cell3, cell4,cell5].iter().map(|cell|{to_hash_value(cell)}).collect();

        // let tree = MerkleTree::new([[1;32], [2;32]]);
        let val = [0; 32];
        let val2 = [1; 32];


        // match
        //  MerkleTree::<E, Sha256Hasher, VecStore<E>>::new([val, val2]) {
        // Ok(_) => {},
        // Err(err) => {
        // println!("Merkle Error.");
        // println!("{}", err);
        // assert!(false);
        // },
        // }

        // let tree = MerkleTree::build_merkle_tree(&leaves);
        // let proof = tree.build_proof(indices);
    }
}
