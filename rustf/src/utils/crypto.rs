//! Cryptographic utilities for RustF framework
//!
//! This module provides cryptographic functions commonly used in web development,
//! including various hash functions, password utilities, and simple encryption/decryption operations.
//!
//! # Security Notice
//! - MD5 and SHA1 are considered cryptographically broken for security purposes
//! - Use SHA256 or higher for security-sensitive applications
//! - These functions are provided for compatibility and non-security use cases

use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Generate a hash of the input string
///
/// Uses a fast hash function suitable for general purposes (not cryptographically secure).
///
/// # Arguments
/// * `input` - String to hash
///
/// # Example
/// ```rust,ignore
/// let hash = hash_string("hello world");
/// println!("Hash: {}", hash);
/// ```
pub fn hash_string(input: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

/// Generate a hash of bytes
///
/// # Arguments
/// * `input` - Bytes to hash
///
/// # Example
/// ```rust,ignore
/// let hash = hash_bytes(&[1, 2, 3, 4, 5]);
/// println!("Hash: {}", hash);
/// ```
pub fn hash_bytes(input: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

/// Simple XOR encryption/decryption
///
/// WARNING: This is NOT cryptographically secure and should only be used
/// for obfuscation or educational purposes. Use proper encryption libraries
/// for sensitive data.
///
/// # Arguments
/// * `data` - Data to encrypt/decrypt
/// * `key` - Key for XOR operation
///
/// # Example
/// ```rust,ignore
/// let encrypted = xor_encrypt("hello", "key");
/// let decrypted = xor_decrypt(&encrypted, "key");
/// assert_eq!(decrypted, "hello");
/// ```
pub fn xor_encrypt(data: &str, key: &str) -> Vec<u8> {
    xor_bytes(data.as_bytes(), key.as_bytes())
}

/// Simple XOR decryption to string
///
/// # Arguments
/// * `data` - Encrypted bytes
/// * `key` - Key for XOR operation
///
/// # Example
/// ```rust,ignore
/// let encrypted = xor_encrypt("hello", "key");
/// let decrypted = xor_decrypt(&encrypted, "key");
/// assert_eq!(decrypted, "hello");
/// ```
pub fn xor_decrypt(data: &[u8], key: &str) -> String {
    let decrypted_bytes = xor_bytes(data, key.as_bytes());
    String::from_utf8_lossy(&decrypted_bytes).to_string()
}

/// XOR operation on bytes
///
/// # Arguments
/// * `data` - Data bytes
/// * `key` - Key bytes
fn xor_bytes(data: &[u8], key: &[u8]) -> Vec<u8> {
    if key.is_empty() {
        return data.to_vec();
    }

    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

/// Generate a simple checksum for data integrity
///
/// This is a simple checksum, not suitable for security purposes.
/// Use for data integrity checks in non-security contexts.
///
/// # Arguments
/// * `data` - Data to checksum
///
/// # Example
/// ```rust,ignore
/// let checksum = simple_checksum("important data");
/// println!("Checksum: {}", checksum);
/// ```
pub fn simple_checksum(data: &str) -> u32 {
    data.bytes().map(|b| b as u32).sum()
}

/// Constant-time string comparison
///
/// Compares two strings in constant time to prevent timing attacks.
/// Always use this for comparing sensitive data like passwords or tokens.
///
/// # Arguments
/// * `a` - First string
/// * `b` - Second string
///
/// # Example
/// ```rust,ignore
/// let is_equal = constant_time_compare("secret", "secret");
/// assert!(is_equal);
/// ```
pub fn constant_time_compare(a: &str, b: &str) -> bool {
    constant_time_compare_bytes(a.as_bytes(), b.as_bytes())
}

/// Constant-time byte comparison
///
/// # Arguments
/// * `a` - First byte slice
/// * `b` - Second byte slice
pub fn constant_time_compare_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}

/// Generate a simple obfuscated string
///
/// This is basic obfuscation, not encryption. Don't use for sensitive data.
///
/// # Arguments
/// * `input` - String to obfuscate
/// * `offset` - Character offset for obfuscation
///
/// # Example
/// ```rust,ignore
/// let obfuscated = obfuscate_string("hello", 3);
/// let deobfuscated = deobfuscate_string(&obfuscated, 3);
/// assert_eq!(deobfuscated, "hello");
/// ```
pub fn obfuscate_string(input: &str, offset: u8) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                let base = if c.is_ascii_lowercase() { b'a' } else { b'A' };
                let shifted = ((c as u8 - base + offset) % 26) + base;
                shifted as char
            } else {
                c
            }
        })
        .collect()
}

/// Deobfuscate a string
///
/// # Arguments
/// * `input` - Obfuscated string
/// * `offset` - Character offset used for obfuscation
pub fn deobfuscate_string(input: &str, offset: u8) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                let base = if c.is_ascii_lowercase() { b'a' } else { b'A' };
                let shifted = ((c as u8 - base + 26 - (offset % 26)) % 26) + base;
                shifted as char
            } else {
                c
            }
        })
        .collect()
}

