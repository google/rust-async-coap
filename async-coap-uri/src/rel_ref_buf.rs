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
use std::ops::Deref;

/// Sized, heap-allocated string type guaranteed to contain a well-formed [IETF-RFC3986]
/// [relative-reference].
///
/// The unsized counterpart is [`RelRef`](crate::RelRef).
///
/// *This type cannot hold a network path*. If this type contains a path that looks like a network
/// path, it will be considered [degenerate](crate::RelRef::is_degenerate) and you will not be able
/// to losslessly convert it to a [`UriRef`](crate::UriRef) or [`UriRefBuf`](crate::UriRefBuf).
/// See ["Network Path Support"](index.html#network-path-support) for more details.
///
/// This type implements [`std::ops::Deref<RelRef>`], so you can also use all of the
/// methods from [`RelRef`] on this type.
///
/// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
/// [relative-reference]: https://tools.ietf.org/html/rfc3986#section-4.2
#[derive(Clone, Eq, Hash)]
pub struct RelRefBuf(pub(super) UriRefBuf);

impl_uri_buf_traits!(RelRefBuf, RelRef);

impl Default for RelRefBuf {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RelRefBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.write_to(f)
    }
}

impl Deref for RelRefBuf {
    type Target = RelRef;

    fn deref(&self) -> &Self::Target {
        self.as_rel_ref()
    }
}

impl AsRef<RelRef> for RelRefBuf {
    fn as_ref(&self) -> &RelRef {
        self.as_rel_ref()
    }
}

impl From<&RelRef> for RelRefBuf {
    fn from(x: &RelRef) -> Self {
        x.to_rel_ref_buf()
    }
}

/// # Constructors
impl RelRefBuf {
    /// Constructs a new, empty relative reference buffer.
    pub fn new() -> RelRefBuf {
        RelRefBuf(UriRefBuf::new())
    }

    /// Creates a new, empty [`RelRefBuf`] with a capacity of `capacity`.
    pub fn with_capacity(capacity: usize) -> RelRefBuf {
        RelRefBuf(UriRefBuf::with_capacity(capacity))
    }

    /// Attempts to create a new [`RelRefBuf`] from a string reference.
    pub fn from_str<S: AsRef<str>>(s: S) -> Result<RelRefBuf, ParseError> {
        RelRef::from_str(s.as_ref()).map(Self::from_rel_ref)
    }

    /// Attempts to create a new [`RelRefBuf`] from a [`String`].
    pub fn from_string(s: String) -> Result<RelRefBuf, ParseError> {
        if let Some(first_error) = s.as_str().unescape_uri().first_error() {
            return Err(ParseError::new(
                "Bad percent encoding or illegal characters",
                Some(first_error..s.len()),
            ));
        } else {
            let mut ret = unsafe { Self::from_string_unchecked(s) };

            ret.disambiguate();

            Ok(ret)
        }
    }

    /// Attempts to create a new [`RelRefBuf`] from a [`UriRef`] reference.
    pub fn from_uri_ref<S: AsRef<UriRef>>(s: S) -> Option<RelRefBuf> {
        s.as_ref()
            .as_rel_ref()
            .map(ToString::to_string)
            .map(|s| unsafe { Self::from_string_unchecked(s) })
    }

    /// Attempts to create a new [`RelRefBuf`] from a [`RelRef`] reference.
    pub fn from_rel_ref<S: AsRef<RelRef>>(s: S) -> RelRefBuf {
        unsafe { Self::from_string_unchecked(s.as_ref().to_string()) }
    }
}

/// # Conversions
impl RelRefBuf {
    /// Borrows a [`RelRef`] slice containing this relative reference.
    #[inline(always)]
    pub fn as_rel_ref(&self) -> &RelRef {
        unsafe { RelRef::from_str_unchecked(self.as_str()) }
    }

    /// Borrows a mutable [`RelRef`] slice containing this relative reference.
    #[inline(always)]
    pub fn as_mut_rel_ref(&mut self) -> &mut RelRef {
        unsafe { RelRef::from_str_unchecked_mut(self.as_mut_str()) }
    }
}

