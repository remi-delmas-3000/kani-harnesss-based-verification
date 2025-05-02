#[allow(dead_code)]
use std::alloc::{self, Layout};
use std::{mem::align_of, ptr::NonNull};
use std::fmt;


const MIN_CAP: usize = 32;
const MAX_CAP: usize = 1usize << 48;
const MAX_REQ: usize = 1 << 63;

/// Returns `true` iff n is a power of two.
fn is_power_of_two(n: usize) -> bool {
    n != 0 && (n & (n - 1)) == 0
}

/// Returns the smallest power of two greater than or equal to `req`.
fn smallest_power_of_two(req: usize) -> usize {
    assert!(req <= MAX_REQ);
    if req < 2 {
        return 1;
    }
    let mut n = req;
    n -= 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n |= n >> 32;
    n += 1;
    n
}

/// Decides if the buffer needs to be resized in order to accomodate `extra_bytes` more bytes.
fn must_increase_cap(
    cap: usize,
    len: usize,
    extra_bytes: usize,
) -> Result<Option<usize>, ByteBufferError> {
    assert!(cap_len_ok(cap, len));
    if cap - len >= extra_bytes {
        return Result::Ok(None);
    }
    if usize::MAX - len < extra_bytes {
        return Result::Err(ByteBufferError::CapacityOverflow);
    }
    let req = len + extra_bytes;
    if req > MAX_REQ {
        return Result::Err(ByteBufferError::CapacityOverflow);
    }
    let new_cap = smallest_power_of_two(req);
    if new_cap > MAX_CAP {
        return Result::Err(ByteBufferError::CapacityOverflow);
    }
    Result::Ok(Some(new_cap))
}

