use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};

use crate::hash::{Hashers, Hashes};

/// A logical set of hash functions derived from two inner hash functions
/// with Kirsch-Mitzenmacher Optimization.
#[derive(Clone)]
pub struct KMHashers<B1, B2>
where
    B1: BuildHasher,
    B2: BuildHasher,
{
    bh1: B1,
    bh2: B2,
    p: u64,
}

impl KMHashers<RandomState, RandomState> {
    pub fn new(p: u64) -> Self {
        KMHashers::with_build_hashers(p, RandomState::new(), RandomState::new())
    }
}

impl<B1, B2> KMHashers<B1, B2>
where
    B1: BuildHasher,
    B2: BuildHasher,
{
    fn with_build_hashers(p: u64, bh1: B1, bh2: B2) -> Self
    where
        B1: BuildHasher,
        B2: BuildHasher,
    {
        Self { p, bh1, bh2 }
    }
}

impl<B1, B2> Hashers for KMHashers<B1, B2>
where
    B1: BuildHasher,
    B2: BuildHasher,
{
    type H = KMHashes;

    fn hash<Q: Hash, V: Borrow<Q>>(&self, value: V) -> KMHashes {
        let value = value.borrow();
        let mut h1 = self.bh1.build_hasher();
        let mut h2 = self.bh2.build_hasher();
        value.hash(&mut h1);
        value.hash(&mut h2);

        KMHashes {
            x1: h1.finish() % self.p,
            x2: h2.finish() % self.p,
            p: self.p,
        }
    }
}

#[derive(Clone, Debug)]
pub struct KMHashes {
    x1: u64,
    x2: u64,
    p: u64,
}

impl Hashes for KMHashes {
    fn get(&self, i: u64) -> u64 {
        // TODO: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/ ?
        (self.x1 + i * self.x2) % self.p
    }
}
