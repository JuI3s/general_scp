use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

pub fn to_hash_value<T>(obj: T) -> u64
where
    T: Hash,
{
    let hasher: [u8; 32] = [0; 32];
    let hr = DefaultHasher::new();
    hr.finish()
}
