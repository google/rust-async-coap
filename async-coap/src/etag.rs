// Copyright 2019 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use super::util::encode_u32;
use core::convert::From;
use core::ops::Deref;

/// Type for holding the value of an ETag option.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Ord, PartialOrd)]
pub struct ETag {
    len: u8,
    bytes: [u8; 8],
}

impl ETag {
    /// Describes the maximum length of an ETag (8 bytes).
    pub const MAX_LEN: usize = 8;

    /// Constant representing an empty ETag.
    pub const EMPTY: ETag = ETag {
        len: 0u8,
        bytes: [0; 8],
    };

    /// Creates a new ETag instance from the given byte slice.
    pub fn new(x: &[u8]) -> ETag {
        ETag::from(x)
    }

    /// Returns the length of this ETag in bytes.
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns true if the length of this ETag is zero.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the value of this ETag as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }
}

impl std::fmt::Display for ETag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in self.as_bytes() {
            write!(f, "{:02X}", b)?;
        }
        Ok(())
    }
}

impl Default for ETag {
    fn default() -> Self {
        ETag::EMPTY
    }
}

impl Deref for ETag {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl core::cmp::PartialEq<[u8]> for ETag {
    fn eq(&self, other: &[u8]) -> bool {
        self.as_bytes() == other
    }
}

impl core::convert::From<u32> for ETag {
    fn from(x: u32) -> Self {
        let mut bytes = [0u8; 8];
        let len = encode_u32(x, &mut bytes).len();
        ETag {
            len: len as u8,
            bytes,
        }
    }
}

impl core::convert::From<i32> for ETag {
    fn from(x: i32) -> Self {
        core::convert::Into::into(x as u32)
    }
}

impl core::convert::From<u16> for ETag {
    fn from(x: u16) -> Self {
        core::convert::Into::into(x as u32)
    }
}

impl core::convert::From<&[u8]> for ETag {
    // Note: this will panic if x is too big.
    fn from(x: &[u8]) -> Self {
        let mut bytes = [0u8; 8];
        let len = x.len();
        bytes[..len].copy_from_slice(x);
        ETag {
            len: len as u8,
            bytes,
        }
    }
}
