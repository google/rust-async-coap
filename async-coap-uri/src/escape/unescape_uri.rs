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

use core::fmt::Write;
use std::borrow::Cow;
use std::char::REPLACEMENT_CHARACTER;
use std::convert::TryInto;
use std::fmt;
use std::fmt::Display;
use std::iter::FusedIterator;
use std::str::{from_utf8, from_utf8_unchecked};

/// Used for determining how long a given UTF8 sequence is.
/// Table values come from https://tools.ietf.org/html/rfc3629
static UTF8_CHAR_WIDTH: [u8; 256] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x1F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x3F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x5F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x7F
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 0x9F
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 0xBF
    0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
    2, // 0xDF
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // 0xEF
    4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xFF
];

/// An iterator used to apply URI percent decoding to strings.
///
/// It is constructed via the method [`unescape_uri()`].
/// See the documentation for [`StrExt`] for more information.
///
/// The iterator can be checked for detected errors via the [`first_error()`] method.
/// Otherwise, the iterator will attempt to make a best-effort lossy conversion when
/// faced with errors.
///
/// [`StrExt`]: trait.StrExt.html
/// [`unescape_uri()`]: trait.StrExt.html#tymethod.unescape_uri
/// [`first_error()`]: #method.first_error
#[derive(Debug, Clone)]
pub struct UnescapeUri<'a> {
    pub(super) iter: std::str::Chars<'a>,
    pub(super) iter_index: usize,
    pub(super) next_c: Option<(char, Option<char>)>,
    pub(super) had_error: Option<UnescapeError>,
    pub(super) skip_slashes: bool,
}

impl<'a> Display for UnescapeUri<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.clone().try_for_each(|c| f.write_char(c))
    }
}

impl<'a> From<UnescapeUri<'a>> for Cow<'a, str> {
    fn from(iter: UnescapeUri<'a>) -> Self {
        iter.to_cow()
    }
}

impl<'a> FusedIterator for UnescapeUri<'a> {}

impl<'a> UnescapeUri<'a> {
    /// Returns the first encountered encoding error, if any.
    pub fn first_error(&self) -> Option<DecodingError> {
        let mut iter = self.clone();
        let begin = iter.index();

        while let Some(_) = iter.next() {
            if iter.had_error.is_some() {
                break;
            }
        }

        if let Some(err) = iter.had_error {
            let end = iter.index();
            let i = (end - begin).saturating_sub(1);

            return Some(DecodingError::new(err, i));
        }

        None
    }

    /// Indicates the number of characters that have been read by this iterator
    /// from the source string.
    pub fn index(&self) -> usize {
        self.iter_index
    }

    /// Consumes the iterator and returns a new one that will skip encoded slashes.
    /// See [Skipping Slashes] for more information.
    ///
    /// [Skipping Slashes]: trait.StrExt.html#Skipping_Slashes
    pub fn skip_slashes(mut self) -> Self {
        self.skip_slashes = true;
        self
    }

    /// Decodes the string (lossily if necessary), returning it as a copy-on-write type.
    #[cfg(feature = "std")]
    pub fn to_cow(&self) -> Cow<'a, str> {
        let as_str = self.iter.as_str();
        if as_str
            .find(|c: char| !c.is_ascii_graphic() || c == '%')
            .is_some()
        {
            Cow::from(self.to_string())
        } else {
            Cow::from(as_str)
        }
    }

    /// Attempts to losslessly decode the string, returning it as a copy-on-write type.
    ///
    /// # Errors
    ///
    /// If the string cannot be decoded losslessly, then a [`DecodingError`] is
    /// returned, from which the location of the decoding error can be obtained
    /// with [`DecodingError::index`].
    #[cfg(feature = "std")]
    pub fn try_to_cow(&self) -> Result<Cow<'a, str>, DecodingError> {
        let as_str = self.iter.as_str();
        if as_str
            .find(|c: char| !c.is_ascii_graphic() || c == '%')
            .is_some()
        {
            self.try_to_string().map(Cow::from)
        } else {
            Ok(Cow::from(as_str))
        }
    }

    /// Attempts to losslessly decode the string, returning it as a standard [`String`].
    ///
    /// If the string cannot be decoded losslessly, then the location of the encoding
    /// error is returned as an `Err`.
    ///
    /// [`String`]: std::string::String
    #[cfg(feature = "std")]
    pub fn try_to_string(&self) -> Result<String, DecodingError> {
        self.clone().try_into()
    }

    /// Checks to see if this iterator has the given *unescaped* prefix,
    /// and, if it does, returns the index of the end of the pattern in the haystack.
    ///
    /// Note that this doesn't take a `Pattern` as an argument, because that
    /// trait only works on string slices and not iterators.
    ///
    /// If encoding errors are encountered then the method returns `None`.
    ///
    /// ## Example
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// let path = "bl%C3%A5b%C3%A6r/%2F/syltet%C3%B8y/and/on/and/on";
    /// assert!(path.unescape_uri().starts_with("blåbær///syltetøy").is_some());
    /// ```
    pub fn starts_with<T: AsRef<str>>(&self, unescaped_prefix: T) -> Option<usize> {
        let mut iter_self = self.clone();
        let mut iter_pat = unescaped_prefix.as_ref().chars();

        loop {
            let b = iter_pat.next();

            if b.is_none() {
                break Some(iter_self.index());
            }

            let a = iter_self.next();

            if iter_self.had_error.is_some() {
                break None;
            }

            if a != b {
                break None;
            }
        }
    }
}

