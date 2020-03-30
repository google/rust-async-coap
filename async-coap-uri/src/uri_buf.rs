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
use std::fmt::Write;
use std::ops::Deref;

/// Sized, heap-allocated string type guaranteed to contain a well-formed [IETF-RFC3986] URI
/// or [network path](enum.UriType.html#variant.NetworkPath).
///
/// The unsized counterpart is [`Uri`](crate::Uri).
///
/// This type implements [`std::ops::Deref<Uri>`], so you can also use all of the
/// methods from [`Uri`] on this type.
///
/// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
#[derive(Clone, Eq, Hash)]
pub struct UriBuf(pub(super) UriRefBuf);

impl_uri_buf_traits!(UriBuf, Uri);

impl Deref for UriBuf {
    type Target = Uri;

    fn deref(&self) -> &Self::Target {
        self.as_uri()
    }
}

impl AsRef<Uri> for UriBuf {
    fn as_ref(&self) -> &Uri {
        self.as_uri()
    }
}

impl From<&Uri> for UriBuf {
    fn from(x: &Uri) -> Self {
        x.to_uri_buf()
    }
}

impl std::fmt::Display for UriBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.write_to(f)
    }
}

/// # Unsafe Methods
///
/// `UriBuf` needs some unsafe methods in order to function properly. This section is where
/// they are all located.
impl UriBuf {
    /// Unchecked version of [`UriBuf::from_string`].
    ///
    /// # Safety
    ///
    /// This method is marked as unsafe because it allows you to construct a `UriBuf` with
    /// a value that is not a well-formed URI reference.
    #[inline(always)]
    pub unsafe fn from_string_unchecked(s: String) -> UriBuf {
        UriBuf(UriRefBuf::from_string_unchecked(s))
    }
}

impl UriBuf {
    /// Creates a new `UriBuf` from *unescaped* component values.
    pub fn new<Sch, Hos, Pat, Que, Frg>(
        scheme: Sch,
        host: Hos,
        port: Option<u16>,
        path: Pat,
        query: Option<Que>,
        fragment: Option<Frg>,
    ) -> UriBuf
    where
        Sch: Into<String>,
        Hos: AsRef<str>,
        Pat: AsRef<str>,
        Que: AsRef<str>,
        Frg: AsRef<str>,
    {
        let mut ret: String = Self::from_scheme_host_port(scheme, host, port).into();

        let mut path = path.as_ref();

        if path.starts_with('/') {
            path = &path[1..];
        }

        let path_segment_iter = path.split('/').filter(|seg| *seg != ".");

        for seg in path_segment_iter {
            ret.push('/');
            ret.extend(seg.escape_uri());
        }

        if let Some(query) = query {
            let mut first = true;
            ret.push('?');
            for seg in query.as_ref().split(|c| c == '&' || c == ';') {
                if first {
                    first = false;
                } else {
                    ret.push('&');
                }
                ret.extend(seg.escape_uri().for_query());
            }
        }

        if let Some(fragment) = fragment {
            ret.push('#');
            ret.extend(fragment.as_ref().escape_uri().for_fragment());
        }

        unsafe { Self::from_string_unchecked(ret) }
    }

