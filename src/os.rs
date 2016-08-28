use std::fs::Metadata;

/// I'm still totally confused about where this comes from and couldn't find
/// an API to grab it... maybe in system header files?
///
/// ST_BLKSIZE returns the *IO* block size, which is 4096
const DEVICE_BLOCKSIZE: u64 = 512;

// linux

#[cfg(target_family = "linux")]
use std::os::linux::fs::MetadataExt;

#[cfg(target_family = "linux")]
pub fn size(metadata: &Metadata) -> u64 {
    metadata.st_blocks() * DEVICE_BLOCKSIZE
}

// unix

#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;

#[cfg(target_family = "unix")]
pub fn size(metadata: &Metadata) -> u64 {
    metadata.blocks() * DEVICE_BLOCKSIZE
}
