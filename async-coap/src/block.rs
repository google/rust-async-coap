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

use std::convert::From;
use std::fmt::{Debug, Display, Formatter};

/// Type for interpreting `block1` and `block2` option values.
#[derive(Copy, Clone, Eq, Ord, Hash, PartialOrd, PartialEq)]
pub struct BlockInfo(pub u32);

impl BlockInfo {
    const MORE_FLAG: u32 = 0b1000;

    /// Maximum legal value for `num`.
    pub const NUM_MAX: u32 = ((1 << 20) - 1);

    const SZX_RESERVED: u8 = 0b0111;

    /// Maximum legal value for `szx`.
    pub const SZX_MAX: u8 = Self::SZX_RESERVED - 1;

    /// Constructs a new `BlockInfo` from the number, more flag, and size exponent.
    pub fn new(num: u32, m: bool, szx: u8) -> Option<BlockInfo> {
        if num > Self::NUM_MAX || szx > Self::SZX_MAX {
            None
        } else {
            Some(BlockInfo((num << 4) + ((m as u32) << 3) + szx as u32))
        }
    }

    /// Block number value.
    #[inline]
    pub fn num(&self) -> u32 {
        self.0 >> 4
    }

    /// More flag value. If set, there are more blocks to follow.
    #[inline]
    pub fn more_flag(&self) -> bool {
        (self.0 & Self::MORE_FLAG) == Self::MORE_FLAG
    }

    /// Block size exponent field value.
    #[inline]
    pub fn szx(&self) -> u8 {
        self.0 as u8 & 0b111
    }

    /// The offset (in bytes) that this block starts at.
    #[inline]
    pub fn offset(&self) -> usize {
        let val = self.0 as usize;
        (val & !0xF) << (val & 0b0111)
    }

    /// The length of this block, in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        1 << (self.szx() as usize + 4)
    }

    /// Determines if calling [`BlockInfo::next`] will yield the next block or not.
    ///
    /// If this method returns true, calling [`BlockInfo::next`] will fail.
    pub fn is_max_block(&self) -> bool {
        self.num() == Self::NUM_MAX
    }

    /// Checks the validity of the contained value.
    pub fn is_invalid(&self) -> bool {
        (self.num() > Self::NUM_MAX) || self.szx() == Self::SZX_RESERVED
    }

    /// Checks the contained value for validity and, if valid, returns it in an `Option`.
    pub fn valid(self) -> Option<BlockInfo> {
        if self.is_invalid() {
            None
        } else {
            Some(self)
        }
    }

    /// Calculates what the next block will be, if any.
    pub fn next(&self) -> Option<BlockInfo> {
        if self.num() < Self::NUM_MAX {
            BlockInfo(self.0 + 0x10).valid()
        } else {
            None
        }
    }

    /// Calculates a smaller block size that maintains this block's offset.
    pub fn smaller(&self) -> Option<BlockInfo> {
        let szx = self.szx();
        if szx != Self::SZX_RESERVED && szx > 0 {
            Self::new(self.num() * 2, self.more_flag(), szx - 1)
        } else {
            None
        }
    }

    /// Returns this `BlockInfo`'s value *with* the more flag set.
    pub fn with_more_flag(&self) -> BlockInfo {
        BlockInfo(self.0 | Self::MORE_FLAG)
    }

    /// Returns this `BlockInfo`'s value *without* the more flag set.
    pub fn without_more_flag(&self) -> BlockInfo {
        BlockInfo(self.0 & !Self::MORE_FLAG)
    }
}

impl From<u32> for BlockInfo {
    fn from(x: u32) -> Self {
        BlockInfo(x)
    }
}

impl Default for BlockInfo {
    /// Returns a block info with an offset of zero and a block size of 1024.
    fn default() -> Self {
        BlockInfo(6)
    }
}

impl Display for BlockInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}/{}/{}",
            self.num(),
            self.more_flag() as u8,
            self.len()
        )?;
        if self.is_invalid() {
            f.write_str("(!)")
        } else {
            Ok(())
        }
    }
}

impl Debug for BlockInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "BlockInfo(0x{:06X})", self.0)?;
        Display::fmt(self, f)?;
        f.write_str(")")
    }
}

/// Tool for reconstructing block-wise messages.
///
/// This mechanism was designed so that it can be used successfully with `block1` (request payload)
/// and `block2` (response payload) messages.
#[derive(Debug)]
pub struct BlockReconstructor<F> {
    next_block: BlockInfo,
    is_finished: bool,
    write: F,
}

impl<F: Default + std::io::Write> Default for BlockReconstructor<F> {
    fn default() -> Self {
        BlockReconstructor::new(F::default(), Default::default())
    }
}

