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
use core::ops::{Deref, Range};
use core::str::FromStr;

#[cfg(feature = "std")]
use std::borrow::Cow;

/// Unsized string-slice type guaranteed to contain a well-formed [IETF-RFC3986] [URI-reference].
///
/// From [IETF-RFC3986 Section 4.1][URI-reference]:
///
/// > \[A\] URI-reference is used to denote the most common usage of a resource
/// > identifier.
/// >
/// > ```abnf
/// > URI-reference = URI / relative-ref
/// > ```
/// >
/// > A URI-reference is either a URI or a relative reference.  If the
/// > URI-reference's prefix does not match the syntax of a scheme followed
/// > by its colon separator, then the URI-reference is a relative
/// > reference.
///
/// [`UriRef`] is similar to [`str`] in that it is an unsized type and is generally only seen
/// in its borrowed form: `&UriRef`. The big difference between `str` and `UriRef` is that `UriRef`
/// guarantees that it contains a well-formed URI-reference.
///
/// The sized counterpart is [`UriRefBuf`], but the underlying data can be owned by just about
/// anything.
///
/// ## Examples
///
/// String literals can be made into `URI-reference` using the [`uri_ref!`] macro:
///
/// [`uri_ref!`]: macro.uri_ref.html
///
/// ```
/// use async_coap_uri::prelude::*;
///
/// let uri = uri_ref!("http://example.com/test");
/// ```
///
/// Depending on your needs, you can access the raw (escaped) components individually, or
/// calculate them all at once using [`UriRawComponents`]:
///
/// ```
/// # use async_coap_uri::*;
/// # let uri = uri_ref!("http://example.com/test");
/// #
/// // Accessed and calculated individually...
/// assert_eq!(Some("http"), uri.scheme());
/// assert_eq!(Some("example.com"), uri.raw_authority());
/// assert_eq!("/test", uri.raw_path());
///
/// // ...or calculate all of them at once.
/// let components = uri.components();
/// assert_eq!(Some("http"), components.scheme());
/// assert_eq!(Some("example.com"), components.raw_authority());
/// assert_eq!("/test", components.raw_path());
/// ```
///
/// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
/// [URI-reference]: https://tools.ietf.org/html/rfc3986#section-4.1
#[derive(Eq, Hash)]
pub struct UriRef(pub(super) str);

_impl_uri_traits_base!(UriRef);

impl Deref for UriRef {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

/// The default `&UriRef` is an empty URI-reference.
impl Default for &UriRef {
    fn default() -> Self {
        uri_ref!("")
    }
}

/// The default `&mut UriRef` is an empty URI-reference.
///
/// Note that even though it is technically mutable, because the slice
/// is empty it is mutable in name only.
impl Default for &mut UriRef {
    fn default() -> Self {
        use std::slice::from_raw_parts_mut;
        use std::str::from_utf8_unchecked_mut;
        unsafe {
            // SAFETY: An empty slice is pretty harmless, mutable or not.
            let empty_slice = from_raw_parts_mut(0usize as *mut u8, 0);
            let empty_string = from_utf8_unchecked_mut(empty_slice);
            UriRef::from_str_unchecked_mut(empty_string)
        }
    }
}

impl AnyUriRef for UriRef {
    fn write_to<T: core::fmt::Write + ?Sized>(
        &self,
        write: &mut T,
    ) -> Result<(), core::fmt::Error> {
        write.write_str(self.as_str())
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn uri_type(&self) -> UriType {
        if self.starts_with('#') {
            return UriType::Fragment;
        }

        if self.starts_with('?') {
            return UriType::Query;
        }

        if self.starts_with("//") {
            return UriType::NetworkPath;
        }

        if self.starts_with('/') {
            return UriType::AbsolutePath;
        }

        let pat = |c| c == ':' || c == '/' || c == '?' || c == '#';
        if let Some(i) = self.find(pat) {
            if self[i..].starts_with("://") {
                return UriType::Uri;
            } else if self[i..].starts_with(":/") {
                return UriType::UriNoAuthority;
            } else if self[i..].starts_with(':') {
                return UriType::UriCannotBeABase;
            }
        }

        return UriType::RelativePath;
    }

    fn components(&self) -> UriRawComponents<'_> {
        UriRawComponents::from_str(self.as_str()).unwrap()
    }
}

impl std::fmt::Display for UriRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.write_to(f)
    }
}

impl UriRef {
    /// Attempts to convert a string slice into a [`&UriRef`](UriRef), returning `Err(ParseError)`
    /// if the string slice contains data that is not a valid URI reference.
    ///
    /// Example:
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(UriRef::from_str("http://example.com"), Ok(uri_ref!("http://example.com")));
    /// assert_eq!(UriRef::from_str("/a/b/c"), Ok(uri_ref!("/a/b/c")));
    /// assert_eq!(UriRef::from_str("I N V A L I D").err().unwrap().span(), Some(1..2));
    /// ```
    pub fn from_str(s: &str) -> Result<&UriRef, ParseError> {
        UriRawComponents::from_str(s)?;
        Ok(unsafe { Self::from_str_unchecked(s.as_ref()) })
    }

