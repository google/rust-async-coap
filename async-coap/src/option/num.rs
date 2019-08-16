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

/// Type representing a CoAP option number.
#[derive(Copy, Eq, PartialEq, Hash, Clone, Ord, PartialOrd)]
pub struct OptionNumber(pub u16);

impl OptionNumber {
    /// IF_MATCH option.
    pub const IF_MATCH: OptionNumber = OptionNumber(1);

    /// URI_HOST option.
    pub const URI_HOST: OptionNumber = OptionNumber(3);

    /// ETAG option.
    pub const ETAG: OptionNumber = OptionNumber(4);

    /// IF_NONE_MATCH option.
    pub const IF_NONE_MATCH: OptionNumber = OptionNumber(5);

    /// OBSERVE option.
    pub const OBSERVE: OptionNumber = OptionNumber(6);

    /// URI_PORT option.
    pub const URI_PORT: OptionNumber = OptionNumber(7);

    /// LOCATION_PATH option.
    pub const LOCATION_PATH: OptionNumber = OptionNumber(8);

    /// OSCORE option.
    pub const OSCORE: OptionNumber = OptionNumber(9);

    /// URI_PATH option.
    pub const URI_PATH: OptionNumber = OptionNumber(11);

    /// CONTENT_FORMAT option.
    pub const CONTENT_FORMAT: OptionNumber = OptionNumber(12);

    /// MAX_AGE option.
    pub const MAX_AGE: OptionNumber = OptionNumber(14);

    /// URI_QUERY option.
    pub const URI_QUERY: OptionNumber = OptionNumber(15);

    /// ACCEPT option.
    pub const ACCEPT: OptionNumber = OptionNumber(17);

    /// LOCATION_QUERY option.
    pub const LOCATION_QUERY: OptionNumber = OptionNumber(20);

    /// BLOCK2 option.
    pub const BLOCK2: OptionNumber = OptionNumber(23);

    /// BLOCK1 option.
    pub const BLOCK1: OptionNumber = OptionNumber(27);

    /// SIZE2 option.
    pub const SIZE2: OptionNumber = OptionNumber(28);

    /// PROXY_URI option.
    pub const PROXY_URI: OptionNumber = OptionNumber(35);

    /// PROXY_SCHEME option.
    pub const PROXY_SCHEME: OptionNumber = OptionNumber(39);

    /// SIZE1 option.
    pub const SIZE1: OptionNumber = OptionNumber(60);

    /// NO_RESPONSE option.
    pub const NO_RESPONSE: OptionNumber = OptionNumber(258);

    /// Returns true if this option number is critical, false if it is optional.
    pub fn is_critical(self) -> bool {
        const FLAG_CRITICAL: u16 = 1;
        self.0 & FLAG_CRITICAL == FLAG_CRITICAL
    }

    /// Returns true if this option is "un-safe".
    pub fn is_un_safe(self) -> bool {
        const FLAG_UN_SAFE: u16 = 2;
        self.0 & FLAG_UN_SAFE == FLAG_UN_SAFE
    }

    /// Returns true if this option is a "no-cache-key" option.
    pub fn is_no_cache_key(self) -> bool {
        const FLAG_NO_CACHE_KEY_MASK: u16 = 0x1e;
        const FLAG_NO_CACHE_KEY_MAGIC: u16 = 0x1c;
        self.0 & FLAG_NO_CACHE_KEY_MASK == FLAG_NO_CACHE_KEY_MAGIC
    }

    /// Returns the expected value type for this option number.
    pub fn option_value_type(self) -> OptionValueType {
        match self {
            OptionNumber::IF_MATCH => OptionValueType::Opaque,
            OptionNumber::URI_HOST => OptionValueType::String,
            OptionNumber::ETAG => OptionValueType::Opaque,
            OptionNumber::IF_NONE_MATCH => OptionValueType::Flag,
            OptionNumber::OBSERVE => OptionValueType::Integer,
            OptionNumber::URI_PORT => OptionValueType::Integer,
            OptionNumber::LOCATION_PATH => OptionValueType::String,
            OptionNumber::OSCORE => OptionValueType::Opaque,
            OptionNumber::URI_PATH => OptionValueType::String,
            OptionNumber::CONTENT_FORMAT => OptionValueType::ContentFormat,
            OptionNumber::MAX_AGE => OptionValueType::Integer,
            OptionNumber::URI_QUERY => OptionValueType::String,
            OptionNumber::ACCEPT => OptionValueType::ContentFormat,
            OptionNumber::LOCATION_QUERY => OptionValueType::String,
            OptionNumber::BLOCK2 => OptionValueType::Block,
            OptionNumber::BLOCK1 => OptionValueType::Block,
            OptionNumber::SIZE2 => OptionValueType::Integer,
            OptionNumber::PROXY_URI => OptionValueType::String,
            OptionNumber::PROXY_SCHEME => OptionValueType::String,
            OptionNumber::SIZE1 => OptionValueType::Integer,
            OptionNumber::NO_RESPONSE => OptionValueType::Integer,
            OptionNumber(_) => OptionValueType::Opaque,
        }
    }

