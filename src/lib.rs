#[allow(dead_code)]
use std::alloc::{self, Layout};
use std::{mem::align_of, ptr::NonNull};

const MIN_CAP: usize = 32;

/// Returns `true` iff n is a power of two.
fn is_power_of_two(n: usize) -> bool {
    n != 0 && (n & (n - 1)) == 0
}

/// Returns the smallest power of two greater than or equal to `req`.
fn smallest_power_of_two(req: usize) -> usize {
    let mut n = 1;
    while n < req {
        n *= 2;
    }
    n
}

/// Decides if the buffer needs to be resized in order to accomodate `extra_bytes` more bytes.
fn must_increase_cap(cap: usize, len: usize, extra_bytes: usize) -> Option<usize> {
    if cap - len >= extra_bytes {
        return None;
    }
    let req: usize = len + extra_bytes;
    let new_cap = smallest_power_of_two(req);
    Some(new_cap)
}

/// Returns `true` iff `cap` in the capacity range, is a power of two and is larger than len.
fn cap_len_ok(cap: usize, len: usize) -> bool {
    MIN_CAP <= cap && is_power_of_two(cap) && len <= cap
}

/// Returns `true` iff `hbuf` is `Some(_)` when when `cap` is greater than `MIN_CAP`.
fn hbuf_ok(cap: usize, hbuf: NonNull<u8>) -> bool {
    (cap > MIN_CAP) == (hbuf != NonNull::dangling())
}

/// Byte buffer with an inline buffer of `MIN_CAP` capacity.
/// Transitions to the heap when more capacity is needed.
#[derive(Debug)]
pub struct ByteBuffer {
    cap: usize,
    len: usize,
    hbuf: NonNull<u8>,
    ibuf: [u8; MIN_CAP],
}

impl ByteBuffer {
    /// Invariant that must hold for the buffer to be well formed.
    pub fn invariant(&self) -> bool {
        cap_len_ok(self.cap, self.len) && hbuf_ok(self.cap, self.hbuf)
    }

    /// Creates a new [ByteBuffer].
    pub fn new() -> Self {
        ByteBuffer {
            cap: MIN_CAP,
            len: 0,
            hbuf: NonNull::dangling(),
            ibuf: [0; MIN_CAP],
        }
    }

    /// Returns true iff the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Pushes a new value at the end of the [ByteBuffer].
    pub fn push(&mut self, v: u8) {
        self.resize(1);
        if self.len < MIN_CAP {
            self.ibuf[self.len] = v;
        } else {
            unsafe {
                let ptr = self.hbuf.as_ptr();
                let target = ptr.add(self.len);
                target.write(v);
            };
        }
        self.len += 1;
    }
    /// Pops a value from the end of the [ByteBuffer].
    pub fn pop(&mut self) -> Option<u8> {
        if self.is_empty() {
            return None;
        }
        let result = if self.len <= MIN_CAP {
            self.ibuf[self.len - 1]
        } else {
            unsafe {
                let ptr = self.hbuf.as_ptr();
                let target = ptr.add(self.len);
                target.read()
            }
        };
        self.len -= 1;
        Some(result)
    }

    /// Resizes the [ByteBuffer] to accomodate `extra_bytes`.
    fn resize(&mut self, extra_bytes: usize) {
        if let Some(new_cap) = must_increase_cap(self.cap, self.len, extra_bytes) {
            if self.hbuf == NonNull::dangling() {
                let layout = Layout::from_size_align(new_cap, align_of::<u8>()).unwrap();
                self.hbuf = NonNull::new(unsafe { alloc::alloc(layout) }).unwrap();
            } else {
                let layout = Layout::from_size_align(self.cap, align_of::<u8>()).unwrap();
                self.hbuf = NonNull::new(unsafe {
                    alloc::realloc(self.hbuf.as_ptr() as *mut u8, layout, new_cap)
                })
                .unwrap();
            }
            self.cap = new_cap;
        }
    }
}

/// Returns `true`iff `cap` is the smallest power of two greater than or equal to `req`.
#[cfg(any(test, kani))]
fn is_smallest_power_of_two(cap: usize, req: usize) -> bool {
    is_power_of_two(cap) && (cap >= req) && (cap == 1 || (cap / 2) < req)
}

#[cfg(test)]
mod bolero_tests {
    use super::*;
    use bolero::check;

    #[test]
    fn test_is_smallest_power_of_two() {
        // Test known cases
        assert!(is_smallest_power_of_two(1, 1));
        assert!(is_smallest_power_of_two(2, 2));
        assert!(is_smallest_power_of_two(4, 3));
        assert!(is_smallest_power_of_two(4, 4));
        assert!(is_smallest_power_of_two(8, 5));
        assert!(is_smallest_power_of_two(8, 7));
        assert!(is_smallest_power_of_two(8, 8));
        assert!(is_smallest_power_of_two(16, 9));

        // Test negative cases
        assert!(!is_smallest_power_of_two(3, 2)); // not a power of two
        assert!(!is_smallest_power_of_two(4, 5)); // too small for req
        assert!(!is_smallest_power_of_two(8, 4)); // not smallest (4 would work)
        assert!(!is_smallest_power_of_two(0, 0)); // zero is not valid
        assert!(!is_smallest_power_of_two(16, 8)); // not smallest (8 would work)

        // Test some larger values
        assert!(is_smallest_power_of_two(32, 17));
        assert!(is_smallest_power_of_two(64, 33));
        assert!(is_smallest_power_of_two(128, 65));
    }

    #[test]
    // #[cfg_attr(kani, kani::proof)]
    fn fuzz_smallest_power_of_two() {
        check!().with_type::<usize>().for_each(|req| {
            let cap = smallest_power_of_two(*req);
            assert!(is_smallest_power_of_two(cap, *req));
        });
    }

    #[test]
    // #[cfg_attr(kani, kani::proof)]
    fn fuzz_must_increase_cap() {
        check!()
            .with_type::<(usize, usize, usize)>()
            .cloned()
            .for_each(|(cap, len, extra_bytes)| {
                // Rejection sampling for the assumption
                if !cap_len_ok(cap, len) {
                    return;
                }

                match must_increase_cap(cap, len, extra_bytes) {
                    None => {
                        assert!(cap - len >= extra_bytes);
                    }
                    Some(new_cap) => {
                        assert!(cap - len < extra_bytes);
                        assert!(cap < new_cap);
                        assert!(is_smallest_power_of_two(new_cap, len + extra_bytes));
                    }
                }
            });
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn smallest_power_of_two_harness() {
        let req = kani::any();
        let cap = smallest_power_of_two(req);
        assert!(is_smallest_power_of_two(cap, req));
    }

    #[kani::proof]
    fn must_increase_cap_harness() {
        let cap = kani::any();
        let len = kani::any();
        kani::assume(cap_len_ok(cap, len));
        let extra_bytes = kani::any();
        match must_increase_cap(cap, len, extra_bytes) {
            None => {
                assert!(cap - len >= extra_bytes);
            }
            Some(new_cap) => {
                assert!(cap - len < extra_bytes);
                assert!(cap < new_cap);
                assert!(is_smallest_power_of_two(new_cap, len + extra_bytes));
            }
        }
    }
}
