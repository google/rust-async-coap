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

/// Unsized string-slice type guaranteed to contain a well-formed [IETF-RFC3986] URI
/// *or* [network path](index.html#network-path-support).
///
/// The sized counterpart is [`crate::UriBuf`].
///
/// You can create static constants with this class by using the [`uri!`] macro:
///
/// ```
/// # use async_coap_uri::*;
/// let uri = uri!("http://example.com/test");
/// let components = uri.components();
///
/// assert_eq!(Some("http"),        components.scheme());
/// assert_eq!(Some("example.com"), components.raw_host());
/// assert_eq!(None,                components.port());
/// assert_eq!("/test",             components.raw_path());
/// ```
///
/// [`uri!`]: macro.uri.html
/// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
#[derive(Eq, Hash)]
pub struct Uri(UriRef);

_impl_uri_traits_base!(Uri);

impl Deref for Uri {
    type Target = UriRef;

    fn deref(&self) -> &Self::Target {
        self.as_uri_ref()
    }
}

impl AsRef<UriRef> for Uri {
    fn as_ref(&self) -> &UriRef {
        &self.0
    }
}

impl AnyUriRef for Uri {
    unsafe fn write_to_unsafe<T: core::fmt::Write + ?Sized>(
        &self,
        write: &mut T,
    ) -> Result<(), core::fmt::Error> {
        write.write_str(self.as_str())
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Determines what kind of URI this is.
    ///
    /// This function may return any one of the following values:
    ///
    /// * [`UriType::Uri`](enum.UriType.html#variant.Uri)
    /// * [`UriType::UriNoAuthority`](enum.UriType.html#variant.UriNoAuthority)
    /// * [`UriType::UriCannotBeABase`](enum.UriType.html#variant.UriCannotBeABase)
    /// * [`UriType::NetworkPath`](enum.UriType.html#variant.NetworkPath)
    fn uri_type(&self) -> UriType {
        if self.0.starts_with("//") {
            UriType::NetworkPath
        } else {
            let i = self.find(':').expect("Uri contract broken");
            if self[i..].starts_with("://") {
                UriType::Uri
            } else if self[i..].starts_with(":/") {
                UriType::UriNoAuthority
            } else {
                UriType::UriCannotBeABase
            }
        }
    }

    fn components(&self) -> UriRawComponents<'_> {
        self.0.components()
    }
}

impl std::fmt::Display for Uri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.write_to(f)
    }
}

impl Uri {
    /// Attempts to convert a string slice into a [`&Uri`](Uri), returning `Err(ParseError)`
    /// if the string slice contains data that is not a valid URI.
    ///
    /// Example:
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(Uri::from_str("http://example.com"), Ok(uri!("http://example.com")));
    /// assert!(Uri::from_str("/a/b/c").is_err());
    /// ```
    pub fn from_str(input: &str) -> Result<&Uri, ParseError> {
        // TODO(#10): Replace this with an optimized validity check.
        //       We are currently using `UriRawComponents::from_str()` as a crutch here;
        //       it includes extraneous operations that are not related to verifying if a
        //       URI is well-formed.
        if UriRawComponents::from_str(input)?
            .uri_type()
            .can_borrow_as_uri()
        {
            Ok(unsafe { Self::from_str_unchecked(input) })
        } else {
            Err(ParseError::new("Not a URI", None))
        }
    }

    /// Determines if the given string can be considered a valid URI.
    ///
    /// The key difference between this and [`crate::UriRef::is_str_valid`] is that this
    /// function will return false for [relative-references] like `/a/b/c`, whereas
    /// `UriRef::is_str_valid` would return true.
    ///
    /// Example:
    ///
    /// ```
    /// use async_coap_uri::Uri;
    /// assert!(Uri::is_str_valid("http://example.com"));
    /// assert!(!Uri::is_str_valid("/a/b/c"));
    /// assert!(!Uri::is_str_valid("Not a URI"));
    /// ```
    /// [relative-reference]: https://tools.ietf.org/html/rfc3986#section-4.2
    pub fn is_str_valid<S: AsRef<str>>(s: S) -> bool {
        let str_ref = s.as_ref();
        // TODO(#10): Replace this with an optimized validity check.
        //       We are currently using `UriRawComponents::from_str()` as a crutch here;
        //       it includes extraneous operations that are not related to verifying if a
        //       URI is well-formed.
        if let Ok(components) = UriRawComponents::from_str(str_ref) {
            if components.uri_type().can_borrow_as_uri() {
                return true;
            }
        }

        false
    }

