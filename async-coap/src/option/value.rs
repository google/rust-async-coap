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

use super::*;

/// Type describing the type of an option's value.
#[derive(Debug, Copy, Eq, PartialEq, Hash, Clone)]
pub enum OptionValueType {
    /// Opaque option value.
    Opaque,

    /// Option value is determined by the presence or absence of the option.
    Flag,

    /// Integer value.
    Integer,

    /// UTF8 string value.
    String,

    /// Integer value containing a `ContentFormat`.
    ContentFormat,

    /// Integer value containing a `BlockInfo`.
    Block,
}

#[doc(hidden)]
#[derive(Debug)]
pub enum OptionValue<'a> {
    Integer(u32),
    Bytes(&'a [u8]),
    ETag(ETag),
}

impl<'a> From<u8> for OptionValue<'a> {
    fn from(value: u8) -> Self {
        OptionValue::Integer(value as u32)
    }
}

impl<'a> From<u16> for OptionValue<'a> {
    fn from(value: u16) -> Self {
        OptionValue::Integer(value as u32)
    }
}

impl<'a> From<u32> for OptionValue<'a> {
    fn from(value: u32) -> Self {
        OptionValue::Integer(value)
    }
}

impl<'a> From<ContentFormat> for OptionValue<'a> {
    fn from(value: ContentFormat) -> Self {
        OptionValue::Integer(value.0 as u32)
    }
}

impl<'a> From<BlockInfo> for OptionValue<'a> {
    fn from(value: BlockInfo) -> Self {
        OptionValue::Integer(value.0 as u32)
    }
}

impl<'a> From<ETag> for OptionValue<'a> {
    fn from(value: ETag) -> Self {
        OptionValue::ETag(value)
    }
}

impl<'a> From<&'a [u8]> for OptionValue<'a> {
    fn from(value: &'a [u8]) -> Self {
        OptionValue::Bytes(value)
    }
}

impl<'a> From<&'a str> for OptionValue<'a> {
    fn from(value: &'a str) -> Self {
        OptionValue::Bytes(value.as_bytes())
    }
}

impl<'a, 'b> From<&'b &'a str> for OptionValue<'a> {
    fn from(value: &'b &'a str) -> Self {
        OptionValue::Bytes(value.as_bytes())
    }
}

impl<'a> From<()> for OptionValue<'a> {
    fn from(_: ()) -> Self {
        OptionValue::Bytes(&[])
    }
}

#[doc(hidden)]
pub trait TryOptionValueFrom<'a>: Sized {
    fn try_option_value_from(buffer: &'a [u8]) -> Option<Self>;
}

impl<'a> TryOptionValueFrom<'a> for ETag {
    fn try_option_value_from(buffer: &'a [u8]) -> Option<Self> {
        if buffer.len() <= ETag::MAX_LEN {
            Some(ETag::new(buffer))
        } else {
            None
        }
    }
}

impl<'a> TryOptionValueFrom<'a> for &'a [u8] {
    fn try_option_value_from(buffer: &'a [u8]) -> Option<Self> {
        Some(buffer)
    }
}

impl<'a> TryOptionValueFrom<'a> for u32 {
    fn try_option_value_from(buffer: &'a [u8]) -> Option<Self> {
        try_decode_u32(buffer)
    }
}

impl<'a> TryOptionValueFrom<'a> for ContentFormat {
    fn try_option_value_from(buffer: &'a [u8]) -> Option<Self> {
        Some(ContentFormat(try_decode_u16(buffer)?))
    }
}

impl<'a> TryOptionValueFrom<'a> for BlockInfo {
    fn try_option_value_from(buffer: &'a [u8]) -> Option<Self> {
        BlockInfo(try_decode_u32(buffer)?).valid()
    }
}

impl<'a> TryOptionValueFrom<'a> for u16 {
    fn try_option_value_from(buffer: &'a [u8]) -> Option<Self> {
        try_decode_u16(buffer)
    }
}

impl<'a> TryOptionValueFrom<'a> for () {
    fn try_option_value_from(_: &'a [u8]) -> Option<Self> {
        Some(())
    }
}

impl<'a> TryOptionValueFrom<'a> for &'a str {
    fn try_option_value_from(buffer: &'a [u8]) -> Option<Self> {
        core::str::from_utf8(buffer).ok()
    }
}