#[cfg(feature = "std")]
impl<'a> TryInto<String> for UnescapeUri<'a> {
    type Error = DecodingError;

    fn try_into(mut self) -> Result<String, Self::Error> {
        let mut buffer = String::with_capacity(self.size_hint().0);

        while let Some(ch) = self.next() {
            if let Some(e) = self.had_error {
                return Err(DecodingError::new(e, self.index()));
            }

            buffer.push(ch);
        }

        buffer.shrink_to_fit();
        Ok(buffer)
    }
}

/// This is returned, when an error occured while decoding.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecodingError {
    inner: UnescapeError,
    /// The index at which the error occured.
    pub index: usize,
}

#[cfg(feature = "std")]
impl ::std::error::Error for DecodingError {}

impl fmt::Display for DecodingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl DecodingError {
    pub(crate) fn new(inner: UnescapeError, index: usize) -> Self {
        Self { inner, index }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum UnescapeError {
    /// found unescaped ascii control char
    UnescapedAsciiControl(char),
    /// found an unescaped space (' ')
    Space,
    /// there is no char after the `%`
    MissingChar,
    /// the char following a `%` must be an ascii character (in the range from 0-9 and a-f or A-F)
    InvalidEscape(char),
    /// We don't allow escaped ascii control codes for security reasons.
    AsciiControl(char),
    InvalidUtf8 {
        buf: [u8; 4],
        len: usize,
    },
    InvalidByte(u8),
    UnexpectedChar(char),
}

impl fmt::Display for UnescapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnescapedAsciiControl(c) => {
                write!(f, "unescaped ascii control character `{:?}`", c)
            }
            Self::Space => write!(f, "unescaped space ` `"),
            Self::MissingChar => write!(f, "missing char after `%`"),
            Self::InvalidEscape(c) => write!(
                f,
                "the char after `%` must be a valid hex character `{:?}`",
                c
            ),
            Self::AsciiControl(c) => write!(
                f,
                "ascii control codes are not allowed for security reasons `{:?}`",
                c
            ),
            Self::InvalidUtf8 { buf, len } => write!(f, "invalid utf8 {:?}", &buf[..*len]),
            Self::InvalidByte(b) => write!(f, "not a valid utf8 character `{}`", b),
            Self::UnexpectedChar(c) => write!(f, "unexpected char `{:?}`", c),
        }
    }
}

#[cfg(feature = "std")]
impl ::std::error::Error for UnescapeError {}