/// Returns `true` iff `cap` in the capacity range, is a power of two and is larger than len.
fn cap_len_ok(cap: usize, len: usize) -> bool {
    MIN_CAP <= cap && cap <= MAX_CAP && is_power_of_two(cap) && len <= cap
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

pub enum ByteBufferError {
    CapacityOverflow,
    AllocationFailed,
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
    pub fn push(&mut self, v: u8) -> Result<(), ByteBufferError> {
        self.resize(1)?;
        if self.len < MIN_CAP {
            self.ibuf[self.len] = v;
        } else {
            unsafe {
                let ptr = self.hbuf.as_ptr();
                let target = ptr.add(self.len - MIN_CAP);
                target.write(v);
            };
        }
        self.len += 1;
        Result::Ok(())
    }

    /// Pops a value from the end of the [ByteBuffer].
    pub fn pop(&mut self) -> Option<u8> {
        if self.is_empty() {
            return None;
        }
        self.len -= 1; // Decrease length first
        let result = if self.len < MIN_CAP {
            self.ibuf[self.len] // Now using the new length
        } else {
            unsafe {
                let ptr = self.hbuf.as_ptr();
                let target = ptr.add(self.len - MIN_CAP);
                target.read()
            }
        };
        Some(result)
    }

    /// Resizes the [ByteBuffer] to accomodate `extra_bytes`.
    fn resize(&mut self, extra_bytes: usize) -> Result<(), ByteBufferError> {
        match must_increase_cap(self.cap, self.len, extra_bytes)? {
            None => Result::Ok(()),
            Some(new_cap) => {
                let raw_ptr = if self.hbuf == NonNull::dangling() {
                    let layout =
                        Layout::from_size_align(new_cap - MIN_CAP, align_of::<u8>()).unwrap();
                    unsafe { alloc::alloc(layout) }
                } else {
                    // the old layout
                    let layout =
                        Layout::from_size_align(self.cap - MIN_CAP, align_of::<u8>()).unwrap();
                    unsafe {
                        alloc::realloc(self.hbuf.as_ptr() as *mut u8, layout, new_cap - MIN_CAP)
                    }
                };
                self.hbuf = match NonNull::new(raw_ptr) {
                    None => {
                        return Result::Err(ByteBufferError::AllocationFailed);
                    }
                    Some(ptr) => ptr,
                };
                self.cap = new_cap;
                Result::Ok(())
            }
        }
    }
}

impl Drop for ByteBuffer {
    fn drop(&mut self) {
        if self.hbuf != NonNull::dangling() {
            let layout = Layout::from_size_align(self.cap - MIN_CAP, align_of::<u8>()).unwrap();
            unsafe {
                alloc::dealloc(self.hbuf.as_ptr(), layout);
            }
        }
    }
}

impl fmt::Display for ByteBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for i in 0..self.len {
            if i > 0 {
                write!(f, ", ")?;
            }
            let value = if i < MIN_CAP {
                self.ibuf[i]
            } else {
                unsafe {
                    let ptr = self.hbuf.as_ptr();
                    ptr.add(i - MIN_CAP).read()
                }
            };
            write!(f, "{}", value)?;
        }
        write!(f, "]")
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
    #[cfg_attr(kani, kani::proof)]
    fn fuzz_smallest_power_of_two() {
        check!().with_type::<usize>().for_each(|req| {
            // Rejection sampling for the assumption
            if *req > MAX_REQ {
                return;
            }

            let cap = smallest_power_of_two(*req);
            assert!(is_smallest_power_of_two(cap, *req));
        });
    }

    #[test]
    #[cfg_attr(kani, kani::proof)]
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
                    Result::Ok(None) => {
                        assert!(cap - len >= extra_bytes);
                    }
                    Result::Ok(Some(new_cap)) => {
                        assert!(cap - len < extra_bytes);
                        assert!(cap < new_cap && new_cap <= MAX_CAP);
                        assert!(is_smallest_power_of_two(new_cap, len + extra_bytes));
                    }
                    Result::Err(_) => {}
                }
            });
    }

    #[test]
    fn fuzz_push_pop() {
        check!().for_each(|data: &[u8]| {
            let mut buf = ByteBuffer::new();
            let mut reference_data = Vec::new();

            // Push all bytes
            for &b in data {
                if let Err(_) = buf.push(b) {
                    return; // Stop if we hit an error (like capacity overflow)
                }
                reference_data.push(b);
            }

            // Pop and verify in reverse order
            while let Some(b) = buf.pop() {
                assert_eq!(Some(b), reference_data.pop());
            }

            assert!(buf.is_empty());
            assert!(reference_data.is_empty());
        });
    }

    #[test]
    fn fuzz_push_capacity() {
        check!().for_each(|data: &[u8]| {
            let mut buf = ByteBuffer::new();

            for &b in data {
                match buf.push(b) {
                    Ok(_) => {
                        assert!(buf.invariant(), "Buffer invariant violated");
                        assert!(buf.len <= buf.cap, "Length exceeded capacity");
                    }
                    Err(_) => return,
                }
            }
        });
    }

    #[test]
    fn fuzz_resize() {
        check!().for_each(|data: &[u8]| {
            if data.len() < 2 {
                return;
            }
            let size = (data[0] as usize) % 1024; // Use first byte as size
            let extra = (data[1] as usize) % 1024; // Use second byte as extra

            let mut buf = ByteBuffer::new();

            // First fill buffer to size
            for i in 0..size {
                if let Err(_) = buf.push(i as u8) {
                    return;
                }
            }

            // Try to resize
            match buf.resize(extra) {
                Ok(_) => {
                    assert!(buf.invariant(), "Buffer invariant violated");
                    assert!(
                        buf.cap >= buf.len + extra,
                        "Capacity insufficient after resize"
                    );
                }
                Err(_) => (),
            }
        });
    }

    #[test]
    fn fuzz_transition_to_heap() {
        check!().for_each(|data: &[u8]| {
            let mut buf = ByteBuffer::new();
            let mut reference_data = Vec::new();

            for &b in data {
                if let Err(_) = buf.push(b) {
                    return;
                }
                reference_data.push(b);

                // Create a copy of the buffer to verify
                let mut test_buf = ByteBuffer::new();
                for &v in &reference_data {
                    let _ = test_buf.push(v);
                }

                // Verify values can be popped correctly
                for &expected in reference_data.iter().rev() {
                    assert_eq!(test_buf.pop(), Some(expected));
                }
                assert!(test_buf.is_empty());
            }
        });
    }
}

