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

/// Sized, heap-allocated string type containing either a URI or a relative-reference.
///
/// Similar to `String`, but with additional guarantees on internal structure.
/// The unsized counterpart is `UriRef`.
///
/// This type implements [`std::ops::Deref<UriRef>`], so you can also use all of the
/// methods from [`UriRef`](crate::UriRef) on this type.
#[derive(Clone, Eq, Hash)]
pub struct UriRefBuf(pub(super) String);

_impl_uri_buf_traits_base!(UriRefBuf, UriRef);

impl Default for UriRefBuf {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for UriRefBuf {
    type Target = UriRef;

    fn deref(&self) -> &Self::Target {
        self.as_uri_ref()
    }
}

impl AsRef<String> for UriRefBuf {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl std::fmt::Display for UriRefBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.write_to(f)
    }
}

/// # Constructors
impl UriRefBuf {
    /// Creates a new, empty [`UriRefBuf`].
    pub fn new() -> UriRefBuf {
        UriRefBuf(String::new())
    }

    /// Creates a new, empty [`UriRefBuf`] with a capacity of `capacity`.
    pub fn with_capacity(capacity: usize) -> UriRefBuf {
        UriRefBuf(String::with_capacity(capacity))
    }

    /// Attempts to create a new [`UriRefBuf`] from a string reference.
    pub fn from_str<S: AsRef<str> + Copy>(s: S) -> Result<Self, ParseError> {
        let str_ref = s.as_ref();
        UriRef::from_str(str_ref)?;
        Ok(UriRefBuf(str_ref.to_string()))
    }

    /// Attempts to create a new [`UriRefBuf`] from a [`String`].
    pub fn from_string(s: String) -> Result<Self, ParseError> {
        UriRef::from_str(s.as_str())?;
        Ok(UriRefBuf(s))
    }
}

/// # Conversions
impl UriRefBuf {
    /// Borrows a string slice containing this URI reference.
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Borrows a [`UriRef`] slice containing this URI reference.
    #[inline(always)]
    pub fn as_uri_ref(&self) -> &UriRef {
        unsafe { UriRef::from_str_unchecked(self.as_str()) }
    }

    /// Borrows a mutable [`UriRef`] slice containing this URI reference.
    #[inline(always)]
    pub fn as_mut_uri_ref(&mut self) -> &mut UriRef {
        unsafe { UriRef::from_str_unchecked_mut(self.as_mut_str()) }
    }
}

/// # Manipulation
impl UriRefBuf {
    /// Removes fragment component, if present.
    pub fn truncate_fragment(&mut self) {
        if let Some(i) = self.fragment_start() {
            self.0.truncate(i)
        }
    }

    /// Truncates the authority, path, query, and fragment components of a `UriRefBuf`.
    pub fn truncate_heir_part(&mut self) {
        self.0.truncate(self.heir_part_start())
    }

    /// Removes query and fragment components, if present.
    /// E.g.:
    ///
    /// ```
    /// # use async_coap_uri::*;
    /// # fn main() {
    ///     let mut uri = uri_ref!("http://example.com/blah/bleh?query").to_uri_ref_buf();
    ///     uri.truncate_query();
    ///     assert_eq!("http://example.com/blah/bleh", uri.as_str());
    /// # }
    /// ```
    /// ```
    /// # use async_coap_uri::*;
    /// # fn main() {
    ///     let mut uri = uri_ref!("http://example.com/blah/bleh#foobar").to_uri_ref_buf();
    ///     uri.truncate_query();
    ///     assert_eq!("http://example.com/blah/bleh", uri.as_str());
    /// # }
    /// ```
    /// ```
    /// # use async_coap_uri::*;
    /// # fn main() {
    ///     let mut uri = uri_ref!("another?foo#bar").to_uri_ref_buf();
    ///     uri.truncate_query();
    ///     assert_eq!("another", uri.as_str());
    /// # }
    /// ```
    pub fn truncate_query(&mut self) {
        if let Some(i) = self.query_start().or(self.fragment_start()) {
            self.0.truncate(i)
        }
    }

    /// Truncates the path, query, and fragments components of a `UriRefBuf`.
    pub fn truncate_path(&mut self) {
        self.0.truncate(self.path_start());
    }

