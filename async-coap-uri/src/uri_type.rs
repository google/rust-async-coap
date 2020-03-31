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

/// Enum describing the type of a URI.
#[derive(Debug, Eq, Clone, Copy, PartialEq, Hash)]
pub enum UriType {
    /// An [IETF-RFC3986] "URI", including both scheme and authority components;
    /// E.g. `http://example.com/abcd`
    ///
    /// ```
    /// # use async_coap_uri::{*,UriType};
    /// assert_eq!(UriType::Uri, uri_ref!("http://example.com/abcd").uri_type());
    /// ```
    ///
    /// URI references with this type can borrowed as a [`&Uri`](struct.Uri.html).
    ///
    /// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
    Uri,

    /// An [IETF-RFC3986] "URI" that has a scheme but no authority component, where
    /// the path begins with a slash;
    /// E.g. `unix:/run/foo.socket`
    ///
    /// ```
    /// # use async_coap_uri::{*,UriType};
    /// assert_eq!(UriType::UriNoAuthority, uri_ref!("unix:/run/foo.socket").uri_type());
    /// ```
    ///
    /// URI references with this type can borrowed as a [`&Uri`](struct.Uri.html).
    ///
    /// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
    UriNoAuthority,

    /// An [IETF-RFC3986] "URI" that has a scheme but no authority component, where
    /// the path does not start with a slash (`/`);
    /// E.g. `tel:+1-555-867-5309`
    ///
    /// ```
    /// # use async_coap_uri::{*,UriType};
    /// assert_eq!(UriType::UriCannotBeABase, uri_ref!("tel:+1-555-867-5309").uri_type());
    /// ```
    ///
    /// URI references with this type can borrowed as a [`&Uri`](struct.Uri.html).
    ///
    /// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
    UriCannotBeABase,

    /// An [IETF-RFC3986] "network-path" that has an authority component but no scheme;
    /// E.g. `//example.com/foo/bar?q`
    ///
    /// ```
    /// # use async_coap_uri::{*,UriType};
    /// assert_eq!(UriType::NetworkPath, uri_ref!("//example.com/foo/bar").uri_type());
    /// ```
    ///
    /// Note that according to [IETF-RFC3986] this is a "relative-reference", but because it
    /// includes an authority this library considers it to be borrowable as a [`&Uri`](struct.Uri.html).
    ///
    /// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
    NetworkPath,

    /// An [IETF-RFC3986] "relative-reference" with a "path-absolute" and optionally a query and/or fragment;
    /// E.g. `/foo/bar?q`
    ///
    /// ```
    /// # use async_coap_uri::{*,UriType};
    /// assert_eq!(UriType::AbsolutePath, uri_ref!("/foo/bar?q").uri_type());
    /// ```
    ///
    /// URI references with this type can borrowed as a [`&RelRef`](struct.RelRef.html).
    ///
    /// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
    AbsolutePath,

    /// An [IETF-RFC3986] "relative-reference" with a "path-rootless" and optionally a query and/or fragment;
    /// E.g. `foo/bar?q`
    ///
    /// An empty URI reference has this type, but the path is guaranteed to have a non-zero length
    /// if a query or fragment is present.
    ///
    /// ```
    /// # use async_coap_uri::{*,UriType};
    /// assert_eq!(UriType::RelativePath, uri_ref!("foo/bar?q").uri_type());
    /// assert_eq!(UriType::RelativePath, uri_ref!("").uri_type());
    /// ```
    ///
    /// URI references with this type can borrowed as a [`&RelRef`](struct.RelRef.html).
    ///
    /// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
    RelativePath,

    /// An [IETF-RFC3986] "relative-reference" with a "path-empty", a query, and optionally a fragment;
    /// E.g. `?q=blah`, `?q=blah#foo-bar`
    ///
    /// ```
    /// # use async_coap_uri::{*,UriType};
    /// assert_eq!(UriType::Query, uri_ref!("?q=blah").uri_type());
    /// assert_eq!(UriType::Query, uri_ref!("?q=blah#foo-bar").uri_type());
    /// ```
    ///
    /// URI references with this type can borrowed as a [`&RelRef`](struct.RelRef.html).
    ///
    /// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
    Query,

    /// An [IETF-RFC3986] "relative-reference" that includes only a fragment component;
    /// E.g. `#foo-bar`
    ///
    /// ```
    /// # use async_coap_uri::{*,UriType};
    /// assert_eq!(UriType::Fragment, uri_ref!("#foo-bar").uri_type());
    /// ```
    ///
    /// URI references with this type can borrowed as a [`&RelRef`](struct.RelRef.html).
    ///
    /// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
    Fragment,
}