    /// Determines if the given string can be considered a valid URI reference.
    ///
    /// The key difference between this and [`Uri::is_str_valid`] is that this
    /// function will return true for relative-references like `/a/b/c`, whereas
    /// [`Uri::is_str_valid`] would return false.
    ///
    /// Example:
    ///
    /// ```
    /// use async_coap_uri::UriRef;
    /// assert!(UriRef::is_str_valid("http://example.com"));
    /// assert!(UriRef::is_str_valid("/a/b/c"));
    /// assert!(!UriRef::is_str_valid("Not a URI or relative reference"));
    /// ```
    pub fn is_str_valid<S: AsRef<str>>(s: S) -> bool {
        let str_ref = s.as_ref();

        // TODO: Replace this with an optimized validity check.
        //       We are currently using `UriRawComponents::from_str()` as a crutch here;
        //       it includes extraneous operations that are not related to verifying if a
        //       URI is well-formed.
        UriRawComponents::from_str(str_ref).is_ok()
    }

    /// Returns this URI-reference as a string slice.
    #[inline(always)]
    pub const fn as_str(&self) -> &str {
        &self.0
    }

    /// Attempts to interpret this [`&UriRef`][UriRef] as a [`&Uri`][Uri], returning `None`
    /// if this `UriRef` doesn't contain a proper URI.
    pub fn as_uri(&self) -> Option<&Uri> {
        if self.uri_type().can_borrow_as_uri() {
            Some(unsafe { Uri::from_str_unchecked(self.as_str()) })
        } else {
            None
        }
    }

    /// Attempts to interpret this [`&UriRef`][UriRef] as a [`&RelRef`][RelRef], returning `None`
    /// if this `UriRef` doesn't contain a relative-reference.
    pub fn as_rel_ref(&self) -> Option<&RelRef> {
        if self.uri_type().can_borrow_as_rel_ref() {
            Some(unsafe { RelRef::from_str_unchecked(self.as_str()) })
        } else {
            None
        }
    }
}

/// ## Indexing Methods
impl UriRef {
    /// Returns the index to the start of `heir-part`, as defined in IETF-RFC3986.
    pub fn heir_part_start(&self) -> usize {
        let pat = |c| c == ':' || c == '/' || c == '?' || c == '#';
        if let Some(i) = self.find(pat) {
            if self[i..].starts_with(':') {
                return i + 1;
            }
        }
        0
    }

    /// Returns the index to the first character in the path. Will
    /// return `len()` if there is no path, query, or fragment.
    /// If the return value is zero, then `self` is guaranteed to be
    /// a relative reference.
    ///
    /// ## Examples
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(uri_ref!("a").path_start(),               0);
    /// assert_eq!(uri_ref!("a/b/c?blah#frag").path_start(), 0);
    /// assert_eq!(uri_ref!("/a").path_start(),              0);
    /// assert_eq!(uri_ref!("//foo/?bar").path_start(),      5);
    /// assert_eq!(uri_ref!("http://a/#frag").path_start(),  8);
    /// assert_eq!(uri_ref!("http://a").path_start(),  8);
    /// ```
    pub fn path_start(&self) -> usize {
        let heir_part_start = self.heir_part_start();
        let heir_part = &self[heir_part_start..];

        if heir_part.starts_with("//") {
            let authority = &heir_part[2..];

            // Find the end of the authority.
            let pat = |c| c == '/' || c == '?' || c == '#';
            if let Some(j) = authority.find(pat) {
                heir_part_start + 2 + j
            } else {
                self.len()
            }
        } else {
            heir_part_start
        }
    }

    /// Returns the index of the end of the path
    pub fn path_end(&self) -> usize {
        let pat = |c| c == '?' || c == '#';
        if let Some(i) = self.find(pat) {
            i
        } else {
            self.len()
        }
    }

    /// Returns the index of the start of the query, including the `?`.
    /// If there is no query, returns `None`.
    pub fn query_start(&self) -> Option<usize> {
        let pat = |c| c == '?' || c == '#';
        if let Some(i) = self.find(pat) {
            if self[i..].starts_with('?') {
                return Some(i);
            }
        }
        None
    }

    /// Returns the index of the start of the fragment, including the `#`.
    /// If there is no fragment, returns `None`.
    pub fn fragment_start(&self) -> Option<usize> {
        self.find('#')
    }

    /// Returns the byte index range that contains the authority, if present.
    pub fn authority_range(&self) -> Option<Range<usize>> {
        let pat = |c| c == '/' || c == '?' || c == '#';
        if let Some(i) = self.find(pat) {
            let step1 = &self[i..];
            if !step1.starts_with("//") {
                return None;
            }
            let step2 = &step1[2..];
            if let Some(j) = step2.find(pat) {
                return Some(i + 2..i + 2 + j);
            } else {
                return Some(i + 2..self.len());
            }
        }
        None
    }
}

/// ## Splitting
impl UriRef {
    /// Splits this URI into the base and relative portions.
    pub fn split(&self) -> (Option<&Uri>, &RelRef) {
        let path_start = self.path_start();
        if path_start == 0 {
            // This is a relative URI, so there is no base part.
            (None, unsafe { RelRef::from_str_unchecked(self.as_str()) })
        } else {
            let (base, rel) = self.split_at(path_start);
            let base = unsafe { Uri::from_str_unchecked(base) };
            let rel = unsafe { RelRef::from_str_unchecked(rel) };

            (Some(base), rel)
        }
    }