impl<'a> Iterator for UnescapeUri<'a> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        let mut utf8_buf = [0u8; 4];
        let mut utf8_len = 0usize;

        if let Some((c, next_c)) = self.next_c.take() {
            if let Some(x) = next_c {
                self.next_c = Some((x, None));
            }

            return Some(c);
        }

        while let Some(c) = self.iter.next() {
            self.iter_index += 1;

            // Immediately drop all unescaped ascii control codes
            if c.is_ascii_control() {
                self.had_error = Some(UnescapeError::UnescapedAsciiControl(c));
                continue;
            }

            match c {
                ' ' => {
                    // Mark unescaped spaces as an error, but pass them along.
                    self.had_error = Some(UnescapeError::Space);

                    if utf8_len == 0 {
                        return Some(c);
                    } else {
                        self.next_c = Some((c, None));
                        return Some(REPLACEMENT_CHARACTER);
                    }
                }
                '%' => {
                    self.iter_index += 1;

                    let msn = match self.iter.next() {
                        Some(c) if c.is_ascii_hexdigit() => c as u8,
                        Some(c) => {
                            self.had_error = Some(UnescapeError::InvalidEscape(c));

                            if utf8_len == 0 {
                                self.next_c = Some((c, None));
                                return Some('%');
                            } else {
                                self.next_c = Some((c, None));
                                return Some(REPLACEMENT_CHARACTER);
                            }
                        }
                        None => {
                            self.iter_index -= 1;
                            self.had_error = Some(UnescapeError::MissingChar);
                            self.next_c = Some((c, None));
                            return Some(REPLACEMENT_CHARACTER);
                        }
                    };

                    self.iter_index += 1;

                    let lsn = match self.iter.next() {
                        Some(c) if c.is_ascii_hexdigit() => c as u8,
                        Some(c) => {
                            self.had_error = Some(UnescapeError::InvalidEscape(c));

                            if utf8_len == 0 {
                                self.next_c = Some((msn as char, Some(c)));
                                return Some('%');
                            } else {
                                self.next_c = Some((c, None));
                                return Some(REPLACEMENT_CHARACTER);
                            }
                        }
                        None => {
                            self.iter_index -= 1;
                            self.had_error = Some(UnescapeError::MissingChar);
                            self.next_c = Some((c, None));
                            return Some(REPLACEMENT_CHARACTER);
                        }
                    };

                    let buf = [msn, lsn];

                    // SAFETY: This is safe because we just verified that
                    // these two characters were ASCII hex digits.
                    let buf_str = unsafe { from_utf8_unchecked(&buf) };

                    let decoded = u8::from_str_radix(&buf_str, 16).unwrap();

                    if self.skip_slashes && decoded == b'/' {
                        // Skip decoding escaped slashes.
                        self.next_c = Some((msn as char, Some(lsn as char)));
                        return Some('%');
                    }

                    if (decoded as char).is_ascii_control() {
                        // We don't allow escaped ascii control codes for security reasons.
                        // We signal our error and then convert the code to it's visual
                        // equivalent in the `CONTROL_PICTURES` unicode block at 0x2400.
                        const CONTROL_PICTURES: u32 = 0x2400;
                        let c = core::char::from_u32(decoded as u32 + CONTROL_PICTURES).unwrap();
                        self.had_error = Some(UnescapeError::AsciiControl(decoded as char));

                        if utf8_len == 0 {
                            return Some(c);
                        } else {
                            self.next_c = Some((c, None));
                            return Some(REPLACEMENT_CHARACTER);
                        }
                    } else if decoded & 0x80 == 0x80 {
                        // This is a UTF8 byte.
                        utf8_buf[utf8_len] = decoded;
                        utf8_len += 1;

                        if utf8_len >= UTF8_CHAR_WIDTH[utf8_buf[0] as usize] as usize {
                            if let Ok(utf8_str) = from_utf8(&utf8_buf[..utf8_len]) {
                                return Some(utf8_str.chars().next().unwrap());
                            } else {
                                self.had_error = Some(UnescapeError::InvalidUtf8 {
                                    buf: utf8_buf,
                                    len: utf8_len,
                                });
                                return Some(REPLACEMENT_CHARACTER);
                            }
                        }
                    } else if utf8_len != 0 {
                        self.had_error = Some(UnescapeError::InvalidByte(decoded));
                        self.next_c = Some((decoded as char, None));
                        return Some(REPLACEMENT_CHARACTER);
                    } else {
                        return Some(decoded as char);
                    }
                }
                c => {
                    if utf8_len != 0 {
                        self.next_c = Some((c, None));
                        self.had_error = Some(UnescapeError::UnexpectedChar(c));
                        return Some(REPLACEMENT_CHARACTER);
                    }

                    return Some(c);
                }
            }
        }

        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.iter.size_hint().0;
        (n, Some(n))
    }
}