impl UriType {
    /// Returns true if this type can be borrowed as a [`&Uri`], false otherwise.
    pub fn can_borrow_as_uri(&self) -> bool {
        match self {
            UriType::Uri
            | UriType::UriNoAuthority
            | UriType::NetworkPath
            | UriType::UriCannotBeABase => true,
            _ => false,
        }
    }

    /// Returns true if this type can be borrowed as a [`&RelRef`], false otherwise.
    pub fn can_borrow_as_rel_ref(&self) -> bool {
        !self.can_borrow_as_uri()
    }

    /// Returns true if this type can be assumed to have an absolute path.
    pub fn has_absolute_path(&self) -> bool {
        match self {
            UriType::Uri
            | UriType::UriNoAuthority
            | UriType::AbsolutePath
            | UriType::NetworkPath => true,
            _ => false,
        }
    }

    /// Returns true if [IETF-RFC3986] considers this type to be a "relative-reference".
    ///
    /// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
    pub fn is_ietf_rfc3986_relative_reference(&self) -> bool {
        match self {
            UriType::Uri | UriType::UriNoAuthority | UriType::UriCannotBeABase => false,
            _ => true,
        }
    }

    /// Returns true if this type cannot be used as a base when performing URI reference resolution.
    pub fn cannot_be_a_base(&self) -> bool {
        match self {
            UriType::UriCannotBeABase | UriType::Fragment | UriType::Query => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{UriType, *};

    #[test]
    fn uri_type_uri() {
        assert_eq!(uri_ref!("http://example.com/abcd").uri_type(), UriType::Uri);
        assert_eq!(iuri!("http://example.com/abcd").uri_type(), UriType::Uri);
        assert_eq!(
            uri_ref!("http://example.com/abcd").components().uri_type(),
            UriType::Uri
        );
    }

    #[test]
    fn uri_type_uri_cannot_be_a_base() {
        assert_eq!(
            uri_ref!("tel:+1-555-867-5309").uri_type(),
            UriType::UriCannotBeABase
        );
        assert_eq!(
            iuri!("tel:+1-555-867-5309").uri_type(),
            UriType::UriCannotBeABase
        );
        assert_eq!(
            uri_ref!("tel:+1-555-867-5309").components().uri_type(),
            UriType::UriCannotBeABase
        );
    }

    #[test]
    fn uri_type_uri_no_authority() {
        assert_eq!(
            uri_ref!("unix:/run/foo.socket").uri_type(),
            UriType::UriNoAuthority
        );
        assert_eq!(
            iuri!("unix:/run/foo.socket").uri_type(),
            UriType::UriNoAuthority
        );
        assert_eq!(
            uri_ref!("unix:/run/foo.socket").components().uri_type(),
            UriType::UriNoAuthority
        );
    }

    #[test]
    fn uri_type_network_path() {
        assert_eq!(
            uri_ref!("//example.com/foo/bar").uri_type(),
            UriType::NetworkPath
        );
        assert_eq!(
            iuri!("//example.com/foo/bar").uri_type(),
            UriType::NetworkPath
        );
        assert_eq!(
            uri_ref!("//example.com/foo/bar").components().uri_type(),
            UriType::NetworkPath,
            "{:?}",
            uri_ref!("//example.com/foo/bar").components()
        );
    }

    #[test]
    fn uri_type_absolute_path() {
        assert_eq!(uri_ref!("/foo/bar?q").uri_type(), UriType::AbsolutePath);
        assert_eq!(rel_ref!("/foo/bar?q").uri_type(), UriType::AbsolutePath);
        assert_eq!(
            uri_ref!("/foo/bar?q").components().uri_type(),
            UriType::AbsolutePath
        );
    }

    #[test]
    fn uri_type_relative_path() {
        assert_eq!(uri_ref!("foo/bar?q").uri_type(), UriType::RelativePath);
        assert_eq!(rel_ref!("foo/bar?q").uri_type(), UriType::RelativePath);
        assert_eq!(
            uri_ref!("foo/bar?q").components().uri_type(),
            UriType::RelativePath
        );
    }

    #[test]
    fn uri_type_query() {
        assert_eq!(uri_ref!("?q#frag").uri_type(), UriType::Query);
        assert_eq!(rel_ref!("?q#frag").uri_type(), UriType::Query);
        assert_eq!(uri_ref!("?q#frag").components().uri_type(), UriType::Query);
    }

    #[test]
    fn uri_type_fragment() {
        assert_eq!(uri_ref!("#foo-bar").uri_type(), UriType::Fragment);
        assert_eq!(rel_ref!("#foo-bar").uri_type(), UriType::Fragment);
        assert_eq!(
            uri_ref!("#foo-bar").components().uri_type(),
            UriType::Fragment
        );
    }
}
