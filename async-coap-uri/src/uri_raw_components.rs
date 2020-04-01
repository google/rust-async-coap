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

use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// Struct that holds parsed URI components.
///
/// Internally, all components are referenced in raw/escaped form,
/// but this type does provide methods for convenient decoded/unescaped access.
///
/// Instances of this type are usually created by calling a method named `components()`
/// on the URI type you are working with.
///
/// That this struct implements [`AnyUriRef`], allowing it to be used
/// as an argument wherever a [`AnyUriRef`] is accepted.
#[derive(Debug, Eq, Clone, Copy, PartialEq, Hash)]
pub struct UriRawComponents<'a> {
    pub(crate) scheme: Option<&'a str>,
    pub(crate) authority: Option<&'a str>,
    pub(crate) userinfo: Option<&'a str>,
    pub(crate) host: Option<&'a str>,
    pub(crate) port: Option<u16>,
    pub(crate) path: &'a str,
    pub(crate) query: Option<&'a str>,
    pub(crate) fragment: Option<&'a str>,
}

impl AnyUriRef for UriRawComponents<'_> {
    /// Note that the implementation of this method for [`UriRawComponents`] ignores
    /// the value of `self.userinfo`, `self.host`, and `self.port`; instead relying entirely
    /// on `self.authority`.
    unsafe fn write_to_unsafe<T: core::fmt::Write + ?Sized>(
        &self,
        f: &mut T,
    ) -> Result<(), core::fmt::Error> {
        // Note that everything in `self` is already escaped, so we
        // don't need to do that here.
        if let Some(scheme) = self.scheme {
            f.write_str(scheme)?;
            f.write_char(':')?;
        }

        if let Some(authority) = self.authority {
            f.write_str("//")?;
            f.write_str(authority)?;
        }

        f.write_str(self.path)?;

        if let Some(query) = self.query {
            f.write_char('?')?;
            f.write_str(query)?;
        }

        if let Some(fragment) = self.fragment {
            f.write_char('#')?;
            f.write_str(fragment)?;
        }

        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.scheme.is_none()
            && self.authority.is_none()
            && self.path.is_empty()
            && self.query.is_none()
            && self.fragment.is_none()
    }

    fn components(&self) -> UriRawComponents<'_> {
        self.clone()
    }

    fn uri_type(&self) -> UriType {
        if self.authority.is_some() {
            if self.scheme.is_some() {
                return UriType::Uri;
            } else {
                return UriType::NetworkPath;
            }
        } else if self.scheme.is_some() {
            if self.path.starts_with('/') {
                return UriType::UriNoAuthority;
            } else {
                return UriType::UriCannotBeABase;
            }
        } else if self.path.starts_with('/') {
            return UriType::AbsolutePath;
        } else if self.path.is_empty() {
            if self.query.is_some() {
                return UriType::Query;
            } else if self.fragment.is_some() {
                return UriType::Fragment;
            }
        }

        return UriType::RelativePath;
    }
}

impl Display for UriRawComponents<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.write_to(f)
    }
}

impl From<UriRawComponents<'_>> for String {
    fn from(comp: UriRawComponents<'_>) -> Self {
        String::from(&comp)
    }
}

impl From<&UriRawComponents<'_>> for String {
    fn from(comp: &UriRawComponents<'_>) -> Self {
        comp.to_string()
    }
}

impl<'a> UriRawComponents<'a> {
    /// Constructs a new `UriRawComponents` from the given string slice, which is assumed
    /// to contain a URI-reference.
    pub fn from_str(uri: &'a str) -> Result<UriRawComponents<'a>, ParseError> {
        if let Some(i) = uri.unescape_uri().first_error() {
            return Err(ParseError::from(i));
        }

        let captures = match RFC3986_APPENDIX_B.captures(uri) {
            Some(x) => x,
            None => {
                return Err(ParseError::new(
                    "Cannot find URI components",
                    Some(0..uri.len()),
                ));
            }
        };

        let scheme = if let Some(x) = captures.get(2) {
            // Do an additional syntax check on the scheme to make sure it is valid.
            if URI_CHECK_SCHEME.captures(x.as_str()).is_some() {
                Some(x.as_str())
            } else {
                return Err(ParseError::new(
                    "Invalid URI scheme",
                    Some(x.start()..x.end()),
                ));
            }
        } else {
            None
        };

        let authority = captures.get(4).map(|x| x.as_str());
        let query = captures.get(7).map(|x| x.as_str());
        let fragment = captures.get(9).map(|x| x.as_str());

        // Unwrap safety: Capture 5 is not an optional capture in the regex.
        let path = captures.get(5).unwrap().as_str();

        unsafe {
            Ok(UriRawComponents::from_components_unchecked(
                scheme, authority, path, query, fragment,
            ))
        }
    }

    #[inline(always)]
    /// Returns the slice of the URI that describes the URI scheme, if present.
    /// Percent encoding is not allowed in the scheme, so no decoding is required.
    ///
    pub fn scheme(&self) -> Option<&'a str> {
        self.scheme
    }