    /// Splits this URI into the base and relative portions.
    pub fn split_mut(&mut self) -> (Option<&mut Uri>, &mut RelRef) {
        let path_start = self.path_start();
        if path_start == 0 {
            // This is a relative URI, so there is no base part.
            (None, unsafe {
                // SAFETY: If path_start() returns zero, this is guaranteed to be a
                //         URI-reference.
                RelRef::from_str_unchecked_mut(self.as_mut_str())
            })
        } else {
            unsafe {
                let (base, rel) = self.as_mut_str().split_at_mut(path_start);
                (
                    Some(Uri::from_str_unchecked_mut(base)),
                    RelRef::from_str_unchecked_mut(rel),
                )
            }
        }
    }

    /// Returns the subset of this URI that contains the scheme and authority, without the
    /// path, query, or fragment. If this is possible, the result is a `&Uri`.
    pub fn base(&self) -> Option<&Uri> {
        self.split().0
    }

    /// Returns this URI as a `&RelRef`, including only the path, query, and fragment.
    /// If this URI is already relative, this method simply returns the given URI
    /// unchanged.
    pub fn rel(&self) -> &RelRef {
        self.split().1
    }

    /// Returns this URI as a `&mut RelRef`, including only the path, query, and fragment.
    /// If this URI is already relative, this method simply returns the given URI
    /// unchanged.
    pub fn rel_mut(&mut self) -> &mut RelRef {
        self.split_mut().1
    }
}

/// ## Component Accessors
impl UriRef {
    /// Returns the scheme of this URI, if it has a scheme.
    /// The returned string can be used directly and does not need to be unescaped
    /// or percent-decoded.
    pub fn scheme(&self) -> Option<&str> {
        let pat = |c| c == ':' || c == '/' || c == '?' || c == '#';
        if let Some(i) = self.find(pat) {
            if self[i..].starts_with(':') {
                return Some(&self[..i]);
            }
        }
        None
    }

    /// Returns the percent-encoded authority part of this URI, if it has one.
    ///
    /// The unescaped version of this method is [`UriRef::authority`].
    ///
    /// In general, this value should not be used directly. Most users will find the
    /// method [`raw_userinfo_host_port`](#method.raw_userinfo_host_port) to be a more
    /// useful extraction.
    ///
    /// See [`StrExt`] for more details on unescaping.
    pub fn raw_authority(&self) -> Option<&str> {
        Some(&self[self.authority_range()?])
    }