    /// Returns true if this option is allowed in requests, false if it is prohibited in requests.
    pub fn is_ok_in_request(self) -> bool {
        match self {
            OptionNumber::IF_MATCH => true,
            OptionNumber::URI_HOST => true,
            OptionNumber::ETAG => true,
            OptionNumber::IF_NONE_MATCH => true,
            OptionNumber::OBSERVE => true,
            OptionNumber::URI_PORT => true,
            OptionNumber::LOCATION_PATH => false,
            OptionNumber::URI_PATH => true,
            OptionNumber::CONTENT_FORMAT => true,
            OptionNumber::MAX_AGE => false,
            OptionNumber::URI_QUERY => true,
            OptionNumber::ACCEPT => true,
            OptionNumber::LOCATION_QUERY => false,
            OptionNumber::BLOCK2 => true,
            OptionNumber::BLOCK1 => true,
            OptionNumber::SIZE2 => false,
            OptionNumber::PROXY_URI => true,
            OptionNumber::PROXY_SCHEME => true,
            OptionNumber::SIZE1 => true,
            OptionNumber::NO_RESPONSE => true,

            // We default to true for unknown options.
            OptionNumber(_) => true,
        }
    }

    /// Returns true if this option is allowed in responses, false if it is prohibited in responses.
    pub fn is_ok_in_response(self) -> bool {
        match self {
            OptionNumber::IF_MATCH => false,
            OptionNumber::URI_HOST => false,
            OptionNumber::ETAG => true,
            OptionNumber::IF_NONE_MATCH => false,
            OptionNumber::OBSERVE => true,
            OptionNumber::URI_PORT => false,
            OptionNumber::LOCATION_PATH => true,
            OptionNumber::URI_PATH => false,
            OptionNumber::CONTENT_FORMAT => true,
            OptionNumber::MAX_AGE => true,
            OptionNumber::URI_QUERY => false,
            OptionNumber::ACCEPT => false,
            OptionNumber::LOCATION_QUERY => true,
            OptionNumber::BLOCK2 => true,
            OptionNumber::BLOCK1 => true,
            OptionNumber::SIZE2 => true,
            OptionNumber::PROXY_URI => false,
            OptionNumber::PROXY_SCHEME => false,
            OptionNumber::SIZE1 => false,
            OptionNumber::NO_RESPONSE => false,

            // We default to true for unknown options.
            OptionNumber(_) => true,
        }
    }

    /// Returns true if multiple instances of this option are allowed, false if only one instance
    /// is allowed.
    pub fn is_repeatable(self) -> bool {
        match self {
            OptionNumber::IF_MATCH => true,
            OptionNumber::URI_HOST => false,
            OptionNumber::ETAG => true,
            OptionNumber::IF_NONE_MATCH => false,
            OptionNumber::OBSERVE => false,
            OptionNumber::URI_PORT => false,
            OptionNumber::LOCATION_PATH => true,
            OptionNumber::URI_PATH => true,
            OptionNumber::CONTENT_FORMAT => false,
            OptionNumber::MAX_AGE => false,
            OptionNumber::URI_QUERY => true,
            OptionNumber::ACCEPT => false,
            OptionNumber::LOCATION_QUERY => true,
            OptionNumber::BLOCK2 => false,
            OptionNumber::BLOCK1 => false,
            OptionNumber::SIZE2 => false,
            OptionNumber::PROXY_URI => false,
            OptionNumber::PROXY_SCHEME => false,
            OptionNumber::SIZE1 => false,
            OptionNumber::NO_RESPONSE => false,

            // We default to true for unknown options.
            OptionNumber(_) => true,
        }
    }