    /// Reinterpret this [`&Uri`][Uri] as a [`&UriRef`][UriRef].
    #[inline(always)]
    pub const fn as_uri_ref(&self) -> &UriRef {
        &self.0
    }

    /// Copy the content of this [`&Uri`][Uri] into a new [`UriBuf`] and return it.
    pub fn to_uri_buf(&self) -> UriBuf {
        unsafe { UriBuf::from_string_unchecked(self.to_string()) }
    }
}

/// ## Splitting
impl Uri {
    /// Splits this URI into the base and relative portions.
    pub fn split(&self) -> (&Uri, &RelRef) {
        let (uri_base, uri_rel) = self.0.split();
        (uri_base.unwrap(), uri_rel)
    }
}

/// ## Trimming
impl Uri {
    /// Returns this URI without a fragment.
    ///
    /// ## Examples
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(uri!("http://a/#frag").trim_fragment(),  uri!("http://a/"));
    /// assert_eq!(uri!("//a/b/c?blah#frag").trim_fragment(), uri!("//a/b/c?blah"));
    /// ```
    pub fn trim_fragment(&self) -> &Uri {
        unsafe { Uri::from_str_unchecked(self.0.trim_fragment().as_str()) }
    }

    /// Returns this URI without a query or fragment.
    ///
    /// ## Examples
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(uri!("//foo/?bar").trim_query(),      uri!("//foo/"));
    /// assert_eq!(uri!("http://a/#frag").trim_query(),  uri!("http://a/"));
    /// ```
    pub fn trim_query(&self) -> &Uri {
        unsafe { Uri::from_str_unchecked(self.0.trim_query().as_str()) }
    }

    /// Returns this URI without a path, query, or fragment.
    ///
    /// ## Examples
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(uri!("//foo/?bar").trim_path(),      uri!("//foo"));
    /// assert_eq!(uri!("http://a/#frag").trim_path(),  uri!("http://a"));
    /// ```
    pub fn trim_path(&self) -> &Uri {
        unsafe { Uri::from_str_unchecked(self.0.trim_path().as_str()) }
    }

    /// Returns this URI without the trailing part of the path that would be
    /// removed during relative-reference resolution.
    ///
    /// ## Examples
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(uri!("//foo/?bar").trim_resource(),      uri!("//foo/"));
    /// assert_eq!(uri!("http://a/#frag").trim_resource(),  uri!("http://a/"));
    /// ```
    pub fn trim_resource(&self) -> &Uri {
        unsafe { Uri::from_str_unchecked(self.0.trim_resource().as_str()) }
    }
}

/// # Unsafe Methods
///
/// `Uri` needs some unsafe methods in order to function properly. This section is where
/// they are all located.
impl Uri {
    /// Creates a `Uri` slice from a string slice without checking that the content
    /// of the string slice is a valid URI.
    ///
    /// Since containing a valid URI is a fundamental guarantee of the `Uri` type, this method is
    /// `unsafe`.
    #[inline(always)]
    pub unsafe fn from_str_unchecked(s: &str) -> &Uri {
        &*(s as *const str as *const Uri)
    }

    /// Creates a mutable `Uri` slice from a mutable string slice without checking that the content
    /// of the mutable string slice is a valid URI.
    ///
    /// Since containing a valid URI is a fundamental guarantee of the `Uri` type, this method is
    /// `unsafe`.
    #[inline(always)]
    pub unsafe fn from_str_unchecked_mut(s: &mut str) -> &mut Uri {
        &mut *(s as *mut str as *mut Uri)
    }

    /// Returns this slice as a mutable `str` slice.
    ///
    /// ## Safety
    ///
    /// This is unsafe because it allows you to change the contents of the slice in
    /// such a way that would make it no longer consistent with the `Uri`'s promise
    /// that it can only ever contain a valid URI.
    #[inline(always)]
    pub unsafe fn as_mut_str(&mut self) -> &mut str {
        self.0.as_mut_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_type() {
        assert_eq!(uri!("scheme://example").uri_type(), UriType::Uri);
        assert_eq!(uri!("scheme:/example").uri_type(), UriType::UriNoAuthority);
        assert_eq!(uri!("scheme:example").uri_type(), UriType::UriCannotBeABase);
        assert_eq!(
            uri!("scheme:example/://not_a_url").uri_type(),
            UriType::UriCannotBeABase
        );
    }
}