#[cfg(kani)]
mod proofs {

    use super::*;

    unsafe extern "C" {
        /// A CBMC primitive to havoc a slice of size `len` of a pointer to `u8`.
        unsafe fn __cbmc_havoc_object_u8(ptr: *mut u8, len: usize) -> u8;
    }

    impl kani::Arbitrary for ByteBuffer {
        /// Generates a nondet instance of ByteBuffer that satisfies the invariant
        /// Will be used to check all methods from an arbitrary state
        fn any() -> Self {
            let cap = kani::any_where(|cap| MIN_CAP <= *cap && *cap <= MAX_CAP);
            let len = kani::any();
            kani::assume(cap_len_ok(cap, len));
            let mut ibuf = [0; MIN_CAP];
            for i in 0..MIN_CAP {
                if i < len {
                    ibuf[i] = kani::any();
                }
            }
            let hbuf = if cap > MIN_CAP {
                let layout = Layout::from_size_align(cap - MIN_CAP, align_of::<u8>()).unwrap();
                let ptr = unsafe { alloc::alloc(layout) };
                kani::assume(!ptr.is_null());
                // Only havoc elements that are actually used (len - MIN_CAP)
                if len > MIN_CAP {
                    unsafe {
                        __cbmc_havoc_object_u8(ptr, len - MIN_CAP);
                    }
                }
                NonNull::new(ptr).unwrap()
            } else {
                NonNull::dangling()
            };
            let res = ByteBuffer {
                cap,
                len,
                hbuf,
                ibuf,
            };
            assert!(res.invariant());
            res
        }
    }

    #[kani::proof]
    fn new_harness() {
        let buf = ByteBuffer::new();
        assert!(buf.invariant());
    }

    #[kani::proof]
    fn resize_new_harness() {
        let mut buf = ByteBuffer::new();
        let extra_bytes = kani::any();
        let result = buf.resize(extra_bytes);
        match result {
            Result::Ok(()) => {
                assert!(buf.invariant());
            }
            _ => {}
        }
    }

    #[kani::proof]
    fn resize_any_harness() {
        let mut buf: ByteBuffer = kani::any();
        let extra_bytes = kani::any();
        match buf.resize(extra_bytes) {
            Result::Ok(()) => {
                assert!(buf.invariant());
            }
            _ => {}
        }
    }

    #[kani::proof]
    fn push_any_harness() {
        let mut buf: ByteBuffer = kani::any();
        let value = kani::any();
        match buf.push(value) {
            Result::Ok(()) => {
                assert!(buf.invariant());
            }
            _ => {}
        }
    }

    #[kani::proof]
    fn pop_harness() {
        let mut buf = ByteBuffer::new();
        match buf.resize(kani::any()) {
            Result::Ok(()) => {
                let _ = buf.pop();
                assert!(buf.invariant());
            }
            _ => {}
        }
    }

    #[kani::proof]
    fn pop_any_harness() {
        let mut buf: ByteBuffer = kani::any();
        let _ = buf.pop();
        assert!(buf.invariant());
    }

    #[kani::proof]
    fn push_pop_any_harness() {
        let mut buf: ByteBuffer = kani::any();
        let value = kani::any();
        match buf.push(value) {
            Result::Ok(()) => match buf.pop() {
                Some(result) => {
                    assert!(result == value);
                }
                None => {}
            },
            _ => {}
        }
    }
}