    /// Returns the escaped slice of the URI that contains the "authority", if present.
    ///
    /// See [`UriRawComponents::authority`] for the percent-decoded version.
    #[inline(always)]
    pub fn raw_authority(&self) -> Option<&'a str> {
        self.authority
    }

    /// Returns the escaped slice of the URI that contains the "userinfo", if present.
    ///
    /// See [`UriRawComponents::userinfo`] for the percent-decoded version.
    #[inline(always)]
    pub fn raw_userinfo(&self) -> Option<&'a str> {
        self.userinfo
    }

    /// Returns the escaped slice of the URI that contains the "host", if present.
    ///
    /// See [`UriRawComponents::host`] for the percent-decoded version.
    #[inline(always)]
    pub fn raw_host(&self) -> Option<&'a str> {
        self.host
    }

    /// Returns the 16-bit representation of the port number, if present in the authority.
    #[inline(always)]
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Returns the escaped slice of the URI that contains the "path".
    ///
    /// See [`UriRawComponents::path`] for the percent-decoded version.
    #[inline(always)]
    pub fn raw_path(&self) -> &'a str {
        self.path
    }

    /// Returns the subset of this URI that is a path, without the
    /// scheme, authority, query, or fragment. Since this is itself
    /// a valid relative URI, it returns a `&RelRef`.
    pub fn path_as_rel_ref(&self) -> &'a RelRef {
        unsafe { RelRef::from_str_unchecked(self.raw_path()) }
    }

    /// Returns the escaped substring of the URI that contains the "query", if present.
    ///
    /// See [`StrExt`] for details on unescaping the results.
    #[inline(always)]
    pub fn raw_query(&self) -> Option<&'a str> {
        self.query
    }

    /// Returns the escaped substring of the URI that contains the "fragment", if present.
    ///
    /// See [`UriRawComponents::fragment`] for the percent-decoded version.
    #[inline(always)]
    pub fn raw_fragment(&self) -> Option<&'a str> {
        self.fragment
    }

    /// An iterator which returns each individual *escaped* path item.
    ///
    /// See [`UriRawComponents::path_segments`] for the percent-decoded version.
    pub fn raw_path_segments(&self) -> impl Iterator<Item = &'a str> {
        if self.path.is_empty() {
            let mut ret = "".split('/');
            let _ = ret.next();
            return ret;
        } else {
            self.path.split('/')
        }
    }

    /// An iterator which returns each individual *escaped* query item.
    ///
    /// See [`UriRawComponents::query_items`] for the percent-decoded version.
    pub fn raw_query_items(&self) -> impl Iterator<Item = &'a str> {
        let pattern = |c| c == '&' || c == ';';
        match self.query {
            Some(query) => query.split(pattern),
            None => {
                let mut ret = "".split(pattern);
                let _ = ret.next();
                return ret;
            }
        }
    }

    /// An iterator which returns each individual *escaped* query item as a
    /// key/value pair. Note that neither are unescaped.
    ///
    /// See [`UriRawComponents::query_key_values`] for the percent-decoded version.
    pub fn raw_query_key_values(&self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.raw_query_items().map(|comp| match comp.find('=') {
            Some(x) => comp.split_at(x),
            None => (comp, ""),
        })
    }

    /// Unescaped (percent-decoded) version of [`UriRawComponents::raw_fragment`], using
    /// `std::borrow::Cow<str>` instead of `&str`.
    pub fn fragment(&self) -> Option<Cow<'_, str>> {
        self.raw_fragment().map(|f| f.unescape_uri().to_cow())
    }

    /// Unescaped (percent-decoded) version of [`UriRawComponents::raw_host`], using
    /// `std::borrow::Cow<str>` instead of `&str`.
    pub fn host(&self) -> Option<Cow<'_, str>> {
        self.raw_host().map(|f| f.unescape_uri().to_cow())
    }

    /// Unescaped (percent-decoded) version of [`UriRawComponents::raw_authority`], using
    /// `std::borrow::Cow<str>` instead of `&str`.
    pub fn authority(&self) -> Option<Cow<'_, str>> {
        self.raw_authority().map(|f| f.unescape_uri().to_cow())
    }

    /// Unescaped (percent-decoded) version of [`UriRawComponents::raw_userinfo`], using
    /// `std::borrow::Cow<str>` instead of `&str`.
    pub fn userinfo(&self) -> Option<Cow<'_, str>> {
        self.raw_userinfo().map(|f| f.unescape_uri().to_cow())
    }

    /// Unescaped (percent-decoded) version of [`UriRawComponents::raw_query`], using
    /// `std::borrow::Cow<str>` instead of `&str`.
    pub fn query(&self) -> Option<Cow<'_, str>> {
        self.raw_query().map(|f| f.unescape_uri().to_cow())
    }

    /// Unescaped (percent-decoded) version of [`UriRawComponents::raw_path_segments`], using
    /// `std::borrow::Cow<str>` instead of `&str`.
    pub fn path_segments(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.raw_path_segments()
            .map(|item| item.unescape_uri().to_cow())
    }

    /// Unescaped (percent-decoded) version of [`UriRawComponents::raw_query_items`], using
    /// `std::borrow::Cow<str>` instead of `&str`.
    pub fn query_items(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.raw_query_items()
            .map(|item| item.unescape_uri().to_cow())
    }

    /// Unescaped (percent-decoded) version of [`UriRawComponents::raw_query_key_values`], using
    /// `std::borrow::Cow<str>` instead of `&str`.
    pub fn query_key_values(&self) -> impl Iterator<Item = (Cow<'_, str>, Cow<'_, str>)> {
        self.raw_query_key_values().map(|item| {
            (
                item.0.unescape_uri().to_cow(),
                item.1.unescape_uri().to_cow(),
            )
        })
    }

    /// Returns a `UriRawComponents` with any leading dot-slashes trimmed from the path.
    #[must_use]
    pub fn trim_leading_dot_slashes(&self) -> Self {
        UriRawComponents {
            path: self.path_as_rel_ref().trim_leading_dot_slashes(),
            ..self.clone()
        }
    }

    /// Returns a `UriRawComponents` with the query and fragment cleared.
    #[must_use]
    pub fn trim_query(&self) -> Self {
        UriRawComponents {
            query: None,
            fragment: None,
            ..self.clone()
        }
    }

    /// Returns a `UriRawComponents` with the query cleared.
    #[must_use]
    pub fn trim_fragment(&self) -> Self {
        UriRawComponents {
            fragment: None,
            ..self.clone()
        }
    }
}

