use std::borrow::Borrow;
use std::hash::Hash;

pub trait Hashers {
    type H: Hashes;
    fn hash<Q: Hash, V: Borrow<Q>>(&self, value: V) -> Self::H;
}

pub trait Hashes {
    fn get(&self, i: u64) -> u64;
}

pub mod km;
