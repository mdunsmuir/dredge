// Copyright (C) 2016  Michael Dunsmuir
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::fs::Metadata;

/// I'm still not sure I totally understand the implications of hardcoding
/// this to 512, but Google says I'm not the only one doing it so I don't
/// feel too bad about it.
///
/// This value does seem to be available in `stat.h`, but getting it from
/// there is not the easiest thing so I'm not going to bother for now.
///
/// TODO in the future: get this right, scrape this value out of whatever
/// header file it's hiding in on all the platforms we want to support.
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

// windows... coming soon????