    /// Percent-decoded version of [`UriRef::raw_authority`], using `std::borrow::Cow<str>` instead of `&str`.
    #[cfg(feature = "std")]
    pub fn authority(&self) -> Option<Cow<'_, str>> {
        self.raw_authority().map(|f| f.unescape_uri().to_cow())
    }

    /// Returns a tuple containing the raw userinfo, raw host, and port number from the
    /// authority component.
    ///
    /// The percent-decoded version of this method is [`UriRef::userinfo_host_port`].
    ///
    /// The userinfo and host should be unescaped before being used. See [`StrExt`]
    /// for more details.
    pub fn raw_userinfo_host_port(&self) -> Option<(Option<&str>, &str, Option<u16>)> {
        let authority = self.raw_authority()?;
        let userinfo;
        let host_and_port;

        if let Some(i) = authority.find('@') {
            userinfo = Some(&authority[..i]);
            host_and_port = &authority[i + 1..];
        } else {
            userinfo = None;
            host_and_port = authority;
        }

        let host;
        let port;

        if host_and_port.starts_with('[') {
            if let Some(i) = host_and_port.rfind("]:") {
                host = &host_and_port[1..i];
                port = u16::from_str(&host_and_port[i + 2..]).ok();
            } else {
                host = &host_and_port[1..host_and_port.len() - 1];
                port = None;
            }
        } else {
            if let Some(i) = host_and_port.rfind(':') {
                host = &host_and_port[..i];
                port = u16::from_str(&host_and_port[i + 1..]).ok();
            } else {
                host = host_and_port;
                port = None;
            }
        }

        Some((userinfo, host, port))
    }

    /// Percent-decoded version of [`UriRef::raw_userinfo_host_port`], where the unescaped parts are
    /// represented as `std::borrow::Cow<str>` instances.
    #[cfg(feature = "std")]
    pub fn userinfo_host_port(&self) -> Option<(Option<Cow<'_, str>>, Cow<'_, str>, Option<u16>)> {
        self.raw_userinfo_host_port().map(|item| {
            (
                item.0.map(|s| s.unescape_uri().to_cow()),
                item.1.unescape_uri().to_cow(),
                item.2,
            )
        })
    }

    /// Percent-decoded *host* as a `std::borrow::Cow<str>`, if present.
    #[cfg(feature = "std")]
    pub fn host(&self) -> Option<Cow<'_, str>> {
        self.raw_userinfo_host_port()
            .map(|item| item.1.unescape_uri().to_cow())
    }

    /// Returns a string slice containing the raw, percent-encoded value of the path.
    ///
    /// There is no unescaped version of this method because the resulting ambiguity
    /// of percent-encoded slashes (`/`) present a security risk. Use [`path_segments`] if
    /// you need an escaped version.
    ///
    /// If you absolutely must, you can use the following
    /// code to obtain a lossy, percent-decoded version of the path that doesn't decode
    /// `%2F` into slash characters:
    ///
    /// ```
    /// # use async_coap_uri::prelude::*;
    /// let path = uri_ref!("%2F../a/%23/")
    ///     .raw_path()
    ///     .unescape_uri()
    ///     .skip_slashes()
    ///     .to_string();
    ///
    /// assert_eq!(path, "%2F../a/#/");
    /// ```
    ///
    /// [`path_segments`]: UriRef::path_segments
    #[must_use]
    #[inline(always)]
    pub fn raw_path(&self) -> &str {
        self.path_as_rel_ref().as_str()
    }

    /// Returns the subset of this URI that is a path, without the
    /// scheme, authority, query, or fragment. Since this is itself
    /// a valid relative URI, it returns a `&RelRef`.
    pub fn path_as_rel_ref(&self) -> &RelRef {
        self.rel().path_as_rel_ref()
    }

    /// Returns the subset of this URI that is a path and query, without the
    /// scheme, authority, or fragment. Since this is itself
    /// a valid relative URI, it returns a `&RelRef`.
    pub fn path_query_as_rel_ref(&self) -> &RelRef {
        self.trim_fragment().rel()
    }

    /// An iterator which returns each individual raw, percent-encoded *path segment*.
    ///
    /// The percent-decoded (unescaped) version of this method is [`UriRef::path_segments`].
    ///
    /// The values returned by this iterator should be unescaped before being used.
    /// See [`StrExt`] and [`StrExt::unescape_uri`] for more details.
    ///
    /// ## Example
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// let rel_ref = uri_ref!("g:a/%2F/bl%c3%a5b%c3%a6r");
    /// let mut iter = rel_ref.raw_path_segments();
    ///
    /// assert_eq!(iter.next(), Some("a"));
    /// assert_eq!(iter.next(), Some("%2F"));
    /// assert_eq!(iter.next(), Some("bl%c3%a5b%c3%a6r"));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn raw_path_segments(&self) -> impl Iterator<Item = &str> {
        self.rel().raw_path_segments()
    }

    /// Percent-decoded (unescaped) version of [`UriRef::raw_path_segments`], using
    /// `std::borrow::Cow<str>` instead of `&str`.
    ///
    /// ## Example
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// use std::borrow::Cow;
    /// let uri_ref = uri_ref!("g:a/%2F/bl%c3%a5b%c3%a6r");
    /// let mut iter = uri_ref.path_segments();
    ///
    /// assert_eq!(iter.next(), Some(Cow::from("a")));
    /// assert_eq!(iter.next(), Some(Cow::from("/")));
    /// assert_eq!(iter.next(), Some(Cow::from("blåbær")));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[cfg(feature = "std")]
    pub fn path_segments(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.raw_path_segments()
            .map(|item| item.unescape_uri().to_cow())
    }

    /// Returns the subset of this URI that is the query and fragment, without the
    /// scheme, authority, or path. This method includes the
    /// `?` prefix, making it a valid relative URI.
    pub fn query_fragment_as_rel_ref(&self) -> Option<&RelRef> {
        if let Some(i) = self.query_start() {
            Some(unsafe { RelRef::from_str_unchecked(&self[i..]) })
        } else {
            None
        }
    }

    /// Returns the subset of this URI that is the query, without the
    /// scheme, authority, path, or fragment. This method includes the
    /// `?` prefix, making it a valid relative URI.
    pub fn query_as_rel_ref(&self) -> Option<&RelRef> {
        self.trim_fragment().query_fragment_as_rel_ref()
    }

    /// Returns the escaped slice of the URI that contains the "query", if present.
    ///
    /// There is no unescaped version of this method because the resulting ambiguity
    /// of percent-encoded ampersands (`&`) and semicolons (`;`) present a security risk.
    /// Use [`query_items`] or [`query_key_values`] if you need a percent-decoded version.
    ///
    /// [`query_items`]: UriRef::query_items
    /// [`query_key_values`]: UriRef::query_key_values
    pub fn raw_query(&self) -> Option<&str> {
        self.query_as_rel_ref().map(|s| &s[1..])
    }

    /// Returns an iterator that iterates over all of the query items.
    ///
    /// Both `;` and `&` are acceptable query item delimiters.
    ///
    /// The percent-decoded version of this method is [`UriRef::query_items`].
    ///
    /// The values returned by this iterator should be unescaped before being used.
    /// See [`StrExt`] and [`StrExt::unescape_uri`] for more details.
    ///
    /// ## Example
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// let uri_ref = uri_ref!("/a/b/c?q=this:is&q=fun&q=bl%c3%a5b%c3%a6rsyltet%c3%b8y");
    /// let mut iter = uri_ref.raw_query_items();
    ///
    /// assert_eq!(iter.next(), Some("q=this:is"));
    /// assert_eq!(iter.next(), Some("q=fun"));
    /// assert_eq!(iter.next(), Some("q=bl%c3%a5b%c3%a6rsyltet%c3%b8y"));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn raw_query_items(&self) -> impl Iterator<Item = &str> {
        let pattern = |c| c == '&' || c == ';';
        match self.raw_query() {
            Some(query) => query.split(pattern),
            None => {
                let mut ret = "".split(pattern);
                let _ = ret.next();
                return ret;
            }
        }
    }

    /// Similar to [`raw_query_items()`], but additionally separates the key
    /// from the value for each query item.
    ///
    /// The percent-decoded version of this method is [`UriRef::query_key_values`].
    ///
    /// Both keys and values are in their raw, escaped form. If you want escaped
    /// values, consider [`query_key_values()`].
    ///
    /// [`raw_query_items()`]: #method.raw_query_items
    /// [`query_key_values()`]: #method.query_key_values
    ///
    /// ## Example
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// let uri_ref = uri_ref!("/a/b/c?inc&a=ok&b=q=q&c=bl%c3%a5b%c3%a6r");
    /// let mut iter = uri_ref.raw_query_key_values();
    ///
    /// assert_eq!(iter.next(), Some(("inc", "")));
    /// assert_eq!(iter.next(), Some(("a", "ok")));
    /// assert_eq!(iter.next(), Some(("b", "q=q")));
    /// assert_eq!(iter.next(), Some(("c", "bl%c3%a5b%c3%a6r")));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn raw_query_key_values(&self) -> impl Iterator<Item = (&str, &str)> {
        self.raw_query_items().map(|comp| match comp.find('=') {
            Some(x) => {
                let split = comp.split_at(x);
                (split.0, &split.1[1..])
            }
            None => (comp, ""),
        })
    }

    /// Percent-decoded version of [`UriRef::raw_query_items`], using `std::borrow::Cow<str>` instead of `&str`.
    #[cfg(feature = "std")]
    pub fn query_items(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.raw_query_items()
            .map(|item| item.unescape_uri().to_cow())
    }

    /// Similar to [`query_items()`], but additionally separates the key
    /// from the value for each query item.
    ///
    /// Both keys and values are percent-decoded and ready-to-use. If you want them in their raw,
    /// percent-encoded in form, use [`raw_query_key_values()`].
    ///
    /// This method uses the Copy-on-write type ([`std::borrow::Cow`]) to avoid unnecessary memory allocations.
    ///
    /// [`query_items()`]: #method.query_items
    /// [`raw_query_key_values()`]: #method.raw_query_key_values
    ///
    /// ## Example
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// use std::borrow::Cow;
    /// let uri_ref = uri_ref!("/a/b/c?inc&a=ok&b=q=q&c=bl%c3%a5b%c3%a6r");
    /// let mut iter = uri_ref.query_key_values();
    ///
    /// assert_eq!(iter.next(), Some((Cow::from("inc"), Cow::from(""))));
    /// assert_eq!(iter.next(), Some((Cow::from("a"), Cow::from("ok"))));
    /// assert_eq!(iter.next(), Some((Cow::from("b"), Cow::from("q=q"))));
    /// assert_eq!(iter.next(), Some((Cow::from("c"), Cow::from("blåbær"))));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[cfg(feature = "std")]
    pub fn query_key_values(&self) -> impl Iterator<Item = (Cow<'_, str>, Cow<'_, str>)> {
        self.raw_query_key_values().map(|item| {
            (
                item.0.unescape_uri().to_cow(),
                item.1.unescape_uri().to_cow(),
            )
        })
    }

    /// Returns the subset of this URI that is the query, without the
    /// scheme, authority, path, or query. This method includes the
    /// `#` prefix, making it a valid relative URI.
    pub fn fragment_as_rel_ref(&self) -> Option<&RelRef> {
        if let Some(i) = self.fragment_start() {
            Some(unsafe { RelRef::from_str_unchecked(&self[i..]) })
        } else {
            None
        }
    }

    /// Returns a string slice containing the fragment, if any.
    ///
    /// The percent-decoded version of this method is [`UriRef::fragment`].
    ///
    /// This value should be unescaped before being used. See [`StrExt`] for more details.
    pub fn raw_fragment(&self) -> Option<&str> {
        self.fragment_as_rel_ref().map(|s| &s[1..])
    }

    /// Percent-decoded version of [`UriRef::raw_fragment`], using `std::borrow::Cow<str>` instead of `&str`.
    #[cfg(feature = "std")]
    pub fn fragment(&self) -> Option<Cow<'_, str>> {
        self.raw_fragment().map(|f| f.unescape_uri().to_cow())
    }
}

