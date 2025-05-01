# kani-harnesss-based-verification

Tutorial about using bolero and Kani for harness-based testing and verification

The code in its different versions is found in different branches of the repo.

# ByteBuffer

A memory-efficient byte buffer implementation in Rust that combines stack and heap allocation strategies.
Stores `u8` to keep things simple. The goal is to illustrate how to use Bolero and Kani together in a harness-based verification approach.

Similar to:
- [`SmallVec`](https://docs.rs/smallvec/latest/smallvec/),
- [compact_str](https://docs.rs/compact_str/latest/compact_str/struct.CompactString.html)

## Features

- Starts with an inline buffer of 32 bytes (MIN_CAP) stored on the stack
- Automatically transitions to heap allocation when more capacity is needed
- Maintains power-of-two capacity sizing for efficient memory management
- Supports basic operations like push and pop
- Fuzzed using Bolero
- Formally verified using Kani Rust Verifier

## Usage

```rust
let mut buffer = ByteBuffer::new();

// Push bytes to the buffer
buffer.push(42).unwrap();

// Pop bytes from the buffer
let value = buffer.pop(); // Returns Option<u8>
```

## Implementation Details

- Initial capacity: 32 bytes (stored inline)
- Maximum capacity: 2^48 bytes
- Maximum request size: 2^63 bytes
- Automatically resizes when capacity is exceeded
- Maintains power-of-two sizing for all capacity increases

## Error Handling

- `ByteBufferError::CapacityOverflow`: When requested capacity exceeds maximum limits

## Verification

The implementation includes formal verification proofs using Kani, covering:
- Power of two calculations
- Capacity management
- Push/pop operations
- Buffer invariants
- Memory safety

## Performance Characteristics

- Small buffers (≤32 bytes) use stack allocation
- Larger buffers automatically transition to heap allocation
- Power-of-two sizing ensures efficient memory usage
- Amortized O(1) push and pop operations

## License

See `LICENSE` file in the root folder.

## Contributing

Send PRs.
