//! Pixel buffers lent to the rasteriser and given back when the strip cache
//! evicts what was drawn in them (m8 04).
//!
//! Why: a strip is megabytes, which macOS malloc serves from its large
//! allocator, and it keeps freed large blocks resident in its free cache.
//! Held `j` drew and dropped a strip-sized buffer hundreds of times a second,
//! so resident memory grew with the churn even after the live set was
//! bounded (`docs/perf/scroll-pixel-memory-budget.txt`). Reusing the buffers
//! stops the churn at its cause; asking malloc to give memory back would
//! only treat the symptom.
//!
//! Every buffer is made at one size, the largest strip asked for since the
//! last flush, so any spare fits any request: a backdrop, an emphasis image
//! cropped in place, a composite.

/// Spare buffers kept between a free and the next draw. The cache gives one
/// back per eviction and takes one per miss, so a few cover the steady state.
const MAX_SPARE: usize = 4;

/// Buffers of one strip size, flushed with the cache.
#[derive(Debug, Default)]
pub struct RasterPool {
    spare: Vec<Vec<u8>>,
    /// Capacity a new buffer is made with: the largest request since the
    /// last flush.
    size: usize,
    /// Buffers made, ever: the count a steady state stops raising.
    allocated: u64,
}

impl RasterPool {
    /// No buffers yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A buffer of `len` zero bytes: a spare when one is big enough, else a
    /// new one made at the pool's strip size.
    pub fn take(&mut self, len: usize) -> Vec<u8> {
        self.size = self.size.max(len);
        let spare = self.spare.iter().position(|spare| spare.capacity() >= len);
        let mut buffer = if let Some(index) = spare {
            self.spare.swap_remove(index)
        } else {
            self.allocated += 1;
            Vec::with_capacity(self.size)
        };
        buffer.clear();
        buffer.resize(len, 0);
        buffer
    }

    /// Take `buffer` back for the next draw. One smaller than today's strip
    /// size, or past [`MAX_SPARE`], is dropped: it could not serve every
    /// request, or nothing is waiting for it.
    pub fn give(&mut self, buffer: Vec<u8>) {
        if buffer.capacity() >= self.size && self.spare.len() < MAX_SPARE {
            self.spare.push(buffer);
        }
    }

    /// Drop every spare and forget the strip size: the cell size, theme or
    /// terminal changed, and strips may be another size now.
    pub fn flush(&mut self) {
        self.spare.clear();
        self.size = 0;
    }

    /// Buffers made since the pool was.
    #[must_use]
    pub fn allocated(&self) -> u64 {
        self.allocated
    }
}

#[cfg(test)]
mod tests {
    use super::RasterPool;

    #[test]
    fn a_given_back_buffer_serves_the_next_take() {
        let mut pool = RasterPool::new();
        let first = pool.take(100);
        assert_eq!((first.len(), pool.allocated()), (100, 1));
        pool.give(first);
        let again = pool.take(60);
        assert_eq!(again.len(), 60);
        assert!(again.iter().all(|byte| *byte == 0), "zeroed");
        assert!(again.capacity() >= 100, "made at the strip size");
        assert_eq!(pool.allocated(), 1, "no new buffer");
    }

    #[test]
    fn a_buffer_smaller_than_the_strip_size_is_not_kept() {
        let mut pool = RasterPool::new();
        let small = pool.take(10);
        let _big = pool.take(100);
        pool.give(small);
        pool.take(100);
        assert_eq!(pool.allocated(), 3);
    }

    #[test]
    fn spares_are_bounded_and_flushed() {
        let mut pool = RasterPool::new();
        let buffers: Vec<Vec<u8>> = (0..10).map(|_| pool.take(8)).collect();
        for buffer in buffers {
            pool.give(buffer);
        }
        for _ in 0..4 {
            pool.take(8);
        }
        assert_eq!(pool.allocated(), 10, "four spares served");
        pool.take(8);
        assert_eq!(pool.allocated(), 11, "only four were kept");
        pool.give(vec![0; 8]);
        pool.flush();
        pool.take(8);
        assert_eq!(pool.allocated(), 12, "a flush drops the spares");
    }
}
