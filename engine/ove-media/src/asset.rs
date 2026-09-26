//! Asset identity: content-hash addressing (the asset IS the cache key).
//!
//! PROJECT_FORMAT_SPEC §5 / DECODER_SPEC §4: ove-media owns asset identity;
//! every downstream cache (probe metadata, keyframe index, decoder pools) keys
//! by content hash so a renamed/moved/copy-deduplicated file is the same asset.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// BLAKE3 content hash (256-bit) of an asset's bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        ContentHash(blake3::hash(bytes).into())
    }

    /// Streaming hash of a file (media files are large; never slurp whole).
    pub fn from_file(path: impl AsRef<Path>) -> std::io::Result<Self> {
        use std::io::Read;
        let mut file = std::fs::File::open(path)?;
        let mut hasher = blake3::Hasher::new();
        let mut buf = vec![0u8; 1 << 20]; // 1 MiB chunks
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(hasher.finalize().as_bytes());
        Ok(ContentHash(out))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in &self.0 {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex())
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({})", self.hex())
    }
}

impl Serialize for ContentHash {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.hex())
    }
}

impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let hex: String = String::deserialize(d)?;
        if hex.len() != 64 {
            return Err(serde::de::Error::custom(
                "content hash must be 64 hex chars",
            ));
        }
        let mut out = [0u8; 32];
        for i in 0..32 {
            out[i] =
                u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(serde::de::Error::custom)?;
        }
        Ok(ContentHash(out))
    }
}

/// Reference to an immutable media asset: identity = content hash,
/// locator = how to reach the bytes (path/URL/scheme handled by callers).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRef {
    pub content_hash: ContentHash,
    /// Path or URI. NOT an identity component — two locators with the same
    /// hash are the same asset (dedup by design).
    pub locator: String,
}

impl AssetRef {
    /// Build from a filesystem path, hashing the file contents.
    pub fn from_path(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let p = path.as_ref();
        let hash = ContentHash::from_file(p)?;
        // locator is canonicalized best-effort; identity never depends on it
        let locator = p.to_string_lossy().into_owned();
        Ok(AssetRef {
            content_hash: hash,
            locator,
        })
    }
}

/// Zero-based stream index inside an asset (container order).
/// Stable identity = (asset content hash, StreamId).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StreamId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamKind {
    Video,
    Audio,
    Data,
    Attachment,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_hex_roundtrips() {
        let h = ContentHash::from_bytes(b"open-video-engine");
        let hex = h.hex();
        assert_eq!(hex.len(), 64);
        let de: ContentHash = serde_json::from_str(&format!("\"{hex}\"")).unwrap();
        assert_eq!(de, h);
        assert_eq!(ContentHash::from_bytes(b"open-video-engine"), h);
        assert_ne!(ContentHash::from_bytes(b"other"), h);
    }

    #[test]
    fn hash_streams_large_file_in_chunks() {
        let dir = std::env::temp_dir().join("ove_media_test_asset");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("blob.bin");
        let data: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&p, &data).unwrap();
        let a = AssetRef::from_path(&p).unwrap();
        assert_eq!(a.content_hash, ContentHash::from_bytes(&data));
        std::fs::remove_file(&p).ok();
    }
}
