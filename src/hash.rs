//! A stable hash, the same on every machine and in every build.

/// FNV-1a, 64 bits: short enough to name a file by, and stable across Rust
/// releases, which `std`'s hasher does not promise.
pub fn fnv(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3))
}
