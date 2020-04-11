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
use std::fmt::Display;
use std::iter::FusedIterator;
use std::str::from_utf8_unchecked;

#[cfg(feature = "std")]
use std::borrow::Cow;

fn is_char_uri_unreserved(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~'
}

fn is_char_uri_sub_delim(c: char) -> bool {
    c == '!'
        || c == '$'
        || c == '&'
        || c == '\''
        || c == '('
        || c == ')'
        || c == '*'
        || c == '+'
        || c == ','
        || c == ';'
        || c == '='
}

fn is_char_uri_pchar(c: char) -> bool {
    is_char_uri_unreserved(c) || is_char_uri_sub_delim(c) || c == ':' || c == '@'
}

fn is_char_uri_quote(c: char) -> bool {
    c != '+' && (is_char_uri_pchar(c) || c == '/' || c == '?')
}

fn is_char_uri_fragment(c: char) -> bool {
    is_char_uri_pchar(c) || c == '/' || c == '?' || c == '#'
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub(super) enum EscapeUriState {
    Normal,
    OutputHighNibble(u8),
    OutputLowNibble(u8),
}

/// An internal, unstable trait that is used to adjust the behavior of [`EscapeUri`].
///
/// It is subject to change and is not considered stable.
#[doc(hidden)]
pub trait NeedsEscape: Clone {
    fn byte_needs_escape(b: u8) -> bool {
        Self::char_needs_escape(b as char) || (b & 0x80) != 0
    }
    fn char_needs_escape(c: char) -> bool;
    fn escape_space_as_plus() -> bool {
        false
    }
}

/// A zero-sized implementor of [`NeedsEscape`] that escapes all reserved characters.
///
/// Its behavior is subject to change and is not considered stable.
#[doc(hidden)]
#[derive(Default, Copy, Clone, Debug)]
pub struct EscapeUriFull;
impl NeedsEscape for EscapeUriFull {
    fn char_needs_escape(c: char) -> bool {
        !is_char_uri_unreserved(c)
    }
}

/// A zero-sized implementor of [`NeedsEscape`] for escaping path segments.
///
/// This used for the default behavior of [`escape_uri()`](trait.StrExt.html#tymethod.escape_uri).
///
/// Its behavior is subject to change and is not considered stable.
#[doc(hidden)]
#[derive(Default, Copy, Clone, Debug)]
pub struct EscapeUriSegment;
impl NeedsEscape for EscapeUriSegment {
    fn char_needs_escape(c: char) -> bool {
        !is_char_uri_pchar(c)
    }
}

/// A zero-sized implementor of [`NeedsEscape`] for escaping the entire authority component.
///
/// Its behavior is subject to change and is not considered stable.
#[doc(hidden)]
#[derive(Default, Copy, Clone, Debug)]
pub struct EscapeUriAuthority;
impl NeedsEscape for EscapeUriAuthority {
    fn char_needs_escape(c: char) -> bool {
        !is_char_uri_pchar(c) && c != '[' && c != ']'
    }
}

/// A zero-sized implementor of [`NeedsEscape`] for escaping query items.
///
/// Its behavior is subject to change and is not considered stable.
///
#[doc(hidden)]
#[derive(Default, Copy, Clone, Debug)]
pub struct EscapeUriQuery;
impl NeedsEscape for EscapeUriQuery {
    fn char_needs_escape(c: char) -> bool {
        !is_char_uri_quote(c)
    }

    fn escape_space_as_plus() -> bool {
        true
    }
}

/// A zero-sized implementor of [`NeedsEscape`] for escaping the fragment.
///
/// Its behavior is subject to change and is not considered stable.
///
#[doc(hidden)]
#[derive(Default, Copy, Clone, Debug)]
pub struct EscapeUriFragment;
impl NeedsEscape for EscapeUriFragment {
    fn char_needs_escape(c: char) -> bool {
        !is_char_uri_fragment(c)
    }
}

/// An iterator used to apply URI percent encoding to strings.
///
/// It is constructed via the method [`escape_uri()`].
/// See the documentation for [`StrExt`] for more information.
///
/// [`StrExt`]: trait.StrExt.html
/// [`escape_uri()`]: trait.StrExt.html#tymethod.escape_uri
#[derive(Debug, Clone)]
pub struct EscapeUri<'a, X: NeedsEscape = EscapeUriSegment> {
    pub(super) iter: std::slice::Iter<'a, u8>,
    pub(super) state: EscapeUriState,
    pub(super) needs_escape: X,
}

