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

/// Typed option key, for type-safe access to CoAP options.
#[derive(Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct OptionKey<T>(pub OptionNumber, core::marker::PhantomData<*const T>);

impl<T> OptionKey<T> {
    /// Creates a new instance with the given option number.
    pub const fn new(n: OptionNumber) -> OptionKey<T> {
        OptionKey(n, core::marker::PhantomData)
    }
}

impl<T> Copy for OptionKey<T> {}

impl<T> Clone for OptionKey<T> {
    fn clone(&self) -> Self {
        OptionKey(self.0, core::marker::PhantomData)
    }
}

unsafe impl<T> Send for OptionKey<T> {}

impl<T> core::fmt::Debug for OptionKey<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<T> core::ops::Deref for OptionKey<T> {
    type Target = OptionNumber;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Typed key for IF_MATCH option.
pub const IF_MATCH: OptionKey<ETag> = OptionKey::new(OptionNumber::IF_MATCH);

/// Typed key for URI_HOST option.
pub const URI_HOST: OptionKey<&str> = OptionKey::new(OptionNumber::URI_HOST);

/// Typed key for ETAG option.
pub const ETAG: OptionKey<ETag> = OptionKey::new(OptionNumber::ETAG);

/// Typed key for IF_NONE_MATCH option.
pub const IF_NONE_MATCH: OptionKey<()> = OptionKey::new(OptionNumber::IF_NONE_MATCH);

/// Typed key for Observe option.
pub const OBSERVE: OptionKey<u32> = OptionKey::new(OptionNumber::OBSERVE);

/// Typed key for URI-Port option.
pub const URI_PORT: OptionKey<u16> = OptionKey::new(OptionNumber::URI_PORT);

/// Typed key for Location-Path option.
pub const LOCATION_PATH: OptionKey<&str> = OptionKey::new(OptionNumber::LOCATION_PATH);

/// Typed key for OSCORE option.
pub const OSCORE: OptionKey<&[u8]> = OptionKey::new(OptionNumber::OSCORE);

/// Typed key for URI-Path option.
pub const URI_PATH: OptionKey<&str> = OptionKey::new(OptionNumber::URI_PATH);

/// Typed key for Content-Format option.
pub const CONTENT_FORMAT: OptionKey<ContentFormat> = OptionKey::new(OptionNumber::CONTENT_FORMAT);

/// Typed key for Max-Age option.
pub const MAX_AGE: OptionKey<u32> = OptionKey::new(OptionNumber::MAX_AGE);

/// Typed key for URI-Query option.
pub const URI_QUERY: OptionKey<&str> = OptionKey::new(OptionNumber::URI_QUERY);

/// Typed key for Accept option.
pub const ACCEPT: OptionKey<ContentFormat> = OptionKey::new(OptionNumber::ACCEPT);

/// Typed key for Location-Query option.
pub const LOCATION_QUERY: OptionKey<&str> = OptionKey::new(OptionNumber::LOCATION_QUERY);

/// Typed key for Block2 option.
pub const BLOCK2: OptionKey<BlockInfo> = OptionKey::new(OptionNumber::BLOCK2);

/// Typed key for Block1 option.
pub const BLOCK1: OptionKey<BlockInfo> = OptionKey::new(OptionNumber::BLOCK1);

/// Typed key for Size2 option.
pub const SIZE2: OptionKey<u32> = OptionKey::new(OptionNumber::SIZE2);

/// Typed key for Proxy-URI option.
pub const PROXY_URI: OptionKey<&str> = OptionKey::new(OptionNumber::PROXY_URI);

/// Typed key for Proxy-Scheme option.
pub const PROXY_SCHEME: OptionKey<&str> = OptionKey::new(OptionNumber::PROXY_SCHEME);

/// Typed key for Size1 option.
pub const SIZE1: OptionKey<u32> = OptionKey::new(OptionNumber::SIZE1);
