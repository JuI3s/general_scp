use std::{
    collections::hash_map::DefaultHasher,
    f32::consts::E,
    hash::{Hash, Hasher},
};

pub fn to_hash_value<T>(obj: T) -> u64
where
    T: Hash,
{
    let mut hasher: [u8; 32] = [0; 32];
    let mut hr = DefaultHasher::new();
    hr.finish()
}