impl<'a> UriRawComponents<'a> {
    /// Constructs a new `UriRawComponents` from the given raw, percent-encoded components,
    /// without checking that the components are valid.
    ///
    /// This method is unsafe because the components are not checked to ensure they are valid.
    pub unsafe fn from_components_unchecked(
        scheme: Option<&'a str>,
        authority: Option<&'a str>,
        path: &'a str,
        query: Option<&'a str>,
        fragment: Option<&'a str>,
    ) -> UriRawComponents<'a> {
        let userinfo;
        let host;
        let port;

        if let Some(authority) = authority {
            match URI_AUTHORITY.captures(authority) {
                Some(y) => {
                    userinfo = if let Some(x) = y.get(2) {
                        Some(x.as_str())
                    } else {
                        None
                    };
                    host = if let Some(x) = y.get(3) {
                        Some(x.as_str())
                    } else {
                        None
                    };
                    port = if let Some(x) = y.get(5) {
                        u16::from_str(x.as_str()).ok()
                    } else {
                        None
                    };
                }
                None => {
                    userinfo = None;
                    host = None;
                    port = None;
                }
            }
        } else {
            userinfo = None;
            host = None;
            port = None;
        };

        UriRawComponents {
            scheme,
            authority,
            userinfo,
            host,
            port,
            path,
            query,
            fragment,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components() {
        {
            let uri = uri_ref!("http://example.com/");
            let components = uri.components();
            assert!(!components.uri_type().cannot_be_a_base());
            assert_eq!(Some("http"), components.scheme());
            assert_eq!(Some("example.com"), components.raw_host());
            assert_eq!(None, components.port());
            assert_eq!("/", components.raw_path());
            assert_eq!(None, components.raw_userinfo());
            assert_eq!(None, components.raw_fragment());
            assert_eq!(None, components.raw_query());
        }

        {
            let uri = UriRefBuf::from_str("http://example.com/").unwrap();
            let uri_ref = uri.as_uri_ref();
            let components = uri_ref.components();
            assert!(!components.uri_type().cannot_be_a_base());
            assert_eq!(Some("http"), components.scheme());
            assert_eq!(Some("example.com"), components.raw_host());
            assert_eq!(None, components.port());
            assert_eq!("/", components.raw_path());
            assert_eq!(None, components.raw_userinfo());
            assert_eq!(None, components.raw_fragment());
            assert_eq!(None, components.raw_query());
        }

        {
            let uri = UriRefBuf::from_str("mailto:fred@example.com").unwrap();
            let uri_ref = uri.as_uri_ref();
            let components = uri_ref.components();
            assert!(components.uri_type().cannot_be_a_base());
            assert_eq!(Some("mailto"), components.scheme());
            assert_eq!(None, components.raw_host());
            assert_eq!(None, components.port());
            assert_eq!("fred@example.com", components.raw_path());
            assert_eq!(None, components.raw_userinfo());
            assert_eq!(None, components.raw_fragment());
            assert_eq!(None, components.raw_query());
        }

        let component_test_table = vec![
            (
                "http://goo.gl/a/b/c/d?query",
                vec!["", "a", "b", "c", "d"],
                vec!["query"],
                rel_ref!("/a/b/c/d?query"),
            ),
            (
                "http://goo.gl/a/b/c/d",
                vec!["", "a", "b", "c", "d"],
                vec![],
                rel_ref!("/a/b/c/d"),
            ),
            (
                "http://goo.gl/a/b/c/d/",
                vec!["", "a", "b", "c", "d", ""],
                vec![],
                rel_ref!("/a/b/c/d/"),
            ),
            (
                "/a/b/c/d/",
                vec!["", "a", "b", "c", "d", ""],
                vec![],
                rel_ref!("/a/b/c/d/"),
            ),
            (
                "a/b/c/d/",
                vec!["a", "b", "c", "d", ""],
                vec![],
                rel_ref!("a/b/c/d/"),
            ),
            (
                "a/b//c/d/",
                vec!["a", "b", "", "c", "d", ""],
                vec![],
                rel_ref!("a/b//c/d/"),
            ),
            (
                "a/b/c/d/?",
                vec!["a", "b", "c", "d", ""],
                vec![""],
                rel_ref!("a/b/c/d/?"),
            ),
            (
                "a?b=1;c=2;d=3",
                vec!["a"],
                vec!["b=1", "c=2", "d=3"],
                rel_ref!("a?b=1;c=2;d=3"),
            ),
            (
                "a?b=1&c=2&d=3",
                vec!["a"],
                vec!["b=1", "c=2", "d=3"],
                rel_ref!("a?b=1&c=2&d=3"),
            ),
            (
                "a/b/%47/d/",
                vec!["a", "b", "%47", "d", ""],
                vec![],
                rel_ref!("a/b/%47/d/"),
            ),
        ];

        for (a, b, c, d) in component_test_table {
            let uri = UriRef::from_str(a).unwrap();
            let components = uri.components();
            let path_components: Vec<&str> = components.raw_path_segments().collect();
            let query_components: Vec<&str> = components.raw_query_items().collect();
            let uri_rel: &RelRef = uri.path_query_as_rel_ref();
            assert_eq!(b, path_components);
            assert_eq!(c, query_components);
            assert_eq!(
                d, uri_rel,
                "Expected <{}>, Found <{}> (Item: <{}>)",
                d, uri_rel, a
            );
        }
    }
}