/// # Manipulations
impl RelRefBuf {
    /// Modifies this relative reference to be unambiguous.
    ///
    /// Specifically:
    ///
    /// * `this:that` is converted to `this%3Athat`, to avoid confusion with a URI.
    /// * `//this/that` is converted to `/.//this/that`, to avoid confusion with a network path.
    ///
    /// Note that ambiguous `RelRefBuf` instances are always displayed/formatted unambiguously,
    /// so there should be little need to ever call this method.
    pub fn disambiguate(&mut self) -> bool {
        if let Some(i) = self.colon_in_first_path_segment() {
            (self.0).0.replace_range(i..i + 1, "%3A");
            true
        } else if self.starts_with("//") {
            (self.0).0.insert_str(0, "/.");
            true
        } else {
            false
        }
    }

    /// **Experimental**: Takes ownership of the [`RelRefBuf`] and returns a `UriUnescapeBuf` that allows for
    /// in-line unescaping during iteration of path segments and query items without additional
    /// memory allocations.
    ///
    /// Example:
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// let rel_ref_buf = rel_ref!(unsafe "g:a/b/bl%c3%a5b%c3%a6r?q=g:a&q=foo&q=syltet%c3%b8y").to_rel_ref_buf();
    /// let mut unescape_buf = rel_ref_buf.into_unescape_buf();
    ///
    /// let mut query_item_iter = unescape_buf.query_items();
    ///
    /// assert_eq!(query_item_iter.next(), Some("q=g:a"));
    /// assert_eq!(query_item_iter.next(), Some("q=foo"));
    /// assert_eq!(query_item_iter.next(), Some("q=syltetøy"));
    /// assert_eq!(query_item_iter.next(), None);
    ///
    /// core::mem::drop(query_item_iter);
    ///
    /// let mut path_seg_iter = unescape_buf.path_segments();
    ///
    /// assert_eq!(path_seg_iter.next(), Some("g:a"));
    /// assert_eq!(path_seg_iter.next(), Some("b"));
    /// assert_eq!(path_seg_iter.next(), Some("blåbær"));
    /// assert_eq!(path_seg_iter.next(), None);
    /// ```
    ///
    /// Lifetimes are enforced on the returned path items, meaning the following code
    /// does not compile:
    ///
    /// ```compile_fail
    /// # use async_coap_uri::*;
    /// # fn main() {
    ///     let mut uri = rel_ref!("this:is/fun/bl%c3%a5b%c3%a6rsyltet%c3%b8y").to_owned();
    ///     let first_part;
    ///     let second_part;
    ///     {
    ///         // This line takes ownership of `uri`...
    ///         let mut path_set = uri.into_unescape_buf();
    ///         let mut iter = path_set.path_segments();
    ///
    ///         // ...so these string slices will be valid
    ///         // for the lifetime of `iter`.
    ///         first_part = iter.next().unwrap();
    ///         second_part = iter.next().unwrap();
    ///     }
    ///     // Iter has been dropped at this point, but the next
    ///     // line uses both `first_part` and `second_part`---so
    ///     // the compiler will flag an error. If it does compile,
    ///     // it represents a use-after-free error.
    ///     panic!("This should not have compiled! {} {}", first_part, second_part);
    /// # }
    /// ```
    #[must_use]
    pub fn into_unescape_buf(self) -> UriUnescapeBuf {
        UriUnescapeBuf::new(self)
    }

    /// Using this relative-reference as the base, performs "URI-reference resolution" on another
    /// relative reference, updating the content of this `RelRefBuf` with the result.
    pub fn resolve<T: AsRef<RelRef>>(&mut self, dest: T) {
        if !dest.as_ref().is_empty() {
            *self = self.resolved_rel_ref(dest);
        }
    }

    /// Completely clears this `RelRefBuf`, leaving it empty.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.0.clear()
    }
}

inherits_uri_ref_buf!(RelRefBuf);

/// # Unsafe Methods
///
/// `RelRefBuf` needs some unsafe methods in order to function properly. This section is where
/// they are all located.
impl RelRefBuf {
    /// Unchecked version of [`RelRefBuf::from_string`].
    ///
    /// # Safety
    ///
    /// This method is marked as unsafe because it allows you to construct a `RelRefBuf` with
    /// a value that is not a well-formed relative-reference.
    #[inline(always)]
    pub unsafe fn from_string_unchecked(s: String) -> RelRefBuf {
        RelRefBuf(UriRefBuf::from_string_unchecked(s))
    }
}