impl UriRef {
    /// Returns true if the path has a trailing slash.
    ///
    /// ## Examples
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert!(uri_ref!("http://a/").has_trailing_slash());
    /// assert!(uri_ref!("/a/").has_trailing_slash());
    /// assert!(uri_ref!("a/").has_trailing_slash());
    /// assert!(uri_ref!("http://a/?q=foo").has_trailing_slash());
    ///
    /// assert!(!uri_ref!("http://a").has_trailing_slash());
    /// assert!(!uri_ref!("a/b").has_trailing_slash());
    /// assert!(!uri_ref!("").has_trailing_slash());
    /// ```
    pub fn has_trailing_slash(&self) -> bool {
        let path_end = self.path_end();
        path_end > 0 && &self[path_end - 1..path_end] == "/"
    }
}

/// ## Trimming
impl UriRef {
    /// Returns this URI-reference without a fragment.
    ///
    /// ## Examples
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(uri_ref!("http://a/#frag").trim_fragment(),  uri_ref!("http://a/"));
    /// assert_eq!(uri_ref!("a/b/c?blah#frag").trim_fragment(), uri_ref!("a/b/c?blah"));
    /// ```
    #[must_use = "this returns the trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_fragment(&self) -> &UriRef {
        if let Some(i) = self.fragment_start() {
            unsafe { Self::from_str_unchecked(&self[..i]) }
        } else {
            self
        }
    }

    /// Returns this URI without a query or fragment.
    ///
    /// ## Examples
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(uri_ref!("//foo/?bar").trim_query(),      uri_ref!("//foo/"));
    /// assert_eq!(uri_ref!("a/b/c?blah#frag").trim_query(), uri_ref!("a/b/c"));
    /// assert_eq!(uri_ref!("http://a/#frag").trim_query(),  uri_ref!("http://a/"));
    /// ```
    #[must_use = "this returns the trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_query(&self) -> &UriRef {
        if let Some(i) = self.find(|c| c == '?' || c == '#') {
            // SAFETY: Trimming on a boundary guaranteed not to be inside of an escaped byte.
            unsafe { Self::from_str_unchecked(&self[..i]) }
        } else {
            self
        }
    }

    /// Returns this URI without a path, query, or fragment.
    ///
    /// ## Examples
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(uri_ref!("a/b/c?blah#frag").trim_path(), uri_ref!(""));
    /// assert_eq!(uri_ref!("//foo/?bar").trim_path(),      uri_ref!("//foo"));
    /// assert_eq!(uri_ref!("http://a/#frag").trim_path(),  uri_ref!("http://a"));
    /// ```
    #[must_use = "this returns the trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_path(&self) -> &UriRef {
        let i = self.path_start();
        unsafe { Self::from_str_unchecked(&self[..i]) }
    }

    /// Returns this URI without the "heir-part", or anything thereafter.
    ///
    /// ## Examples
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(uri_ref!("a/b/c?blah#frag").trim_heir_part(), uri_ref!(""));
    /// assert_eq!(uri_ref!("//foo/?bar").trim_heir_part(), uri_ref!(""));
    /// assert_eq!(uri_ref!("http://a/#frag").trim_heir_part(), uri_ref!("http:"));
    /// ```
    #[must_use = "this returns the trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_heir_part(&self) -> &UriRef {
        let i = self.heir_part_start();
        unsafe { Self::from_str_unchecked(&self[..i]) }
    }

    /// Returns this URI without the trailing part of the path that would be
    /// removed during relative-reference resolution.
    ///
    /// ## Examples
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(uri_ref!("a").trim_resource(),               uri_ref!(""));
    /// assert_eq!(uri_ref!("a/b/c?blah#frag").trim_resource(), uri_ref!("a/b/"));
    /// assert_eq!(uri_ref!("/a").trim_resource(),              uri_ref!("/"));
    /// assert_eq!(uri_ref!("//foo/?bar").trim_resource(),      uri_ref!("//foo/"));
    /// assert_eq!(uri_ref!("http://a/#frag").trim_resource(),  uri_ref!("http://a/"));
    /// ```
    #[must_use = "this returns the trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_resource(&self) -> &UriRef {
        let mut ret = self.trim_query();

        let path_start = self.path_start();

        if let Some(i) = ret.rfind('/') {
            if i + 1 > path_start {
                ret = unsafe { Self::from_str_unchecked(&self[..i + 1]) };
            }
        } else if path_start == 0 {
            ret = uri_ref!("");
        }

        ret
    }

    /// Removes any trailing slash that might be at the end of the path, along with
    /// the query and fragment.
    ///
    /// If the path consists of a single slash ("`/`"), then it is not removed.
    ///
    /// ## Examples
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// assert_eq!(uri_ref!("a").trim_trailing_slash(),                uri_ref!("a"));
    /// assert_eq!(uri_ref!("a/b/c/?blah#frag").trim_trailing_slash(), uri_ref!("a/b/c"));
    /// assert_eq!(uri_ref!("/").trim_trailing_slash(),                uri_ref!("/"));
    /// assert_eq!(uri_ref!("//foo/?bar").trim_trailing_slash(),       uri_ref!("//foo/"));
    /// assert_eq!(uri_ref!("http://a/#frag").trim_trailing_slash(),   uri_ref!("http://a/"));
    /// ```
    ///
    /// Note that the uri-ref "`//`" (a network path with an empty authority and an empty path)
    /// does not get its trailing slash removed because it technically isn't a part of the path.
    /// Likewise, the uri-ref "`///`" doesn't get the last slash removed because this method
    /// won't remove the first slash in the path. The uri-ref "`////`" however will have its
    /// trailing slash removed:
    ///
    /// ```
    /// # use async_coap_uri::prelude::*;
    /// assert_eq!(uri_ref!("//").trim_trailing_slash(),    uri_ref!("//"));
    /// assert_eq!(uri_ref!("///").trim_trailing_slash(),   uri_ref!("///"));
    /// assert_eq!(uri_ref!("////").trim_trailing_slash(),  uri_ref!("///"));
    /// ```
    ///
    #[must_use = "this returns the trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_trailing_slash(&self) -> &UriRef {
        let path_end = self.path_end();
        if path_end > self.path_start() + 1 && &self[path_end - 1..path_end] == "/" {
            unsafe { Self::from_str_unchecked(&self[..path_end - 1]) }
        } else {
            self.trim_query()
        }
    }

    /// Attempts to shorten this URI-reference given a base URI reference.
    ///
    /// The returned reference can then be resolved using the base to recover the
    /// original URI reference.
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    /// let base = uri_ref!("http://example.com/a/b");
    /// let target = uri_ref!("http://example.com/a/x/y/");
    ///
    /// let shortened = target.trim_to_shorten(base).expect("Unable to shorten");
    /// assert_eq!(shortened, rel_ref!("x/y/"));
    ///
    /// let resolved = base.resolved(shortened).expect("Unable to resolve");
    /// assert_eq!(resolved, target);
    /// ```
    #[must_use]
    pub fn trim_to_shorten(&self, base: &UriRef) -> Option<&RelRef> {
        let (base_abs_part, base_rel_part) = base.trim_resource().split();
        let (self_abs_part, self_rel_part) = self.split();

        if self_abs_part.is_some() {
            if base_abs_part.is_none() || base_abs_part != self_abs_part {
                return None;
            }
        }

        if self_rel_part.starts_with(base_rel_part.as_str()) {
            Some(unsafe { RelRef::from_str_unchecked(&self_rel_part[base_rel_part.len()..]) })
        } else {
            None
        }
    }
}