    /// Removes the last path component (up to, but not including, the last slash), along with
    /// the query and fragment components, if present.
    /// E.g.:
    ///
    /// ```
    /// # use async_coap_uri::*;
    /// # fn main() {
    ///     let mut uri = uri_ref!("http://example.com/blah/bleh").to_uri_ref_buf();
    ///     uri.truncate_resource();
    ///     assert_eq!("http://example.com/blah/", uri.as_str());
    /// # }
    /// ```
    /// ```
    /// # use async_coap_uri::*;
    /// # fn main() {
    ///     let mut uri = uri_ref!("http://example.com/blah/").to_uri_ref_buf();
    ///     uri.truncate_resource();
    ///     assert_eq!("http://example.com/blah/", uri.as_str());
    /// # }
    /// ```
    /// ```
    /// # use async_coap_uri::*;
    /// # fn main() {
    ///     let mut uri = uri_ref!("foo#bar").to_uri_ref_buf();
    ///     uri.truncate_resource();
    ///     assert_eq!("", uri.as_str());
    /// # }
    /// ```
    /// * `http://example.com/blah/bleh` becomes `http://example.com/blah/`
    /// * `http://example.com/blah/` becomes `http://example.com/blah/`
    /// * `foo#bar` becomes ``
    pub fn truncate_resource(&mut self) {
        self.truncate_query();
        let path_start = self.as_uri_ref().path_start();

        if let Some(i) = self.as_str().rfind('/') {
            if i + 1 > path_start {
                self.0.truncate(i + 1);
            }
        } else if path_start == 0 {
            self.clear();
        }
    }

    /// Removes the last path item, along with
    /// the query and fragment components, if present.
    ///
    /// This method will only result in an empty path if the path was empty to begin with.
    ///
    /// E.g.:
    ///
    /// ```
    /// # use async_coap_uri::*;
    /// # fn main() {
    ///     let mut uri = uri_ref!("http://example.com/blah/bleh").to_uri_ref_buf();
    ///     uri.truncate_last_path_segment();
    ///     assert_eq!("http://example.com/blah/", uri.as_str());
    ///     uri.truncate_last_path_segment();
    ///     assert_eq!("http://example.com/", uri.as_str());
    ///     uri.truncate_last_path_segment();
    ///     assert_eq!("http://example.com/", uri.as_str());
    /// # }
    /// ```
    /// ```
    /// # use async_coap_uri::*;
    /// # fn main() {
    ///     let mut uri = uri_ref!("foo#bar").to_uri_ref_buf();
    ///     uri.truncate_last_path_segment();
    ///     assert_eq!("./", uri.as_str());
    /// # }
    /// ```
    /// * `http://example.com/blah/bleh` becomes `http://example.com/blah/`
    /// * `http://example.com/blah/` becomes `http://example.com/`
    /// * `foo#bar` becomes ``
    pub fn truncate_last_path_segment(&mut self) {
        let path_start = self.as_uri_ref().path_start();
        let path_end = self.as_uri_ref().path_end();

        if path_start == path_end {
            return;
        }

        self.truncate_query();

        let mut s = self.as_str();

        // Trim off any trailing slash (but only 1)
        if s.ends_with('/') {
            s = &s[..s.len() - 1];
        }

        if let Some(i) = s.rfind('/') {
            if i + 1 > path_start {
                self.0.truncate(i + 1);
            }
        } else if path_start == 0 {
            self.clear();
        }

        if self.raw_path().is_empty() {
            // The empty URI has special behaviors that we don't
            // want here, so make ourselves "./" here instead.
            self.0.push_str("./");
        }
    }

    /// Completely clears this `UriRefBuf`, leaving it empty.
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Adds a trailing slash to the path if there isn't a trailing slash already present.
    pub fn add_trailing_slash(&mut self) -> bool {
        if self.is_empty() {
            self.0.push_str("./");
            true
        } else {
            let path_end = self.path_end();
            if path_end > 0 && &self[path_end - 1..path_end] == "/" {
                false
            } else {
                self.0.insert(path_end, '/');
                true
            }
        }
    }

    /// Adds a leading slash to the path if there isn't one already present.
    pub fn add_leading_slash(&mut self) -> bool {
        let path_begin = self.path_start();
        if self.len() > 0 && &self[path_begin..path_begin + 1] == "/" {
            false
        } else {
            self.0.insert(path_begin, '/');
            true
        }
    }

    /// Using this URI-reference as the base, performs "relative resolution" to the given instance
    /// implementing [`AnyUriRef`], updating the content of this `UriRefBuf` with the result.
    pub fn resolve<T: AnyUriRef + ?Sized>(&mut self, dest: &T) -> Result<(), ResolveError> {
        if !dest.is_empty() {
            *self = self.resolved(dest)?;
        }
        Ok(())
    }

