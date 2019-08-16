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
use std::convert::Into;

/// A convenience iterator for parsing options from a byte buffer.
#[derive(Debug, Clone)]
pub struct OptionIterator<'a> {
    iter: core::slice::Iter<'a, u8>,
    last_option: OptionNumber,
}

impl<'a> Default for OptionIterator<'a> {
    fn default() -> Self {
        OptionIterator::new(&[])
    }
}

impl<'a> OptionIterator<'a> {
    /// Creates a new instance of an `OptionIterator` with the given byte slice.
    pub fn new(buffer: &'a [u8]) -> OptionIterator<'a> {
        OptionIterator {
            iter: buffer.iter(),
            last_option: Default::default(),
        }
    }

    /// Returns the unread remaining options as a byte slice.
    pub fn as_slice(&self) -> &'a [u8] {
        self.iter.as_slice()
    }

    /// Peek ahead to the next option without moving the iterator forward.
    pub fn peek(&mut self) -> Option<Result<(OptionNumber, &'a [u8]), Error>> {
        decode_option(&mut self.iter.clone(), self.last_option).transpose()
    }

    /// Determine if the next option has a specific number and value without moving the
    /// iterator forward.
    pub fn peek_eq<T>(&mut self, key: OptionKey<T>, value: T) -> bool
    where
        T: Into<OptionValue<'a>>,
    {
        let mut temp_array = [0; 8];
        match decode_option(&mut self.iter.clone(), self.last_option) {
            Ok(Some((number, iter_value))) => {
                number == key.0
                    && (match value.into() {
                        OptionValue::Integer(x) => encode_u32(x, &mut temp_array),
                        OptionValue::Bytes(x) => x,
                        OptionValue::ETag(x) => {
                            let temp_slice = &mut temp_array[0..x.len()];
                            temp_slice.copy_from_slice(x.as_bytes());
                            temp_slice
                        }
                    } == iter_value)
            }
            _ => false,
        }
    }
}

impl<'a> Iterator for OptionIterator<'a> {
    type Item = Result<(OptionNumber, &'a [u8]), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = decode_option(&mut self.iter, self.last_option).transpose();
        if let Some(Ok((key, _))) = ret {
            self.last_option = key;
        }
        ret
    }
}

impl AsRef<[u8]> for OptionIterator<'_> {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

/// Extension trait for option iterators that provide additional convenient accessors.
pub trait OptionIteratorExt<'a>: Iterator<Item = Result<(OptionNumber, &'a [u8]), Error>> {
    /// Moves the iterator forward until it finds a matching key or the
    /// spot where it should have been.
    ///
    /// If found, returns the option number and a byte slice of the value.
    ///
    /// Does not consume any options after
    /// the matching key.
    fn find_next(&mut self, key: OptionNumber) -> Option<Result<(OptionNumber, &'a [u8]), Error>>;

    /// Typed version of [`OptionIteratorExt::find_next`].
    ///
    /// Moves the iterator forward until it finds a matching key or the
    /// spot where it should have been.
    ///
    /// If found, returns the value of the option key.
    ///
    /// Does not consume any options after
    /// the matching key.
    fn find_next_of<T>(&mut self, key: OptionKey<T>) -> Option<Result<T, Error>>
    where
        T: TryOptionValueFrom<'a> + Sized,
    {
        if let Some(result) = self.find_next(key.0) {
            match result {
                Ok((_, value)) => {
                    if let Some(x) = T::try_option_value_from(value) {
                        return Some(Ok(x));
                    } else {
                        return Some(Err(Error::ParseFailure));
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }

        None
    }

    /// Extracts a URI relative-reference from the remaining URI_PATH and URI_QUERY options,
    /// moving the iterator past them.
    fn extract_uri(&self) -> Result<RelRefBuf, Error>
    where
        Self: Sized + Clone,
    {
        let mut copy = self.clone();
        let mut buf = String::new();

        while let Some(seg) = copy.find_next_of(option::URI_PATH).transpose()? {
            if !buf.is_empty() {
                buf.push('/');
            }
            buf.extend(seg.escape_uri());
        }

        let mut has_query = false;

        while let Some(item) = copy.find_next_of(option::URI_QUERY).transpose()? {
            if has_query {
                buf.push('&');
            } else {
                buf.push('?');
                has_query = true;
            }
            buf.extend(item.escape_uri().for_query());
        }

        let mut ret = RelRefBuf::from_string(buf).expect("Constructed URI was malformed");

        ret.disambiguate();

        Ok(ret)
    }

    /// Extracts a URI relative-reference from the remaining LOCATION_PATH and LOCATION_QUERY options,
    /// moving the iterator past them.
    fn extract_location(&self) -> Result<RelRefBuf, Error>
    where
        Self: Sized + Clone,
    {
        let mut copy = self.clone();
        let mut buf = String::new();

        while let Some(seg) = copy.find_next_of(option::LOCATION_PATH).transpose()? {
            if !buf.is_empty() {
                buf.push('/');
            }
            buf.extend(seg.escape_uri());
        }

        let mut has_query = false;

        while let Some(item) = copy.find_next_of(option::LOCATION_QUERY).transpose()? {
            if has_query {
                buf.push('&');
            } else {
                buf.push('?');
                has_query = true;
            }
            buf.extend(item.escape_uri().for_query());
        }

        // TODO: Check out those reserved Location-* options and fail if found.
        //       See <https://tools.ietf.org/html/rfc7252#section-5.10.7> for more info.

        Ok(RelRefBuf::from_string(buf).expect("Constructed URI was malformed"))
    }
}

impl<'a, I> OptionIteratorExt<'a> for I
where
    I: Iterator<Item = Result<(OptionNumber, &'a [u8]), Error>> + Sized + Clone,
{
    fn find_next(&mut self, key: OptionNumber) -> Option<Result<(OptionNumber, &'a [u8]), Error>> {
        let next_value = loop {
            let mut iter = self.clone();

            match iter.next()? {
                Err(x) => return Some(Err(x)),
                Ok((number, value)) => {
                    if number == key {
                        *self = iter;
                        break (key, value);
                    }
                    if number < key.0 {
                        *self = iter;
                        continue;
                    }
                }
            };

            return None;
        };

        Some(Ok(next_value))
    }
}