/// # Unsafe Methods
///
/// `UriRef` needs some unsafe methods in order to function properly. This section is where
/// they are all located.
impl UriRef {
    /// Converts a string slice to a UriRef slice without checking
    /// that the string contains valid URI-Reference.
    ///
    /// See the safe version, [`from_str`](#method.from_str), for more information.
    ///
    /// ## Safety
    ///
    /// This function is unsafe because it does not check that the string passed to
    /// it is a valid URI-reference. If this constraint is violated, undefined behavior
    /// results.
    #[inline(always)]
    pub unsafe fn from_str_unchecked(s: &str) -> &UriRef {
        &*(s as *const str as *const UriRef)
    }

    /// Converts a string slice to a UriRef slice without checking
    /// that the string contains valid URI-Reference; mutable version.
    ///
    /// See the immutable version, [`from_str_unchecked`](#method.from_str), for more information.
    #[inline(always)]
    pub unsafe fn from_str_unchecked_mut(s: &mut str) -> &mut UriRef {
        &mut *(s as *mut str as *mut UriRef)
    }

    /// Returns this slice as a mutable `str` slice.
    ///
    /// ## Safety
    ///
    /// This is unsafe because it allows you to change the contents of the slice in
    /// such a way that would make it no longer consistent with the `UriRef`'s promise
    /// that it can only ever contain a valid URI-reference.
    #[inline(always)]
    pub unsafe fn as_mut_str(&mut self) -> &mut str {
        &mut self.0
    }