impl<F> BlockReconstructor<F>
where
    F: std::io::Write,
{
    /// Creates a new instance of `BlockReconstructor`.
    pub fn new(write: F, next_block: BlockInfo) -> BlockReconstructor<F> {
        BlockReconstructor {
            next_block: next_block.without_more_flag(),
            is_finished: false,
            write,
        }
    }

    /// The next block this object wants.
    pub fn next_block(&self) -> BlockInfo {
        self.next_block
    }

    /// Returns true if we have received all of our blocks and have no additional processing
    /// to perform.
    pub fn is_finished(&self) -> bool {
        self.is_finished
    }

    /// Consumes this object and returns the underlying `std::io::Write` instance.
    pub fn into_inner(self) -> F {
        self.write
    }

    /// Feeds a block (with an associated payload) to the reconstructor.
    pub fn feed(&mut self, block: BlockInfo, payload: &[u8]) -> Result<bool, ()> {
        if self.is_finished {
            return Ok(true);
        }

        if block.offset() < self.next_block.offset() {
            // Ignore blocks we have already seen.
            return Ok(false);
        }

        if block.offset() > self.next_block.offset() {
            // This isn't the block we were expecting.
            return Err(());
        }

        if !block.more_flag() {
            self.is_finished = true;
        } else if block.len() > payload.len() {
            // Not enough data?
            return Err(());
        } else if block.len() < payload.len() {
            // Extra data?
            return Err(());
        } else if let Some(next_block) = block.without_more_flag().next() {
            self.next_block = next_block;
        } else {
            // Call to `next()` failed.
            return Err(());
        }

        self.write.write_all(payload).map_err(|_| ())?;

        Ok(self.is_finished)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let block = BlockInfo::default();
        assert_eq!(false, block.more_flag());
        assert_eq!(6, block.szx());
        assert_eq!(0, block.num());
        assert_eq!(1024, block.len());
        assert_eq!(0, block.offset());
        assert_eq!(false, block.is_max_block());
        assert_eq!(false, block.is_invalid());
    }

    #[test]
    fn next() {
        let block = BlockInfo::default().next().unwrap();
        assert_eq!(BlockInfo::default().more_flag(), block.more_flag());
        assert_eq!(6, block.szx());
        assert_eq!(1, block.num());
        assert_eq!(1024, block.len());
        assert_eq!(1024, block.offset());
        assert_eq!(false, block.is_max_block());
        assert_eq!(false, block.is_invalid());
    }

    #[test]
    fn smaller() {
        let block = BlockInfo::default().smaller().unwrap();
        assert_eq!(BlockInfo::default().more_flag(), block.more_flag());
        assert_eq!(5, block.szx());
        assert_eq!(0, block.num());
        assert_eq!(512, block.len());
        assert_eq!(0, block.offset());
        assert_eq!(false, block.is_max_block());
        assert_eq!(false, block.is_invalid());
    }

    #[test]
    fn next_smaller() {
        let block = BlockInfo::default().next().unwrap().smaller().unwrap();
        assert_eq!(BlockInfo::default().more_flag(), block.more_flag());
        assert_eq!(5, block.szx());
        assert_eq!(2, block.num());
        assert_eq!(512, block.len());
        assert_eq!(1024, block.offset());
        assert_eq!(false, block.is_max_block());
        assert_eq!(false, block.is_invalid());

        let smaller = block.smaller().unwrap();
        assert_eq!(256, smaller.len());
        assert_eq!(block.offset(), smaller.offset());
    }

    #[test]
    fn with_and_without_more_flag() {
        let block = BlockInfo::default().without_more_flag();
        assert_eq!(false, block.more_flag());
        assert_eq!(6, block.szx());
        assert_eq!(0, block.num());
        assert_eq!(1024, block.len());
        assert_eq!(0, block.offset());
        assert_eq!(false, block.is_max_block());
        assert_eq!(false, block.is_invalid());

        let block = block.with_more_flag();
        assert_eq!(true, block.more_flag());
        assert_eq!(6, block.szx());
        assert_eq!(0, block.num());
        assert_eq!(1024, block.len());
        assert_eq!(0, block.offset());
        assert_eq!(false, block.is_max_block());
        assert_eq!(false, block.is_invalid());
    }

    #[test]
    fn check_next() {
        let block = BlockInfo::new(BlockInfo::NUM_MAX - 1, true, 6).unwrap();
        assert_eq!(true, block.more_flag());
        assert_eq!(6, block.szx());
        assert_eq!(BlockInfo::NUM_MAX - 1, block.num());
        assert_eq!(1024, block.len());
        assert_eq!(1073739776, block.offset());
        assert_eq!(false, block.is_max_block());
        assert_eq!(false, block.is_invalid());

        let block = block.next().unwrap();

        assert_eq!(true, block.more_flag());
        assert_eq!(6, block.szx());
        assert_eq!(BlockInfo::NUM_MAX, block.num());
        assert_eq!(1024, block.len());
        assert_eq!(1073739776 + 1024, block.offset());
        assert_eq!(true, block.is_max_block());
        assert_eq!(false, block.is_invalid());

        assert_eq!(None, block.next());
    }

    #[test]
    fn check_smaller() {
        let block = BlockInfo::new(BlockInfo::NUM_MAX - 1, true, 6).unwrap();
        assert_eq!(false, block.is_invalid());
        assert_eq!(None, block.smaller());

        let block = BlockInfo(0);
        assert_eq!(false, block.is_invalid());
        assert_eq!(None, block.smaller());
    }

    #[test]
    fn validity() {
        let block = BlockInfo(0);
        assert_eq!(false, block.is_invalid());
        assert_eq!(false, block.smaller().is_some());
        assert_eq!(true, block.next().is_some());
        assert_eq!(Some(block), block.valid());

        let block = BlockInfo(1);
        assert_eq!(false, block.is_invalid());
        assert_eq!(true, block.smaller().is_some());
        assert_eq!(true, block.next().is_some());
        assert_eq!(Some(block), block.valid());

        let block = BlockInfo(!0);
        assert_eq!(true, block.is_invalid());
        assert_eq!(None, block.smaller());
        assert_eq!(None, block.next());
        assert_eq!(None, block.valid());

        let block = BlockInfo(BlockInfo::SZX_RESERVED as u32);
        assert_eq!(true, block.is_invalid());
        assert_eq!(None, block.smaller());
        assert_eq!(None, block.next());
        assert_eq!(None, block.valid());

        let block = BlockInfo(BlockInfo::SZX_RESERVED as u32);
        assert_eq!(true, block.is_invalid());
        assert_eq!(None, block.smaller());
        assert_eq!(None, block.next());
        assert_eq!(None, block.valid());
    }
}