    /// Percent-encodes and appends the given path segment to this URI-reference,
    /// truncating any existing query or fragment in the process.
    ///
    /// If this URI-reference isn't empty and doesn't end with a slash, one
    /// is first added. A trailing slash will be appended depending on the value
    /// of the `trailing_slash` argument.
    ///
    /// # Special Cases
    ///
    /// Calling `x.push_path_segment(".", false)` will do nothing.
    ///
    /// Calling `x.push_path_segment(".", true)` is the same as calling `x.add_trailing_slash()`.
    ///
    /// Calling `x.push_path_segment("..", _)` is the same as calling
    /// `x.truncate_last_path_segment()`. In this case the value of `trailing_slash` is ignored.
    ///
    /// # Example
    ///
    /// ```
    /// # use async_coap_uri::*;
    /// let mut uri = uri_ref!("http://example.com").to_uri_ref_buf();
    ///
    /// uri.push_path_segment("foobar", false);
    /// assert_eq!(uri, uri_ref!("http://example.com/foobar"));
    ///
    /// uri.push_path_segment("a/b/c", true);
    /// assert_eq!(uri, uri_ref!("http://example.com/foobar/a%2Fb%2Fc/"));
    /// ```
    pub fn push_path_segment(&mut self, segment: &str, trailing_slash: bool) {
        if segment == "." {
            if trailing_slash {
                self.add_trailing_slash();
            } else if self.is_empty() {
                // this is the only case where we actually add the dot.
                self.0.push_str(".");
            }
            return;
        }

        if segment == ".." {
            self.truncate_last_path_segment();
            return;
        }

        self.truncate_query();

        if !self.is_empty() {
            self.add_trailing_slash();
        }

        self.0.extend(segment.escape_uri());

        if trailing_slash {
            self.add_trailing_slash();
        }
    }

    /// Percent-encodes and appends the given query item to this instance,
    /// truncating any existing fragment in the process.
    ///
    /// If no query is present, the query item is preceded with a '?' to
    /// indicate the start of the query component. Otherwise, this method
    /// uses `&` to separate query items.
    ///
    /// This method follows the common convention where spaces are encoded
    /// as `+` characters instead of `%20`.
    ///
    /// # Example
    ///
    /// ```
    /// # use async_coap_uri::*;
    /// let mut uri = uri_ref!("http://example.com").to_uri_ref_buf();
    ///
    /// uri.push_query_item("foobar");
    /// assert_eq!(uri, uri_ref!("http://example.com?foobar"));
    ///
    /// uri.push_query_item("q=a vast query");
    /// assert_eq!(uri, uri_ref!("http://example.com?foobar&q=a+vast+query"));
    /// ```
    pub fn push_query_item(&mut self, item: &str) {
        self.truncate_fragment();
        if let Some(_) = self.query_start() {
            self.0.push('&');
        } else {
            self.0.push('?');
        }
        self.0.extend(item.escape_uri().for_query());
    }

    /// Percent-encodes and appends the given query key/value pair to this URI-reference,
    /// truncating any existing fragment in the process.
    ///
    /// If no query is present, the query item is preceded with a '?' to
    /// indicate the start of the query component. Otherwise, this method
    /// uses `&` to separate query items.
    ///
    /// This method follows the common convention where spaces are encoded
    /// as `+` characters instead of `%20`.
    ///
    /// # Example
    ///
    /// ```
    /// # use async_coap_uri::*;
    /// let mut uri = uri_ref!("http://example.com").to_uri_ref_buf();
    ///
    /// uri.push_query_key_value("foo","bar");
    /// assert_eq!(uri, uri_ref!("http://example.com?foo=bar"));
    ///
    /// uri.push_query_key_value("q","a vast query");
    /// assert_eq!(uri, uri_ref!("http://example.com?foo=bar&q=a+vast+query"));
    /// ```
    pub fn push_query_key_value(&mut self, key: &str, value: &str) {
        self.truncate_fragment();
        if let Some(_) = self.query_start() {
            self.0.push('&');
        } else {
            self.0.push('?');
        }
        self.0.extend(key.escape_uri().for_query());
        self.0.push('=');
        self.0.extend(value.escape_uri().for_query());
    }

