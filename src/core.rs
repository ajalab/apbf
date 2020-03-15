use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::hash::Hash;
use std::marker::PhantomData;

use bitvec::prelude as bv;

use crate::hash::km::KMHashers;
use crate::hash::{Hashers, Hashes};

/// Age-Partitioned Bloom Filter (APBF) described in Section 5
/// in the original paper.
///
/// APBF consists of a bit array partitioned into slices.
/// Following three parameters determine the property of the structure:
///
/// - `k`: number of slices filled for each insertion
/// - `l`: number of slices besides the `k` slices above.
/// - `m`: number of bits for each slice.
///
/// Therefore the backing bit array is of size `(k + l) * m` bits.
#[derive(Clone)]
pub struct APBF<T, H: Hashers> {
    hashers: H,
    bits: bv::BitVec, // underlying bit array
    k: usize,         // number of slices to fill for each insertion
    l: usize,         // number of slices in addition to k slices
    m: usize,         // number of bits for each slice

    n: u64,   // counter
    p: usize, // position of the first logical slice on a bit vector
    g: u64,   // generation
    _t: PhantomData<T>,
}

impl<T: Hash> APBF<T, KMHashers<RandomState, RandomState>> {
    /// Creates a new APBF instance.
    pub fn new(k: usize, l: usize, m: usize) -> Self {
        Self::with_hashers(k, l, m, KMHashers::new(m as u64))
    }
}

impl<T, H> APBF<T, H>
where
    T: Hash,
    H: Hashers,
{
    pub fn with_hashers(k: usize, l: usize, m: usize, hashers: H) -> Self {
        debug_assert!(k > 0);
        debug_assert!(l > 0);
        debug_assert!(m > 0);

        let g = ((m as f64) * std::f64::consts::LN_2 / (k as f64)) as u64;
        let bits = bv::bitvec![0; (k + l) * m];
        APBF {
            hashers,
            n: 0,
            k,
            l,
            m,
            g,
            bits,
            p: 0,
            _t: PhantomData,
        }
    }

    fn shift(&mut self) {
        let n_slices = self.k + self.l;

        let prev = self.p.checked_sub(1).unwrap_or(n_slices - 1);
        let slice = self.get_slice_mut(prev);
        slice.set_all(false);

        self.p = if self.p == 0 {
            self.l + self.k - 1
        } else {
            self.p - 1
        };
        self.n = 0;
    }

    fn get_slice(&self, i: usize) -> &bv::BitSlice {
        let p = i * self.m;
        &self.bits[p..p + self.m]
    }

    fn get_slice_mut(&mut self, i: usize) -> &mut bv::BitSlice {
        let p = i * self.m;
        &mut self.bits[p..p + self.m]
    }

    /// Inserts a value to the structure.
    pub fn insert<V>(&mut self, value: V)
    where
        V: Borrow<T>,
    {
        let n_slices = self.k + self.l;

        if self.n >= self.g {
            self.shift();
        }

        let hashes = self.hashers.hash(value);
        for i in 0..self.k {
            // Compute position of the i-th logical slice on the bits.
            let pos = self.p + i;
            let pos = pos.checked_sub(n_slices).unwrap_or(pos);

            let slice = self.get_slice_mut(pos);
            let h = hashes.get(pos as u64) as usize;
            slice.set(h, true);
        }

        self.n += 1;
    }

    /// Returns `true` if the structure holds a given value.
    pub fn contains<V>(&self, value: V) -> bool
    where
        V: Borrow<T>,
    {
        let n_slices = self.k + self.l;
        let mut i = self.l;
        let mut prev_count = 0;
        let mut count = 0;

        let hashes = self.hashers.hash(value);
        loop {
            let pos = self.p + i;
            let pos = pos.checked_sub(n_slices).unwrap_or(pos);

            let slice = self.get_slice(pos);
            let h = hashes.get(pos as u64) as usize;
            let hit = *slice.get(h).unwrap();
            if hit {
                count += 1;
                i += 1;
                if prev_count + count == self.k {
                    return true;
                }
            } else {
                if i < self.k {
                    return false;
                }
                i -= self.k;
                prev_count = count;
                count = 0;
            }
        }
    }

    // Returns width of the sliding window, where inserted values
    // are always persisted.
    pub fn window(&self) -> u64 {
        (self.l as u64) * self.g
    }

    // Returns width of the transition zone following the sliding window.
    pub fn slack(&self) -> u64 {
        (self.k as u64) * self.g
    }

    // Returns generation number, which represents how many insertions will
    // cause a shift of logical slices on the underlying bit array.
    pub fn generation(&self) -> u64 {
        self.g
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;

    #[test]
    fn test_insert() {
        let k = 10;
        let l = 20;
        let m = 256;
        let mut apbf = APBF::new(k, l, m);

        let value = 42usize;
        apbf.insert(value);

        for i in 0..k {
            let slice = apbf.get_slice(i);
            assert_eq!(slice.count_ones(), 1);
        }

        for i in k..(k + l) {
            let slice = apbf.get_slice(i);
            assert_eq!(slice.count_ones(), 0);
        }
    }

    #[test]
    fn test_shift() {
        let k = 10;
        let l = 20;
        let m = 256;
        let mut apbf = APBF::new(k, l, m);

        let mut rng = StdRng::from_seed([0u8; 32]);
        for _ in 0..apbf.g {
            apbf.insert(rng.gen::<u64>());
        }
        assert_eq!(apbf.p, 0);

        apbf.insert(rng.gen::<u64>());
        assert_eq!(apbf.p, apbf.k + apbf.l - 1);
    }

    #[test]
    fn test_contains_immediately() {
        let mut apbf = APBF::new(10, 20, 64);
        let value = 42usize;
        apbf.insert(value);
        assert!(apbf.contains(value));
    }

    #[test]
    fn test_contains_in_window() {
        let mut apbf = APBF::new(10, 20, 64);
        let value = 42usize;

        apbf.insert(value);
        let mut rng = StdRng::from_seed([0u8; 32]);
        let w = apbf.window();
        for i in 0..w {
            apbf.insert(rng.gen::<usize>());
            assert!(
                apbf.contains(value),
                "apbf with window of size {} should remember a value after {} insertions",
                w,
                i
            );
        }
    }

    #[test]
    fn test_contains_forget() {
        let mut apbf = APBF::new(10, 20, 64);
        let value = 42usize;

        apbf.insert(value);
        let mut rng = StdRng::from_seed([0u8; 32]);
        let w = apbf.window();
        let s = apbf.slack();

        for _ in 0..(w + s) {
            apbf.insert(rng.gen::<usize>());
        }
        assert!(!apbf.contains(value));
    }
}
