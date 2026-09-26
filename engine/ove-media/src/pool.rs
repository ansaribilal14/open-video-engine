//! CPU frame pool with generation-based staleness detection.
//!
//! FRAME_CONTRACT §5.2: "CPU buffers come from a per-decoder pool keyed by
//! (format, size, stride class); `generation` increments on pool recycle — a
//! consumer holding a frame across recycle is a bug the pool can detect."
//!
//! Model: buffers are recycled per key; every recycle bumps the KEY's
//! generation; envelopes carry the generation they were born at. Consumers
//! validate via [`FramePool::check_live`] (debug builds assert); a mismatch
//! is the contract's stale-hold bug, reported as `FrameError::StaleFrame`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::frame::{FrameBytes, FrameError, PixelFormat};

/// Pool identity of a buffer class: pixel format + display size + byte-size
/// class (the contract's "(format, size, stride class)").
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PoolKey {
    pub format: PixelFormat,
    pub width: u32,
    pub height: u32,
    /// Byte-size bucket (e.g. total plane bytes); buckets instead of exact
    /// byte counts so differently-padded allocations still share buffers.
    pub byte_class: u64,
}

impl PoolKey {
    /// stride class from explicit strides (sum, rounded up to 64 bytes).
    pub fn from_strides(format: PixelFormat, width: u32, height: u32, strides: &[usize]) -> Self {
        let total: usize = strides.iter().sum();
        PoolKey {
            format,
            width,
            height,
            byte_class: (total.div_ceil(64) * 64) as u64,
        }
    }
}

/// A buffer handed out by the pool. Drop it via [`FramePool::release`] (the
/// decoder owns this protocol); the generation travels with it.
#[derive(Debug)]
pub struct PooledBuffer {
    pub key: PoolKey,
    pub data: Vec<u8>,
    pub generation: u64,
    strides: Vec<usize>,
}

impl PooledBuffer {
    pub fn into_frame_bytes(mut self) -> FrameBytes {
        self.data.clear();
        FrameBytes {
            data: std::mem::take(&mut self.data),
            strides: self.strides.clone(),
        }
    }