// Cryptographic Hash Functions

/// Generate MD5 hash of input string
///
/// **Security Warning**: MD5 is cryptographically broken and should not be used
/// for security purposes. Use SHA256 or higher for security applications.
///
/// # Arguments
/// * `input` - String to hash
///
/// # Example
/// ```rust,ignore
/// let hash = md5("hello world");
/// assert_eq!(hash, "5eb63bbbe01eeed093cb22bb8f5acdc3");
/// ```
pub fn md5(input: &str) -> String {
    md5_bytes(input.as_bytes())
}

/// Generate MD5 hash of byte slice
///
/// **Security Warning**: MD5 is cryptographically broken and should not be used
/// for security purposes.
///
/// # Arguments
/// * `input` - Bytes to hash
///
/// # Example
/// ```rust,ignore
/// let hash = md5_bytes(b"hello world");
/// assert_eq!(hash, "5eb63bbbe01eeed093cb22bb8f5acdc3");
/// ```
pub fn md5_bytes(input: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

/// Generate SHA1 hash of input string
///
/// **Security Warning**: SHA1 is cryptographically broken and should not be used
/// for security purposes. Use SHA256 or higher for security applications.
///
/// # Arguments
/// * `input` - String to hash
///
/// # Example
/// ```rust,ignore
/// let hash = sha1("hello world");
/// assert_eq!(hash, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
/// ```
pub fn sha1(input: &str) -> String {
    sha1_bytes(input.as_bytes())
}

/// Generate SHA1 hash of byte slice
///
/// **Security Warning**: SHA1 is cryptographically broken and should not be used
/// for security purposes.
///
/// # Arguments
/// * `input` - Bytes to hash
///
/// # Example
/// ```rust,ignore
/// let hash = sha1_bytes(b"hello world");
/// assert_eq!(hash, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
/// ```
pub fn sha1_bytes(input: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

/// Generate SHA256 hash of input string
///
/// This is a secure hash function suitable for security applications.
///
/// # Arguments
/// * `input` - String to hash
///
/// # Example
/// ```rust,ignore
/// let hash = sha256("hello world");
/// assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
/// ```
pub fn sha256(input: &str) -> String {
    sha256_bytes(input.as_bytes())
}

/// Generate SHA256 hash of byte slice
///
/// This is a secure hash function suitable for security applications.
///
/// # Arguments
/// * `input` - Bytes to hash
///
/// # Example
/// ```rust,ignore
/// let hash = sha256_bytes(b"hello world");
/// assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
/// ```
pub fn sha256_bytes(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

/// Generate SHA512 hash of input string
///
/// This is a secure hash function suitable for security applications.
///
/// # Arguments
/// * `input` - String to hash
///
/// # Example
/// ```rust,ignore
/// let hash = sha512("hello world");
/// println!("SHA512: {}", hash);
/// ```
pub fn sha512(input: &str) -> String {
    sha512_bytes(input.as_bytes())
}

/// Generate SHA512 hash of byte slice
///
/// This is a secure hash function suitable for security applications.
///
/// # Arguments
/// * `input` - Bytes to hash
///
/// # Example
/// ```rust,ignore
/// let hash = sha512_bytes(b"hello world");
/// println!("SHA512: {}", hash);
/// ```
pub fn sha512_bytes(input: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_string() {
        let hash1 = hash_string("hello");
        let hash2 = hash_string("hello");
        let hash3 = hash_string("world");

        // Same input should produce same hash
        assert_eq!(hash1, hash2);

        // Different input should produce different hash (with high probability)
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_bytes() {
        let hash1 = hash_bytes(&[1, 2, 3]);
        let hash2 = hash_bytes(&[1, 2, 3]);
        let hash3 = hash_bytes(&[3, 2, 1]);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_xor_encryption() {
        let original = "Hello, World!";
        let key = "secret";

        let encrypted = xor_encrypt(original, key);
        let decrypted = xor_decrypt(&encrypted, key);

        assert_eq!(decrypted, original);

        // Encrypted data should be different from original
        assert_ne!(encrypted, original.as_bytes());
    }

    #[test]
    fn test_simple_checksum() {
        let checksum1 = simple_checksum("hello");
        let checksum2 = simple_checksum("hello");
        let checksum3 = simple_checksum("world");

        assert_eq!(checksum1, checksum2);
        assert_ne!(checksum1, checksum3);
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("hello", "hello"));
        assert!(!constant_time_compare("hello", "world"));
        assert!(!constant_time_compare("hello", "hello!"));
        assert!(!constant_time_compare("hello!", "hello"));
    }

    #[test]
    fn test_constant_time_compare_bytes() {
        assert!(constant_time_compare_bytes(b"hello", b"hello"));
        assert!(!constant_time_compare_bytes(b"hello", b"world"));
        assert!(!constant_time_compare_bytes(b"hello", b"hello!"));
    }

    #[test]
    fn test_obfuscation() {
        let original = "Hello World";
        let offset = 5;

        let obfuscated = obfuscate_string(original, offset);
        let deobfuscated = deobfuscate_string(&obfuscated, offset);

        assert_eq!(deobfuscated, original);
        assert_ne!(obfuscated, original);

        // Test with numbers and special characters (should remain unchanged)
        let mixed = "Hello123!";
        let obf_mixed = obfuscate_string(mixed, 3);
        let deobf_mixed = deobfuscate_string(&obf_mixed, 3);
        assert_eq!(deobf_mixed, mixed);
    }

    // Tests for cryptographic hash functions

    #[test]
    fn test_md5() {
        // Test with known test vectors
        assert_eq!(md5(""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5("a"), "0cc175b9c0f1b6a831c399e269772661");
        assert_eq!(md5("abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(md5("hello world"), "5eb63bbbe01eeed093cb22bb8f5acdc3");
        assert_eq!(
            md5("The quick brown fox jumps over the lazy dog"),
            "9e107d9d372bb6826bd81d3542a419d6"
        );

        // Test consistency
        let hash1 = md5("test");
        let hash2 = md5("test");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_md5_bytes() {
        // Test with known test vectors
        assert_eq!(md5_bytes(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_bytes(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            md5_bytes(b"hello world"),
            "5eb63bbbe01eeed093cb22bb8f5acdc3"
        );

        // Test that string and bytes variants produce same result
        assert_eq!(md5("test"), md5_bytes(b"test"));
    }

    #[test]
    fn test_sha1() {
        // Test with known test vectors
        assert_eq!(sha1(""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(sha1("abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            sha1("hello world"),
            "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed"
        );
        assert_eq!(
            sha1("The quick brown fox jumps over the lazy dog"),
            "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"
        );

        // Test consistency
        let hash1 = sha1("test");
        let hash2 = sha1("test");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_sha1_bytes() {
        // Test with known test vectors
        assert_eq!(sha1_bytes(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(
            sha1_bytes(b"abc"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            sha1_bytes(b"hello world"),
            "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed"
        );

        // Test that string and bytes variants produce same result
        assert_eq!(sha1("test"), sha1_bytes(b"test"));
    }

    #[test]
    fn test_sha256() {
        // Test with known test vectors
        assert_eq!(
            sha256(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256("hello world"),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
        assert_eq!(
            sha256("The quick brown fox jumps over the lazy dog"),
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
        );

        // Test consistency
        let hash1 = sha256("test");
        let hash2 = sha256("test");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_sha256_bytes() {
        // Test with known test vectors
        assert_eq!(
            sha256_bytes(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_bytes(b"hello world"),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );

        // Test that string and bytes variants produce same result
        assert_eq!(sha256("test"), sha256_bytes(b"test"));
    }

    #[test]
    fn test_sha512() {
        // Test with known test vectors
        assert_eq!(sha512(""), "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e");
        assert_eq!(sha512("abc"), "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f");
        assert_eq!(sha512("hello world"), "309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f");

        // Test consistency
        let hash1 = sha512("test");
        let hash2 = sha512("test");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_sha512_bytes() {
        // Test with known test vectors
        assert_eq!(sha512_bytes(b""), "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e");
        assert_eq!(sha512_bytes(b"abc"), "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f");

        // Test that string and bytes variants produce same result
        assert_eq!(sha512("test"), sha512_bytes(b"test"));
    }

    #[test]
    fn test_hash_uniqueness() {
        // Test that different inputs produce different hashes
        let input1 = "hello";
        let input2 = "world";

        assert_ne!(md5(input1), md5(input2));
        assert_ne!(sha1(input1), sha1(input2));
        assert_ne!(sha256(input1), sha256(input2));
        assert_ne!(sha512(input1), sha512(input2));
    }

    #[test]
    fn test_hash_lengths() {
        let input = "test";

        // MD5 produces 32 character hex string (128 bits)
        assert_eq!(md5(input).len(), 32);

        // SHA1 produces 40 character hex string (160 bits)
        assert_eq!(sha1(input).len(), 40);

        // SHA256 produces 64 character hex string (256 bits)
        assert_eq!(sha256(input).len(), 64);

        // SHA512 produces 128 character hex string (512 bits)
        assert_eq!(sha512(input).len(), 128);
    }

    #[test]
    fn test_empty_input_handling() {
        // All hash functions should handle empty input gracefully
        assert!(!md5("").is_empty());
        assert!(!sha1("").is_empty());
        assert!(!sha256("").is_empty());
        assert!(!sha512("").is_empty());
    }
}
