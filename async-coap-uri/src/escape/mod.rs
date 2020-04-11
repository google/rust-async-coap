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

//! # URI percent encoding/decoding ("URI Escaping")
//!
//! This module was written before the author was aware of the [`percent-encode`] crate.
//! Nonetheless, it has some convenient features that are not present in that crate,
//! so it remains in the X-URI crate for now.
//!
//! [`percent-encode`]: https://docs.rs/percent-encoding/2.0.0/percent_encoding/index.html
//!
//! It focuses solely on percent encoding/decoding of UTF8-encoded strings, treating all
//! percent-encoded strings that would otherwise decode to invalid UTF8 as themselves invalid.
//!
//! The primary interface to encoding and decoding is a provided trait that extends `str`:
//! [`StrExt`].
//!
//! Percent encoding is performed by [`escape_uri()`], which returns an iterator that
//! escapes the string. Likewise, percent decoding is performed by [`unescape_uri()`].
//!
//! As a special case, the trait also provides [`unescape_uri_in_place()`], which performs
//! in-place percent-decoding for a mutable string slice.
//!
//! # Usage Patterns
//!
//! The iterators returned by [`escape_uri()`] and [`unescape_uri()`] both implement
//! [`core::fmt::Display`], and thus also implement [`std::string::ToString`]:
//!
//! ```
//! use async_coap_uri::prelude::*;
//! let escaped_string = "This needs escaping".escape_uri().to_string();
//!
//! assert_eq!(&escaped_string, "This%20needs%20escaping");
//! ```
//!
//! Both methods also implent `From<Cow<str>>`:
//!
//! ```
//! # use async_coap_uri::prelude::*;
//! use std::borrow::Cow;
//! use std::convert::From;
//! let escaped_cow_str = Cow::from("This needs escaping?+3".escape_uri());
//!
//! assert_eq!(&escaped_cow_str, "This%20needs%20escaping%3F+3");
//! ```
//!
//! # Changing Behavior
//!
//! There is no one-size-fits-all escaping strategy for URIs: Some parts need to be excaped
//! differently than others. For example, *path segments* must have the `?` character escaped to
//! `%3F`, but this character is perfectly acceptable in the *query component*. Also, query
//! components have historically escaped the space character (` `) to the plus (`+`)
//! character, so pluses need to be escaped to `%2B`.
//!
//! By default, [`StrExt::escape_uri`] produces an iterator suitable for encoding *path segments*,
//! but other cases are handled by calling a modifier method on the [`EscapeUri`] iterator:
//!
//! ```
//! # use async_coap_uri::prelude::*;
//! let escaped_string = "This needs escaping?+3".escape_uri().for_query().to_string();
//!
//! assert_eq!(&escaped_string, "This+needs+escaping?%2B3");
//! ```
//!
//! The [`EscapeUri`] iterator also provides the modifier methods `for_fragment()` and `full()`
//! for encoding URI fragments and performing full percent encoding, respectively.
//!
//! ## Skipping Slashes
//!
//! The [`UnescapeUri`] iterator provides a modifier method for assisting in decoding the
//! entire URI *path component* (as opposed to individual *path segments*) where encoded
//! slashes (`%2F`) are not decoded, preserving the hierarchy:
//!
//! ```
//! # use async_coap_uri::prelude::*;
//! let escaped_string = "/this/p%20a%20t%20h/has%2Fextra/segments";
//!
//! let unescaped = escaped_string.unescape_uri().to_string();
//! assert_eq!(&unescaped, "/this/p a t h/has/extra/segments");
//!
//! let unescaped = escaped_string.unescape_uri().skip_slashes().to_string();
//! assert_eq!(&unescaped, "/this/p a t h/has%2Fextra/segments");
//! ```
//!
//! # Handling Encoding Errors
//!
//! While [`escape_uri()`] cannot fail, an escaped string can contain errors.
//! In situations where escaped characters cannot be properly decoded, the
//! [`unescape_uri()`] iterator will by default insert replacement characters
//! where errors are detected:
//!
//!  * Illegal escaped UTF8 errors are replaced with [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD] (`�`).
//!  * Escaped ASCII control codes are decoded as [Unicode Control Pictures] like `␀` and `␊`.
//!  * Unescaped ASCII control codes are dropped entirely.
//!
//! In cases where this is not appropriate, the iterator for [`unescape_uri()`] ([`UnescapeUri`])
//! provides the following methods:
//!
//! * [`first_error()`]\: Returns the location of the first detected encoding error, or `None` if
//!   there are no encoding errors.
//! * [`try_to_string()`]\: Returns an unescaped [`String`] only if no encoding errors were present.
//! * [`try_to_cow()`]\: Returns an unescaped [`Cow<str>`] only if no encoding errors were present.
//!
//! [U+FFFD]: core::char::REPLACEMENT_CHARACTER
//! [Unicode Control Pictures]: https://www.unicode.org/charts/PDF/U2400.pdf
//! [`escape_uri()`]: #method.escape_uri
//! [`unescape_uri()`]: #method.unescape_uri
//! [`unescape_uri_in_place()`]: #method.unescape_uri_in_place
//! [`first_error()`]: struct.UnescapeUri.html#method.first_error
//! [`try_to_string()`]: struct.UnescapeUri.html#method.try_to_string
//! [`try_to_cow()`]: struct.UnescapeUri.html#method.try_to_cow
//! [`EscapeUri`]: struct.EscapeUri.html
//! [`UnescapeUri`]: struct.UnescapeUri.html
//!
mod escape_uri;
pub use escape_uri::*;