    /// Constructs a `UriBuf` from a scheme and authority.
    ///
    /// The authority should not be percent encoded. If the given scheme contains invalid
    /// characters, this method will panic.
    ///
    /// # Example
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// let authority = "user@[2001:0db8:85a3::1%en2]:8080";
    ///
    /// let uri_buf = UriBuf::from_scheme_authority("http", authority);
    ///
    /// assert_eq!(uri_buf, uri!("http://user@[2001:0db8:85a3::1%25en2]:8080"));
    ///
    /// ```
    pub fn from_scheme_authority<Sch, Aut>(scheme: Sch, authority: Aut) -> UriBuf
    where
        Sch: Into<String>,
        Aut: AsRef<str>,
    {
        let mut ret = scheme.into();

        // Make sure that the scheme is proper. This is likely a string constant,
        // so we go ahead and panic if we detect that it is busted.
        assert_eq!(
            ret.find(|c: char| !(c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')),
            None,
            "Scheme contains invalid characters: {:?}",
            ret
        );

        ret.push_str("://");

        ret.extend(authority.as_ref().escape_uri().for_authority());

        unsafe { Self::from_string_unchecked(ret) }
    }

    /// Constructs a network path from a host and a relative reference.
    ///
    /// # Example
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// let host = "example.com";
    /// let rel_ref = rel_ref!("./foobar?q");
    ///
    /// let uri_buf = UriBuf::from_host_rel_ref(host, rel_ref);
    ///
    /// assert_eq!(uri_buf, uri!("//example.com/foobar?q"));
    ///
    /// ```
    pub fn from_host_rel_ref<Hos, RR>(host: Hos, rel_ref: RR) -> UriBuf
    where
        Hos: AsRef<str>,
        RR: AsRef<RelRef>,
    {
        let host = host.as_ref();
        let rel_ref = rel_ref
            .as_ref()
            .trim_leading_dot_slashes()
            .trim_leading_slashes();

        // UNWRAP-SAFETY: This is safe because we are fully
        // escaping the host and we already know rel_ref to
        // be well-formed.
        uri_format!("//{}/{}", host.escape_uri().full(), rel_ref).unwrap()
    }

    /// Constructs a `UriBuf` from a scheme, host and an optional port number.
    ///
    /// The host should not be percent encoded. If the given scheme contains invalid
    /// characters, this method will panic.
    pub fn from_scheme_host_port<Sch, Hos>(scheme: Sch, host: Hos, port: Option<u16>) -> UriBuf
    where
        Sch: Into<String>,
        Hos: AsRef<str>,
    {
        let mut ret = scheme.into();

        // Make sure that the scheme is proper. This is likely a string constant,
        // so we go ahead and panic if we detect that it is busted.
        assert_eq!(
            ret.find(|c: char| !(c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')),
            None,
            "Scheme contains invalid characters: {:?}",
            ret
        );

        ret.push_str("://");

        let mut host = host.as_ref();

        // Trim enclosing brackets.
        if host.starts_with('[') && host.ends_with(']') {
            host = &host[1..host.len()];
        }

        if host.find(':').is_some() {
            ret.push('[');
            ret.extend(host.escape_uri());
            ret.push(']');
        }

        if let Some(port) = port {
            write!(ret, ":{}", port).unwrap();
        }

        unsafe { Self::from_string_unchecked(ret) }
    }

    /// Attempts to create a new [`UriBuf`] from a string slice.
    pub fn from_str<S: AsRef<str>>(s: S) -> Result<UriBuf, ParseError> {
        let s = s.as_ref();
        let components = UriRawComponents::from_str(s)?;

        if components.uri_type().can_borrow_as_uri() {
            Ok(unsafe { Self::from_string_unchecked(s.to_string()) })
        } else {
            Err(ParseError::new("Missing scheme or authority", None))
        }
    }

    /// Attempts to create a new [`UriBuf`] from a [`String`].
    pub fn from_string(s: String) -> Result<UriBuf, ParseError> {
        let components = UriRawComponents::from_str(s.as_str())?;

        if components.uri_type().can_borrow_as_uri() {
            Ok(unsafe { Self::from_string_unchecked(s) })
        } else {
            Err(ParseError::new("Missing scheme or authority", None))
        }
    }

    /// Attempts to create a new [`UriBuf`] from a `UriRef` slice.
    pub fn from_uri<S: AsRef<UriRef>>(s: S) -> Option<UriBuf> {
        if s.as_ref().uri_type().can_borrow_as_uri() {
            Some(UriBuf(s.as_ref().to_uri_ref_buf()))
        } else {
            None
        }
    }
}

impl UriBuf {
    /// Borrows a [`Uri`] slice containing this URI.
    #[inline(always)]
    pub fn as_uri(&self) -> &Uri {
        unsafe { Uri::from_str_unchecked(self.as_str()) }
    }
}

/// # Manipulation
impl UriBuf {
    /// Using this URI as the base, performs "relative resolution" to the given instance
    /// implementing [`AnyUriRef`], updating the content of this `UriBuf` with the result.
    pub fn resolve<T: AnyUriRef + ?Sized>(&mut self, dest: &T) -> Result<(), ResolveError> {
        self.0.resolve(dest)
    }

    /// Replaces the path, query, and fragment with that from `rel`.
    pub fn replace_path(&mut self, rel: &RelRef) {
        self.0.replace_path(rel)
    }
}

inherits_uri_ref_buf!(UriBuf);