    /// Same as [`query_as_rel_ref()`](#method.query_as_rel), but mutable.
    #[doc(hidden)]
    #[must_use = "this returns a new slice, without modifying the original"]
    pub unsafe fn query_as_rel_ref_mut(&mut self) -> Option<&mut RelRef> {
        let self_ptr = self.as_ptr();
        let no_mut = self.query_as_rel_ref()?;
        let begin = no_mut.as_ptr() as usize - self_ptr as usize;
        let end = begin + no_mut.len();

        // SAFETY: We want to convert a `&UriRef` to be a `&mut UriRef`, and we
        //         and the behavior of using transmute to change mutability is
        //         undefined. So we figure out the begin and end of the query and
        //         use that range to make a new mutable slice. Queries with the
        //         `?` prepended are valid relative URIs, so we do an unchecked cast.
        //         to get those extra mechanisms.

        Some(RelRef::from_str_unchecked_mut(
            &mut self.as_mut_str()[begin..end],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        assert!(UriRef::from_str("http://example.com/").is_ok());
        assert!(UriRef::from_str("//example.com/").is_ok());
        assert!(UriRef::from_str("/a/b/c").is_ok());
        assert!(UriRef::from_str("a/b/c").is_ok());
        assert!(UriRef::from_str("?q=123").is_ok());
        assert!(UriRef::from_str("#frag").is_ok());
        assert!(UriRef::from_str("not%auri://a/b/c").is_err());
        assert!(UriRef::from_str("coap+sms://+1-234-567-8901/1/s/levl/v?inc").is_ok());
        assert!(UriRef::from_str("not a uri://a/b/c").is_err());
    }

    #[test]
    fn path_as_rel_ref() {
        assert_eq!(rel_ref!("example/"), uri_ref!("example/").path_as_rel_ref());
        assert_eq!(
            rel_ref!("/blah/"),
            uri_ref!("http://example.com/blah/").path_as_rel_ref()
        );
        assert_eq!(
            rel_ref!("example.com/blah/"),
            uri_ref!("http:example.com/blah/?q").path_as_rel_ref()
        );
    }

    #[test]
    fn has_trailing_slash() {
        assert_eq!(true, uri_ref!("example/").has_trailing_slash());
        assert_eq!(true, uri_ref!("/example/").has_trailing_slash());
        assert_eq!(true, uri_ref!("/example/#frag").has_trailing_slash());
        assert_eq!(true, uri_ref!("example/?query#frag").has_trailing_slash());
        assert_eq!(true, uri_ref!("coap://example//").has_trailing_slash());
        assert_eq!(false, uri_ref!("example").has_trailing_slash());
        assert_eq!(false, uri_ref!("example?/").has_trailing_slash());
        assert_eq!(false, uri_ref!("example#/").has_trailing_slash());
        assert_eq!(false, uri_ref!("example/x").has_trailing_slash());
        assert_eq!(false, uri_ref!("e/x/a/m/p/l/e?/#/").has_trailing_slash());
    }

    #[test]
    fn try_trim_resource() {
        assert_eq!(uri_ref!("example/"), uri_ref!("example/").trim_resource());
        assert_eq!(uri_ref!("/example/"), uri_ref!("/example/").trim_resource());
        assert_eq!(
            uri_ref!("/example/"),
            uri_ref!("/example/#frag").trim_resource()
        );
        assert_eq!(
            uri_ref!("example/"),
            uri_ref!("example/?query#frag").trim_resource()
        );
        assert_eq!(
            uri_ref!("coap://example//"),
            uri_ref!("coap://example//").trim_resource()
        );
        assert_eq!(uri_ref!(""), uri_ref!("example").trim_resource());
        assert_eq!(uri_ref!(""), uri_ref!("example?/").trim_resource());
        assert_eq!(uri_ref!(""), uri_ref!("example#/").trim_resource());
        assert_eq!(uri_ref!("example/"), uri_ref!("example/x").trim_resource());
        assert_eq!(
            uri_ref!("e/x/a/m/p/l/"),
            uri_ref!("e/x/a/m/p/l/e?/#/").trim_resource()
        );
    }

    #[test]
    fn trim_to_shorten() {
        assert_eq!(
            Some(rel_ref!("c")),
            uri_ref!("/a/b/c").trim_to_shorten(uri_ref!("/a/b/"))
        );
        assert_eq!(
            Some(rel_ref!("c/d/e")),
            uri_ref!("/a/b/c/d/e").trim_to_shorten(uri_ref!("/a/b/"))
        );
        assert_eq!(
            None,
            uri_ref!("/a/b/c/d/e").trim_to_shorten(uri_ref!("/a/c/"))
        );
        assert_eq!(
            Some(rel_ref!("c/d/e")),
            uri_ref!("/a/b/c/d/e").trim_to_shorten(uri_ref!("coap://blah/a/b/"))
        );
        assert_eq!(
            Some(rel_ref!("c/d/e")),
            uri_ref!("coap://blah/a/b/c/d/e").trim_to_shorten(uri_ref!("coap://blah/a/b/"))
        );
        assert_eq!(
            None,
            uri_ref!("coap://blah/a/b/c/d/e").trim_to_shorten(uri_ref!("/a/b/"))
        );
        assert_eq!(
            Some(rel_ref!("c")),
            uri_ref!("/a/b/c").trim_to_shorten(uri_ref!("/a/b/d"))
        );
    }

    #[test]
    fn userinfo_host_port() {
        let uri_test_table = vec![
            (
                uri_ref!("http://example.com/a/b/c"),
                Some((None, "example.com", None)),
            ),
            (
                uri_ref!("http://example.com:1234/a/b/c"),
                Some((None, "example.com", Some(1234u16))),
            ),
            (
                uri_ref!("http://example.com:/a/b/c"),
                Some((None, "example.com", None)),
            ),
            (
                uri_ref!("http://username@example.com/a/b/c"),
                Some((Some("username"), "example.com", None)),
            ),
            (
                uri_ref!("http://username:password@example.com/a/b/c"),
                Some((Some("username:password"), "example.com", None)),
            ),
            (
                uri_ref!("http://username:password@example.com:1234/a/b/c"),
                Some((Some("username:password"), "example.com", Some(1234))),
            ),
            (
                uri_ref!("http://username:password@example.com:1234567/a/b/c"),
                Some((Some("username:password"), "example.com", None)),
            ),
            (uri_ref!("http://[::1]/a/b/c"), Some((None, "::1", None))),
            (
                uri_ref!("http://[::1]:1234/a/b/c"),
                Some((None, "::1", Some(1234))),
            ),
            (
                uri_ref!("http://username:password@[::1]:1234/a/b/c"),
                Some((Some("username:password"), "::1", Some(1234))),
            ),
        ];

        for (a, b) in uri_test_table.iter() {
            assert_eq!(*b, a.raw_userinfo_host_port(), "Failed for: <{}>", a);
        }
    }
}