mod unescape_uri;
pub use unescape_uri::*;

#[cfg(test)]
mod test;

/// Trait for `str` adding URI percent encoding/decoding
///
/// See the [module-level](index.html) documentation for more details.
///
pub trait StrExt {
    /// Gets an iterator that performs general-purpose URI percent-encoding.
    ///
    /// By default, all characters described by [`IETF-RFC3986`] as `pchar`s will be escaped,
    /// which is appropriate for escaping path segments.
    /// This behavior can be modified by appending the following modifiers:
    ///
    /// * [`full()`]: Escapes all characters except those which are `unreserved`.
    /// * [`for_query()`]: Escaping appropriate for the query component.
    /// * [`for_fragment()`]: Escaping appropriate for the fragment component.
    ///
    /// The returned iterator will escape ASCII control characters.
    ///
    /// [`full()`]: struct.EscapeUri#method.full
    /// [`for_query()`]: struct.EscapeUri#method.for_query
    /// [`for_fragment()`]: struct.EscapeUri#method.for_fragment
    fn escape_uri(&self) -> EscapeUri<'_, EscapeUriSegment>;

    /// Gets an iterator that performs URI percent-decoding.
    ///
    /// By default, when the iterator encounters an error the behavior is as follows:
    ///
    /// * Unescaped ASCII control codes are dropped.
    /// * Escaped ASCII control codes are converted to [Unicode Control Pictures] (i.e. `%00` => `␀`)
    /// * Bad percent-escape sequences (like `"%Foo"`) are replaced with [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD]
    /// * Incomplete UTF8 sequences (like `"%E2%82"`) are replaced with [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD]
    /// * Invalid UTF8 sequences (like `"%E2%82%E2"`) are replaced with [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD]
    ///
    /// [U+FFFD]: core::char::REPLACEMENT_CHARACTER
    /// [Unicode Control Pictures]: https://www.unicode.org/charts/PDF/U2400.pdf
    fn unescape_uri(&self) -> UnescapeUri<'_>;

    /// **Experimental:** Unescapes the given mutable string in-place, returning a subset of
    /// the mutable slice even if it contains encoding errors or illegal characters.
    ///
    /// The behavior upon encountering errors is identical to that of
    /// [`unescape_uri()`](#method.unescape_uri).
    fn unescape_uri_in_place(&mut self) -> &mut str;
}

impl StrExt for str {
    fn escape_uri(&self) -> EscapeUri<'_, EscapeUriSegment> {
        EscapeUri {
            iter: self.as_bytes().iter(),
            state: EscapeUriState::Normal,
            needs_escape: EscapeUriSegment,
        }
    }

    fn unescape_uri(&self) -> UnescapeUri<'_> {
        UnescapeUri {
            iter: self.chars(),
            iter_index: 0,
            next_c: None,
            had_error: false,
            skip_slashes: false,
        }
    }

    fn unescape_uri_in_place(&mut self) -> &mut str {
        let mut ptr = self.as_mut_ptr();
        let iter = self.unescape_uri();

        for c in iter {
            let mut buf = [0u8; 4];

            for i in 0..c.encode_utf8(&mut buf).len() {
                unsafe {
                    // SAFETY: The correctness of this code depends on the unescape
                    //         iterator always being either at the same place or ahead
                    //         of `ptr`. If this ever turns out to not be the case,
                    //         the result will be corrupt.
                    *ptr = buf[i];
                    ptr = ptr.offset(1);
                }
            }
        }

        let len = (ptr as usize) - (self.as_mut_ptr() as usize);

        &mut self[..len]
    }
}
