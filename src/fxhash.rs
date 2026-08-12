//! # Fast Hasher
//!
//! Symbol tables are hit on every variable read and write, and Rust's default
//! `RandomState` is a cryptographic (SipHash) hasher -- profiling showed it
//! accounting for roughly a quarter of loop runtime.
//!
//! Identifiers are not attacker-controlled here, so we trade collision
//! resistance for speed using the FxHash algorithm (the one rustc itself uses):
//! a multiply-and-rotate over machine words.

use std::hash::{BuildHasherDefault, Hasher};

pub type FxBuildHasher = BuildHasherDefault<FxHasher>;
pub type FxHashMap<K, V> = std::collections::HashMap<K, V, FxBuildHasher>;
pub type FxHashSet<T> = std::collections::HashSet<T, FxBuildHasher>;

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

#[derive(Default, Clone, Copy)]
pub struct FxHasher {
    hash: u64,
}

impl FxHasher {
    #[inline]
    fn add_to_hash(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        while rest.len() >= 8 {
            let (chunk, tail) = rest.split_at(8);
            self.add_to_hash(u64::from_ne_bytes(chunk.try_into().unwrap()));
            rest = tail;
        }
        if rest.len() >= 4 {
            let (chunk, tail) = rest.split_at(4);
            self.add_to_hash(u32::from_ne_bytes(chunk.try_into().unwrap()) as u64);
            rest = tail;
        }
        for &byte in rest {
            self.add_to_hash(byte as u64);
        }
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add_to_hash(i as u64);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add_to_hash(i as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}