    pub fn strides(&self) -> &[usize] {
        &self.strides
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PoolStats {
    pub live: usize,
    pub pooled: usize,
    pub keys: usize,
    pub recycles: u64,
}

pub struct FramePool {
    max_pooled_per_key: usize,
    free: HashMap<PoolKey, Vec<Vec<u8>>>,
    generations: HashMap<PoolKey, u64>,
    live: usize,
    recycles: u64,
}

impl FramePool {
    pub fn new(max_pooled_per_key: usize) -> Self {
        FramePool {
            max_pooled_per_key,
            free: HashMap::new(),
            generations: HashMap::new(),
            live: 0,
            recycles: 0,
        }
    }

    pub fn current_generation(&self, key: &PoolKey) -> u64 {
        self.generations.get(key).copied().unwrap_or(0)
    }

    /// Acquire a buffer of at least `byte_len`. Reuse bumps the key's
    /// generation (a recycle); fresh allocations do not.
    pub fn acquire(&mut self, key: &PoolKey, byte_len: usize, strides: Vec<usize>) -> PooledBuffer {
        let reused = self
            .free
            .get_mut(key)
            .and_then(|v| v.pop())
            .filter(|b| b.capacity() >= byte_len);
        self.live += 1;
        match reused {
            Some(mut data) => {
                let gen = self.generations.entry(key.clone()).or_insert(0);
                *gen += 1;
                self.recycles += 1;
                data.resize(byte_len, 0);
                PooledBuffer {
                    key: key.clone(),
                    data,
                    generation: *gen,
                    strides,
                }
            }
            None => PooledBuffer {
                key: key.clone(),
                data: vec![0u8; byte_len],
                generation: self.current_generation(key),
                strides,
            },
        }
    }

    /// Reclaim raw bytes from a consumed FrameEnvelope (checked against the
    /// generation tag; capacity kept, content dropped). The reclaim path the
    /// decoder uses after its consumer is done with a frame.
    pub fn release_bytes(
        &mut self,
        key: &PoolKey,
        mut data: Vec<u8>,
        generation: u64,
    ) -> Result<(), FrameError> {
        self.check_live(key, generation)?;
        debug_assert!(
            generation == self.current_generation(key),
            "release of stale buffer: held gen {}, pool at {}",
            generation,
            self.current_generation(key)
        );
        self.live = self.live.saturating_sub(1);
        data.clear();
        let slot = self.free.entry(key.clone()).or_default();
        if slot.len() < self.max_pooled_per_key {
            slot.push(data);
        } // else: dropped (pool cap)
        Ok(())
    }

    /// Return a buffer to the free list (capacity kept, content dropped).
    pub fn release(&mut self, buf: PooledBuffer) {
        debug_assert!(
            buf.generation == self.current_generation(&buf.key),
            "release of stale buffer: held gen {}, pool at {}",
            buf.generation,
            self.current_generation(&buf.key)
        );
        self.live = self.live.saturating_sub(1);
        let slot = self.free.entry(buf.key).or_default();
        if slot.len() < self.max_pooled_per_key {
            let mut data = buf.data;
            data.clear();
            slot.push(data);
        } // else: dropped (pool cap)
    }

    /// Contract's stale-hold detection: a consumer holding a frame across a
    /// recycle sees `FrameError::StaleFrame` here.
    pub fn check_live(&self, key: &PoolKey, generation: u64) -> Result<(), FrameError> {
        let current = self.current_generation(key);
        if current == generation {
            Ok(())
        } else {
            Err(FrameError::StaleFrame {
                held: generation,
                current,
            })
        }
    }

    pub fn stats(&self) -> PoolStats {
        PoolStats {
            live: self.live,
            pooled: self.free.values().map(Vec::len).sum(),
            keys: self.free.len(),
            recycles: self.recycles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(w: u32, h: u32) -> PoolKey {
        PoolKey::from_strides(
            PixelFormat::Yuv420p,
            w,
            h,
            &[w as usize, (w / 2) as usize, (w / 2) as usize],
        )
    }

    #[test]
    fn acquire_reuse_bumps_generation() {
        let mut pool = FramePool::new(4);
        let k = key(320, 240);
        let g0 = pool.acquire(&k, 320 * 240 * 3 / 2, vec![320, 160, 160]);
        assert_eq!(pool.current_generation(&k), 0, "fresh alloc: gen 0");
        pool.release(g0);
        let g1 = pool.acquire(&k, 320 * 240 * 3 / 2, vec![320, 160, 160]);
        assert_eq!(pool.current_generation(&k), 1, "recycle bumps generation");
        assert_eq!(g1.generation, 1);
        assert_eq!(pool.stats().recycles, 1);
        pool.release(g1);
    }

    #[test]
    fn stale_hold_is_detectable() {
        let mut pool = FramePool::new(4);
        let k = key(320, 240);
        let a = pool.acquire(&k, 320 * 240 * 3 / 2, vec![320, 160, 160]);
        pool.release(a); // recycle -> gen 1
        let _b = pool.acquire(&k, 320 * 240 * 3 / 2, vec![320, 160, 160]); // reused at gen 1
                                                                           // a consumer that still holds gen 0 is stale:
        assert_eq!(
            pool.check_live(&k, 0),
            Err(FrameError::StaleFrame {
                held: 0,
                current: 1
            })
        );
        assert!(pool.check_live(&k, 1).is_ok());
    }

    #[test]
    fn release_frees_resources_and_caps_pool() {
        let mut pool = FramePool::new(1);
        let k = key(64, 64);
        let a = pool.acquire(&k, 64 * 64 * 3 / 2, vec![64, 32, 32]);
        let b = pool.acquire(&k, 64 * 64 * 3 / 2, vec![64, 32, 32]);
        assert_eq!(pool.stats().live, 2);
        pool.release(a);
        pool.release(b); // second release exceeds cap=1 -> dropped, not pooled
        let s = pool.stats();
        assert_eq!(s.live, 0);
        assert_eq!(s.pooled, 1, "pool cap respected");
        // different byte class = different key (no cross-size reuse)
        let big = PoolKey::from_strides(PixelFormat::Yuv420p, 128, 128, &[128usize, 64, 64]);
        let c = pool.acquire(&big, 128 * 128 * 3 / 2, vec![128, 64, 64]);
        assert_eq!(pool.stats().live, 1);
        pool.release(c);
        let s = pool.stats();
        assert_eq!(s.keys, 2, "two distinct buffer classes pooled separately");
    }
}