    /// Attempts to return a `Some(&'static str)` containing the name of the option.
    ///
    /// If the option number isn't recognized, this method returns `None`.
    pub fn static_name(self) -> Option<&'static str> {
        match self {
            OptionNumber::IF_MATCH => Some("If-Match"),
            OptionNumber::URI_HOST => Some("Uri-Host"),
            OptionNumber::ETAG => Some("ETag"),
            OptionNumber::IF_NONE_MATCH => Some("If-None-Match"),
            OptionNumber::OBSERVE => Some("Observe"),
            OptionNumber::URI_PORT => Some("Uri-Port"),
            OptionNumber::LOCATION_PATH => Some("Location-Path"),
            OptionNumber::OSCORE => Some("OSCORE"),
            OptionNumber::URI_PATH => Some("Uri-Path"),
            OptionNumber::CONTENT_FORMAT => Some("Content-Format"),
            OptionNumber::MAX_AGE => Some("Max-Age"),
            OptionNumber::URI_QUERY => Some("Uri-Query"),
            OptionNumber::ACCEPT => Some("Accept"),
            OptionNumber::LOCATION_QUERY => Some("Location-Query"),
            OptionNumber::BLOCK2 => Some("Block2"),
            OptionNumber::BLOCK1 => Some("Block1"),
            OptionNumber::SIZE2 => Some("Size2"),
            OptionNumber::PROXY_URI => Some("Proxy-Uri"),
            OptionNumber::PROXY_SCHEME => Some("Proxy-Scheme"),
            OptionNumber::SIZE1 => Some("Size1"),
            OptionNumber::NO_RESPONSE => Some("No-Response"),
            _ => None,
        }
    }

    /// Writes out the name of this option along with a text debugging description of the value
    /// associated with this option.
    pub fn fmt_with_value(self, f: &mut std::fmt::Formatter<'_>, value: &[u8]) -> std::fmt::Result {
        write!(f, "{}", self)?;
        match self.option_value_type() {
            OptionValueType::Opaque | OptionValueType::Flag => {
                if !value.is_empty() {
                    f.write_str(":")?;
                    for b in value {
                        write!(f, "{:02X}", b)?;
                    }
                }
            }
            OptionValueType::Integer => {
                if let Some(i) = try_decode_u32(value) {
                    write!(f, ":{}", i)?;
                } else {
                    f.write_str("ERR")?;
                }
            }
            OptionValueType::Block => {
                if let Some(i) = try_decode_u32(value) {
                    write!(f, ":{}", BlockInfo(i))?;
                } else {
                    f.write_str("ERR")?;
                }
            }
            OptionValueType::ContentFormat => {
                if let Some(i) = try_decode_u16(value) {
                    write!(f, ":{}", ContentFormat(i))?;
                } else {
                    f.write_str("ERR")?;
                }
            }
            OptionValueType::String => {
                if let Ok(s) = std::str::from_utf8(value) {
                    write!(f, ":{:?}", s)?;
                } else {
                    f.write_str("ERR")?;
                }
            }
        }

        Ok(())
    }
}

impl core::fmt::Display for OptionNumber {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(name) = self.static_name() {
            f.write_str(name)
        } else {
            // Write out a descriptive identifier.
            if self.is_critical() {
                f.write_str("Opt-")?;
            } else {
                f.write_str("Crit-")?;
            }

            if self.is_un_safe() {
                f.write_str("UnSafe-")?;
            }

            if self.is_no_cache_key() {
                f.write_str("NoCacheKey-")?;
            }

            write!(f, "{}", self.0)
        }
    }
}

impl core::fmt::Debug for OptionNumber {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}({})", self.0, self)
    }
}

impl core::ops::Add<u16> for OptionNumber {
    type Output = Self;
    fn add(self, other: u16) -> Self {
        OptionNumber(self.0 + other)
    }
}

impl core::ops::Sub<OptionNumber> for OptionNumber {
    type Output = u16;
    fn sub(self, other: OptionNumber) -> u16 {
        assert!(self.0 >= other.0);
        self.0 - other.0
    }
}

impl core::cmp::PartialOrd<u16> for OptionNumber {
    fn partial_cmp(&self, other: &u16) -> Option<core::cmp::Ordering> {
        Some(self.0.cmp(other))
    }
}

impl core::cmp::PartialEq<u16> for OptionNumber {
    fn eq(&self, other: &u16) -> bool {
        self.0.eq(other)
    }
}

impl Default for OptionNumber {
    fn default() -> Self {
        OptionNumber(0)
    }
}