    /// Replaces the path, query, and fragment with that from `rel`.
    pub fn replace_path(&mut self, rel: &RelRef) {
        self.truncate_path();
        if !rel.starts_with(|c| c == '/' || c == '?' || c == '#') {
            self.add_trailing_slash();
        }

        self.0.push_str(rel.as_str());
    }
}

/// # Unsafe Methods
///
/// `UriRefBuf` needs some unsafe methods in order to function properly. This section is where
/// they are all located.
impl UriRefBuf {
    /// Unchecked version of [`UriRefBuf::from_string`].
    ///
    /// # Safety
    ///
    /// This method is marked as unsafe because it allows you to construct a `UriRefBuf` with
    /// a value that is not a well-formed URI reference.
    pub unsafe fn from_string_unchecked(s: String) -> UriRefBuf {
        UriRefBuf(s)
    }

    /// Borrows a mutable string slice containing this URI reference.
    ///
    /// This method is marked as unsafe because it allows you to change the
    /// content of the URI reference without any checks on syntax.
    #[inline(always)]
    pub unsafe fn as_mut_str(&mut self) -> &mut str {
        self.0.as_mut_str()
    }

    /// Borrows a mutable [`String`] reference containing this URI reference.
    ///
    /// This method is marked as unsafe because it allows you to change the
    /// content of the URI reference without any checks on syntax.
    pub unsafe fn as_mut_string_ref(&mut self) -> &mut String {
        &mut self.0
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! inherits_uri_ref_buf {
    ($BUF:ident) => {
        /// # Methods inherited from [`UriRefBuf`]
        impl $BUF {
            /// Returns a string slice for this instance.
            #[inline(always)]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            /// Returns a mutable string slice (`&mut str`) for this instance.
            ///
            /// ## Safety
            ///
            /// This method is not safe because this type makes guarantees about
            /// the structure of the content it contains, which may be violated by
            /// using this method.
            #[inline(always)]
            pub unsafe fn as_mut_str(&mut self) -> &mut str {
                self.0.as_mut_str()
            }

            /// Borrows a reference to this mutable instance as a mutable URI-Reference
            /// (`&mut UriRef`).
            #[inline(always)]
            pub fn as_mut_uri_ref(&mut self) -> &mut UriRef {
                self.0.as_mut_uri_ref()
            }

            /// Removes the authority, path, query, and fragment components, if present.
            #[inline(always)]
            pub fn truncate_heir_part(&mut self) {
                self.0.truncate_heir_part()
            }

            /// Removes the path, query, and fragment components, if present.
            #[inline(always)]
            pub fn truncate_path(&mut self) {
                self.0.truncate_path()
            }

            /// Removes the query, and fragment components, if present.
            #[inline(always)]
            pub fn truncate_query(&mut self) {
                self.0.truncate_query()
            }

            /// Removes fragment component, if present.
            #[inline(always)]
            pub fn truncate_fragment(&mut self) {
                self.0.truncate_fragment()
            }

            /// Removes the last path component (up to, but not including, the last slash),
            /// along with the query and fragment components, if present.
            ///
            /// See [`UriRefBuf::truncate_resource`] for more information.
            #[inline(always)]
            pub fn truncate_resource(&mut self) {
                self.0.truncate_resource()
            }

            /// Removes the last path item, along with
            /// the query and fragment components, if present.
            ///
            /// See [`UriRefBuf::truncate_last_path_segment`] for more information.
            #[inline(always)]
            pub fn truncate_last_path_segment(&mut self) {
                self.0.truncate_last_path_segment()
            }

            /// Adds a trailing slash to the path if there isn't a trailing slash already present.
            #[inline(always)]
            pub fn add_trailing_slash(&mut self) -> bool {
                self.0.add_trailing_slash()
            }

            /// Adds a leading slash to the path if there isn't one already present.
            #[inline(always)]
            pub fn add_leading_slash(&mut self) -> bool {
                self.0.add_leading_slash()
            }

            /// Percent-encodes and appends the given path segment to this instance,
            /// truncating any existing query or fragment in the process.
            ///
            /// If this instance isn't empty and doesn't end with a slash, one
            /// is first added. A trailing slash will be appended depending on the value
            /// of the `trailing_slash` argument.
            ///
            #[inline(always)]
            pub fn push_path_segment(&mut self, segment: &str, trailing_slash: bool) {
                self.0.push_path_segment(segment, trailing_slash)
            }

            /// Percent-encodes and appends the given query item to this instance,
            /// truncating any existing fragment in the process.
            ///
            /// If no query is present, the query item is preceded with a '?' to
            /// indicate the start of the query component. Otherwise, this method
            /// uses `&` to separate query items.
            ///
            /// This method follows the common convention where spaces are encoded
            /// as `+` characters instead of `%20`.
            #[inline(always)]
            pub fn push_query_item(&mut self, item: &str) {
                self.0.push_query_item(item)
            }

            /// Percent-encodes and appends the given query key/value pair to this URI-reference,
            /// truncating any existing fragment in the process.
            ///
            /// If no query is present, the query item is preceded with a '?' to
            /// indicate the start of the query component. Otherwise, this method
            /// uses `&` to separate query items.
            ///
            /// This method follows the common convention where spaces are encoded
            /// as `+` characters instead of `%20`.
            #[inline(always)]
            pub fn push_query_key_value(&mut self, key: &str, value: &str) {
                self.0.push_query_key_value(key, value)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        assert!(UriRefBuf::from_str("http://example.com/").is_ok());
    }

    #[test]
    fn push_path_segment() {
        let mut uri = iuri_ref!("").to_uri_ref_buf();

        uri.push_path_segment(".", false);
        assert_eq!(uri, iuri_ref!("."));

        let mut uri = iuri_ref!("").to_uri_ref_buf();

        uri.push_path_segment("foobar", false);
        assert_eq!(uri, iuri_ref!("foobar"));

        uri.push_path_segment("a/b/c", true);
        assert_eq!(uri, iuri_ref!("foobar/a%2Fb%2Fc/"));

        uri.push_path_segment(".", true);
        assert_eq!(uri, iuri_ref!("foobar/a%2Fb%2Fc/"));

        uri.push_path_segment("..", false);
        assert_eq!(uri, iuri_ref!("foobar/"));

        uri.push_path_segment("..", false);
        assert_eq!(uri, iuri_ref!("./"));
    }

    #[test]
    fn add_trailing_slash() {
        let mut uri = iuri_ref!("example/").to_uri_ref_buf();
        assert_eq!(false, uri.add_trailing_slash());

        let mut uri = iuri_ref!("example").to_uri_ref_buf();
        assert_eq!(true, uri.add_trailing_slash());
        assert_eq!(iuri_ref!("example/"), &uri);

        let mut uri = iuri_ref!("example?").to_uri_ref_buf();
        assert_eq!(true, uri.add_trailing_slash());
        assert_eq!(iuri_ref!("example/?"), &uri);

        let mut uri = iuri_ref!("example#").to_uri_ref_buf();
        assert_eq!(true, uri.add_trailing_slash());
        assert_eq!(iuri_ref!("example/#"), &uri);

        let mut uri = iuri_ref!("example?/#/").to_uri_ref_buf();
        assert_eq!(true, uri.add_trailing_slash());
        assert_eq!(iuri_ref!("example/?/#/"), &uri);

        let mut uri = iuri_ref!("/e/x/a/m/p/l/e?/#/").to_uri_ref_buf();
        assert_eq!(true, uri.add_trailing_slash());
        assert_eq!(iuri_ref!("/e/x/a/m/p/l/e/?/#/"), &uri);
    }

    #[test]
    fn add_leading_slash() {
        let mut uri = iuri_ref!("/example").to_uri_ref_buf();
        assert_eq!(false, uri.add_leading_slash());

        let mut uri = iuri_ref!("example").to_uri_ref_buf();
        assert_eq!(true, uri.add_leading_slash());
        assert_eq!(iuri_ref!("/example"), &uri);

        let mut uri = iuri_ref!("example?").to_uri_ref_buf();
        assert_eq!(true, uri.add_leading_slash());
        assert_eq!(iuri_ref!("/example?"), &uri);

        let mut uri = iuri_ref!("example#").to_uri_ref_buf();
        assert_eq!(true, uri.add_leading_slash());
        assert_eq!(iuri_ref!("/example#"), &uri);

        let mut uri = iuri_ref!("example?/#/").to_uri_ref_buf();
        assert_eq!(true, uri.add_leading_slash());
        assert_eq!(iuri_ref!("/example?/#/"), &uri);

        let mut uri = iuri_ref!("e/x/a/m/p/l/e/?/#/").to_uri_ref_buf();
        assert_eq!(true, uri.add_leading_slash());
        assert_eq!(iuri_ref!("/e/x/a/m/p/l/e/?/#/"), &uri);
    }
}