#[cfg(feature = "std")]
impl<'a, X: NeedsEscape> From<EscapeUri<'a, X>> for Cow<'a, str> {
    fn from(iter: EscapeUri<'a, X>) -> Self {
        iter.to_cow()
    }
}

impl<'a, X: NeedsEscape> EscapeUri<'a, X> {
    /// Determines if this iterator will actually escape anything.
    pub fn is_needed(&self) -> bool {
        for b in self.iter.clone() {
            if X::byte_needs_escape(*b) {
                return true;
            }
        }

        false
    }

    /// Converts this iterator into a [`std::borrow::Cow<str>`].
    #[cfg(feature = "std")]
    pub fn to_cow(&self) -> Cow<'a, str> {
        if self.is_needed() {
            Cow::from(self.to_string())
        } else {
            Cow::from(unsafe { from_utf8_unchecked(self.iter.as_slice()) })
        }
    }

    /// Converts this iterator into one that escapes all except unreserved characters.
    pub fn full(self) -> EscapeUri<'a, EscapeUriFull> {
        EscapeUri {
            iter: self.iter,
            state: self.state,
            needs_escape: EscapeUriFull,
        }
    }

    /// Converts this iterator into one optimized for escaping query components.
    pub fn for_query(self) -> EscapeUri<'a, EscapeUriQuery> {
        EscapeUri {
            iter: self.iter,
            state: self.state,
            needs_escape: EscapeUriQuery,
        }
    }

    /// Converts this iterator into one optimized for escaping fragment components.
    pub fn for_fragment(self) -> EscapeUri<'a, EscapeUriFragment> {
        EscapeUri {
            iter: self.iter,
            state: self.state,
            needs_escape: EscapeUriFragment,
        }
    }

    /// Converts this iterator into one optimized for escaping fragment components.
    pub fn for_authority(self) -> EscapeUri<'a, EscapeUriAuthority> {
        EscapeUri {
            iter: self.iter,
            state: self.state,
            needs_escape: EscapeUriAuthority,
        }
    }
}

impl<'a, X: NeedsEscape> Display for EscapeUri<'a, X> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.clone().try_for_each(|c| f.write_char(c))
    }
}

impl<'a, X: NeedsEscape> FusedIterator for EscapeUri<'a, X> {}

impl<'a, X: NeedsEscape> Iterator for EscapeUri<'a, X> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        match self.state {
            EscapeUriState::Normal => match self.iter.next().copied() {
                Some(b) if X::escape_space_as_plus() && b == b' ' => Some('+'),
                Some(b) if X::byte_needs_escape(b) => {
                    self.state = EscapeUriState::OutputHighNibble(b);
                    Some('%')
                }
                Some(b) => Some(b as char),
                None => None,
            },

            EscapeUriState::OutputHighNibble(b) => {
                self.state = EscapeUriState::OutputLowNibble(b);
                let nibble = b >> 4;
                if nibble < 9 {
                    Some((b'0' + nibble) as char)
                } else {
                    Some((b'A' + nibble - 10) as char)
                }
            }

            EscapeUriState::OutputLowNibble(b) => {
                self.state = EscapeUriState::Normal;
                let nibble = b & 0b1111;
                if nibble < 9 {
                    Some((b'0' + nibble) as char)
                } else {
                    Some((b'A' + nibble - 10) as char)
                }
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.iter.size_hint().0;
        (n, Some(n * 3))
    }
}
