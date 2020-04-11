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
use std::fmt;
use std::ops::Deref;
use std::ptr;

/// Unsized string-slice type guaranteed to contain a well-formed [IETF-RFC3986] [relative reference].
///
/// [relative reference]: https://tools.ietf.org/html/rfc3986#section-4.2
///
/// The sized counterpart is [`RelRefBuf`].
///
/// *This type cannot hold a network path*. If this type contains a path that looks like a network
/// path, it will be considered [degenerate](crate::RelRef::is_degenerate) and you will not be able
/// to losslessly convert it to a [`UriRef`](crate::UriRef) or [`UriRefBuf`](crate::UriRefBuf).
/// See ["Network Path Support"](index.html#network-path-support) for more details.
///
/// You can create static constants with this class by using the [`rel_ref!`] macro:
///
/// [`rel_ref!`]: macro.rel_ref.html
///
/// ```
/// # use async_coap_uri::*;
/// let uri = rel_ref!("/test?query");
/// let components = uri.components();
///
/// assert_eq!(None,          components.scheme());
/// assert_eq!(None,          components.raw_host());
/// assert_eq!(None,          components.port());
/// assert_eq!("/test",       components.raw_path());
/// assert_eq!(Some("query"), components.raw_query());
/// ```
///
/// ## RelRef and Deref
///
/// You might think that since both relative and absolute URIs are just special
/// cases of URIs that they could both safely implement [`Deref<Target=UriRef>`](core::ops::Deref).
/// This is true for [`Uri`], but not [`RelRef`]. This section is dedicated to explaining why.
///
/// There is this pesky [section 4.2 of RFC3986](https://tools.ietf.org/html/rfc3986#section-4.2)
/// that throws a wrench into that noble endeavour:
///
/// >  A path segment that contains a colon character (e.g., "this:that")
/// >  cannot be used as the first segment of a relative-path reference, as
/// >  it would be mistaken for a scheme name.  Such a segment must be
/// >  preceded by a dot-segment (e.g., "./this:that") to make a relative-
/// >  path reference.
///
/// This causes big problems for type-safety when derefing a [`RelRef`] into a [`UriRef`]: there is
/// no way for [`UriRef`] to know that it came from a [`RelRef`] and thus recognize that something
/// like `rel_ref!("this:that")` does *NOT* have a scheme of `this`.
///
/// These are tricky edge cases that have serious security implications---it's important
/// that this case be considered and handled appropriately.
///
/// The solution used in this library is to make the transition from [`RelRef`] to [`UriRef`] not
/// guaranteed. However, a transition from a [`RelRef`] to a [`RelRefBuf`] is guaranteed, since the
/// offending colon can be escaped in that case. This is preferred instead of prepending a `"./"`,
/// due to the additional complications that could occur when manipulating paths.
///
/// You can check any [`RelRef`] for this degenerate condition via the method
/// [`is_degenerate()`](#method.is_degenerate).
///
/// [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
#[derive(Eq, Hash)]
pub struct RelRef(pub(super) UriRef);

_impl_uri_traits_base!(RelRef);

impl Deref for RelRef {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Default for &RelRef {
    /// Returns an *empty relative reference*.
    ///
    /// Empty relative references do nothing but clear the base fragment when resolved
    /// against a base.
    fn default() -> Self {
        irel_ref!("")
    }
}

impl Default for &mut RelRef {
    /// Mutable version of `(&RelRef)::default`.
    ///
    /// Despite being marked mutable, since the length is zero the value is effectively immutable.
    fn default() -> Self {
        use std::slice::from_raw_parts_mut;
        use std::str::from_utf8_unchecked_mut;
        unsafe {
            // SAFETY: An empty slice is pretty harmless, mutable or not.
            let empty_slice = from_raw_parts_mut(ptr::null_mut::<u8>(), 0);
            let empty_string = from_utf8_unchecked_mut(empty_slice);
            RelRef::from_str_unchecked_mut(empty_string)
        }
    }
}

impl AnyUriRef for RelRef {
    unsafe fn write_to_unsafe<T: fmt::Write + ?Sized>(&self, write: &mut T) -> fmt::Result {
        if let Some(i) = self.colon_in_first_path_segment() {
            write!(write, "{}%3A{}", &self[..i], &self[i + 1..])
        } else {
            if self.starts_with("//") {
                write.write_str("/.")?;
            }
            write.write_str(self.as_str())
        }
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Determines what kind of relative reference this is:
    ///
    /// This function may return any one of the following values:
    ///
    /// * [`UriType::Fragment`](enum.UriType.html#variant.Fragment)
    /// * [`UriType::Query`](enum.UriType.html#variant.Query)
    /// * [`UriType::AbsolutePath`](enum.UriType.html#variant.AbsolutePath)
    /// * [`UriType::RelativePath`](enum.UriType.html#variant.RelativePath)
    fn uri_type(&self) -> UriType {
        if self.starts_with('#') {
            UriType::Fragment
        } else if self.starts_with('?') {
            UriType::Query
        } else if self.starts_with('/') {
            UriType::AbsolutePath
        } else {
            UriType::RelativePath
        }
    }

    /// Breaks down this relative reference into its [raw components][UriRawComponents].
    fn components(&self) -> UriRawComponents<'_> {
        UriRawComponents {
            scheme: None,
            authority: None,
            userinfo: None,
            host: None,
            port: None,
            path: self.path_as_rel_ref(),
            query: self.raw_query(),
            fragment: self.raw_fragment(),
        }
    }
}

/// RelRef will always format the relative reference for display in an unambiguous fashion.
impl fmt::Display for RelRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(f)
    }
}

impl RelRef {
    /// Attempts to convert a string slice into a [`&RelRef`](RelRef), returning `Err(ParseError)`
    /// if the string slice contains data that is not a valid relative-reference.
    pub fn from_str(s: &str) -> Result<&RelRef, ParseError> {
        if let Some(first_error) = s.unescape_uri().first_error() {
            Err(ParseError::new(
                "Bad percent encoding or illegal characters",
                Some(first_error..first_error + 1),
            ))
        } else {
            Ok(unsafe { Self::from_str_unchecked(s) })
        }
    }

    /// Determines if the given string can be considered a well-formed [relative-reference].
    /// [relative-reference]: https://tools.ietf.org/html/rfc3986#section-4.2
    pub fn is_str_valid<S: AsRef<str>>(s: S) -> bool {
        s.as_ref().unescape_uri().first_error().is_none()
    }

    /// Constructs a new `RelRefBuf` from a `&RelRef`, disambiguating if degenerate.
    #[inline(always)]
    pub fn to_rel_ref_buf(&self) -> RelRefBuf {
        RelRefBuf::from_rel_ref(self)
    }

    /// Constructs a new `UriRefBuf` from a `&RelRef`, disambiguating if degenerate.
    pub fn to_uri_ref_buf(&self) -> UriRefBuf {
        self.to_rel_ref_buf().into()
    }

    /// Casts this relative reference to a string slice.
    #[inline(always)]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Casts a non-degenerate relative reference to a `&UriRef`.
    /// Returns `None` if the relative reference [is degenerate][RelRef::is_degenerate].
    pub fn try_as_uri_ref(&self) -> Option<&UriRef> {
        if self.is_degenerate() {
            None
        } else {
            Some(&self.0)
        }
    }

    /// Returns a [`Cow<UriRef>`] that usually just contains a reference to
    /// this slice, but will contain an owned instance if this relative reference
    /// [is degenerate][RelRef::is_degenerate].
    #[cfg(feature = "std")]
    pub fn as_uri_ref(&self) -> Cow<'_, UriRef> {
        if let Some(uri_ref) = self.try_as_uri_ref() {
            Cow::Borrowed(uri_ref)
        } else {
            Cow::Owned(self.to_uri_ref_buf())
        }
    }
}

/// # URI Component Accessors
impl RelRef {
    /// Trims the query and fragment from this relative reference, leaving only the path.
    ///
    /// See also [`RelRef::trim_query`].
    #[must_use = "this returns a new slice, without modifying the original"]
    pub fn path_as_rel_ref(&self) -> &RelRef {
        self.trim_query()
    }

    /// See [`UriRef::query_as_rel_ref`] for more information.
    #[must_use = "this returns a new slice, without modifying the original"]
    #[inline(always)]
    pub fn query_as_rel_ref(&self) -> Option<&RelRef> {
        self.0.query_as_rel_ref()
    }

    /// See [`UriRef::raw_path`] for more information.
    #[must_use]
    #[inline(always)]
    pub fn raw_path(&self) -> &str {
        self.path_as_rel_ref().as_str()
    }

    /// See [`UriRef::raw_query`] for more information.
    #[must_use = "this returns a new slice, without modifying the original"]
    #[inline(always)]
    pub fn raw_query(&self) -> Option<&str> {
        self.0.raw_query()
    }

    /// See [`UriRef::raw_fragment`] for more information.
    #[must_use = "this returns a new slice, without modifying the original"]
    #[inline(always)]
    pub fn raw_fragment(&self) -> Option<&str> {
        self.0.raw_fragment()
    }

    /// See [`UriRef::raw_path_segments`] for more information.
    pub fn raw_path_segments(&self) -> impl Iterator<Item = &str> {
        let path = self.path_as_rel_ref();

        let mut ret = path.as_str().split('/');

        if path.is_empty() {
            // Skip non-existant segments
            let _ = ret.next();
        } else if path.starts_with('/') {
            // Skip leading slash.
            let _ = ret.next();
        }

        ret
    }

    /// See [`UriRef::raw_query_items`] for more information.
    #[inline(always)]
    pub fn raw_query_items(&self) -> impl Iterator<Item = &str> {
        self.0.raw_query_items()
    }

    /// See [`UriRef::raw_query_key_values`] for more information.
    #[inline(always)]
    pub fn raw_query_key_values(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.raw_query_key_values()
    }

    /// See [`UriRef::fragment`] for more information.
    #[must_use]
    #[cfg(feature = "std")]
    #[inline(always)]
    pub fn fragment(&self) -> Option<Cow<'_, str>> {
        self.0.fragment()
    }

    /// See [`UriRef::path_segments`] for more information.
    #[cfg(feature = "std")]
    #[inline(always)]
    pub fn path_segments(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.0.path_segments()
    }

    /// See [`UriRef::query_items`] for more information.
    #[cfg(feature = "std")]
    #[inline(always)]
    pub fn query_items(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.0.query_items()
    }

    /// See [`UriRef::query_key_values`] for more information.
    #[cfg(feature = "std")]
    #[inline(always)]
    pub fn query_key_values(&self) -> impl Iterator<Item = (Cow<'_, str>, Cow<'_, str>)> {
        self.0.query_key_values()
    }

    /// See [`UriRef::has_trailing_slash`] for more information.
    #[must_use]
    #[inline(always)]
    pub fn has_trailing_slash(&self) -> bool {
        self.0.has_trailing_slash()
    }

    /// Determines if this [`RelRef`] is degenerate specifically because it is a relative path
    /// with a colon in the first path segment and no special characters appearing
    /// before it.
    ///
    /// See the section ["RelRef"](#relref-and-deref) for more details.
    #[must_use]
    pub fn colon_in_first_path_segment(&self) -> Option<usize> {
        for (i, b) in self.bytes().enumerate() {
            match b {
                b if i == 0 && (b as char).is_numeric() => return None,
                b if (b as char).is_ascii_alphanumeric() => continue,
                b'+' | b'-' | b'.' => continue,
                b':' => return Some(i),
                _ => return None,
            }
        }
        None
    }

    /// Determines if this [`RelRef`] is degenerate.
    ///
    /// See the section ["RelRef"](#relref-and-deref) for more details.
    pub fn is_degenerate(&self) -> bool {
        self.starts_with("//") || self.colon_in_first_path_segment().is_some()
    }
}

/// # URI Resolution
impl RelRef {
    /// Resolves a relative URI against this relative URI, yielding a
    /// new relative URI as a `RelRefBuf`.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn resolved_rel_ref<UF: AsRef<RelRef>>(&self, dest: UF) -> RelRefBuf {
        let mut ret = String::with_capacity(self.len() + dest.as_ref().len());

        self.write_resolved(dest.as_ref(), &mut ret)
            .expect("URI resolution failed");
        ret.shrink_to_fit();

        // SAFETY: `write_resolved` is guaranteed to write well-formed RelRefs when
        //         both the base and target are RelRefs.
        let mut ret = unsafe { RelRefBuf::from_string_unchecked(ret) };

        ret.disambiguate();

        ret
    }
}

/// # Trimming
impl RelRef {
    /// Returns this relative reference slice without the fragment component.
    #[must_use = "this returns the trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_fragment(&self) -> &RelRef {
        // SAFETY: Trimming on a boundary guaranteed not to be inside of an escaped byte.
        unsafe { RelRef::from_str_unchecked(self.0.trim_fragment().as_str()) }
    }

    /// Returns this relative reference slice without the query or fragment components.
    #[must_use = "this returns the trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_query(&self) -> &RelRef {
        // SAFETY: Trimming on a boundary guaranteed not to be inside of an escaped byte.
        unsafe { RelRef::from_str_unchecked(self.0.trim_query().as_str()) }
    }

    /// See [`UriRef::trim_resource`] for more information.
    #[must_use = "this returns the trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_resource(&self) -> &RelRef {
        // SAFETY: Trimming on a boundary guaranteed not to be inside of an escaped byte.
        unsafe { RelRef::from_str_unchecked(self.0.trim_resource().as_str()) }
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
    /// assert_eq!(rel_ref!("a").trim_trailing_slash(),               rel_ref!("a"));
    /// assert_eq!(rel_ref!("a/b/c/?blah#frag").trim_trailing_slash(),rel_ref!("a/b/c"));
    /// assert_eq!(rel_ref!("/").trim_trailing_slash(),               rel_ref!("/"));
    /// assert_eq!(rel_ref!(unsafe "//").trim_trailing_slash(),       rel_ref!("/"));
    /// assert_eq!(rel_ref!(unsafe "//foo/?bar").trim_trailing_slash(),rel_ref!(unsafe "//foo"));
    /// ```
    ///
    /// Note that the behavior of this method is different than the behavior for
    /// [`UriRef::trim_trailing_slash`]\: "`//`" is considered to be a path starting with two
    /// slashes rather than a network path with an empty authority and an empty path:
    ///
    /// ```
    /// # use async_coap_uri::prelude::*;
    /// assert_eq!(rel_ref!(unsafe "//").trim_trailing_slash(),    rel_ref!("/"));
    /// assert_eq!(rel_ref!(unsafe "///").trim_trailing_slash(),   rel_ref!(unsafe "//"));
    /// assert_eq!(rel_ref!(unsafe "////").trim_trailing_slash(),  rel_ref!(unsafe "///"));
    /// ```
    ///
    #[must_use = "this returns the trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_trailing_slash(&self) -> &RelRef {
        let path_end = self.0.path_end();
        if path_end > 1 && &self[path_end - 1..path_end] == "/" {
            unsafe { Self::from_str_unchecked(&self[..path_end - 1]) }
        } else {
            self.trim_query()
        }
    }

    /// Returns this relative reference slice without any leading slashes.
    #[must_use = "this returns the trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_leading_slashes(&self) -> &RelRef {
        // SAFETY: Trimming on a boundary guaranteed not to be inside of an escaped byte.
        unsafe { RelRef::from_str_unchecked(self.trim_start_matches('/')) }
    }

    /// Returns this relative reference slice without any leading instances of `"./"` or `"/."`.
    #[must_use = "this returns the trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_leading_dot_slashes(&self) -> &RelRef {
        // SAFETY: Trimming on a boundary guaranteed not to be inside of an escaped byte.
        unsafe {
            let mut str_ref = self.as_str();

            while str_ref.starts_with("/./") {
                str_ref = &str_ref[2..];
            }

            str_ref = str_ref.trim_start_matches("./");
            if str_ref == "." {
                str_ref = &str_ref[..0];
            }
            RelRef::from_str_unchecked(str_ref)
        }
    }

    /// Returns this relative reference slice without its first path segment.
    #[must_use = "this returns the leading path item trimmed uri as a new slice, \
                  without modifying the original"]
    pub fn trim_leading_path_segment(&self) -> (&str, &RelRef) {
        let trimmed = self.trim_leading_slashes();
        if let Some(i) = trimmed.find(|c| c == '/' || c == '?' || c == '#') {
            match trimmed.as_bytes()[i] {
                b'/' => (&trimmed[..i], unsafe {
                    // SAFETY: Trimming on a boundary guaranteed not to
                    //         be inside of an escaped byte.
                    RelRef::from_str_unchecked(&trimmed[i + 1..])
                }),
                _ => (&trimmed[..i], unsafe {
                    // SAFETY: Trimming on a boundary guaranteed not to
                    //         be inside of an escaped byte.
                    RelRef::from_str_unchecked(&trimmed[i..])
                }),
            }
        } else {
            (trimmed.as_str(), unsafe {
                // SAFETY: Trimming on a boundary guaranteed not to
                //         be inside of an escaped byte.
                RelRef::from_str_unchecked(&trimmed[trimmed.len()..])
            })
        }
    }

    #[must_use]
    fn _trim_leading_n_path_segments(&self, n: usize) -> (&str, &RelRef) {
        let mut next = self;

        for _ in 0..n {
            next = next.trim_leading_path_segment().1;
        }

        let i = next.as_ptr() as usize - self.as_ptr() as usize;

        ((&self[..i]).trim_end_matches('/'), next)
    }

    /// Returns a tuple with a string slice contianing the first `n` path segments and
    /// a `&RelRef` containing the rest of the relative reference.
    #[must_use = "this returns the trimmed uri as a new slice, without modifying the original"]
    pub fn trim_leading_n_path_segments(&self, n: usize) -> (&str, &RelRef) {
        self.trim_leading_slashes()._trim_leading_n_path_segments(n)
    }

    /// Attempts to return a shortened version of this relative reference that is
    /// relative to `base`.
    #[must_use = "this returns the trimmed uri as a new slice, without modifying the original"]
    pub fn trim_to_shorten(&self, base: &RelRef) -> Option<&RelRef> {
        self.0.trim_to_shorten(base.try_as_uri_ref()?)
    }
}

/// # Unsafe Methods
///
/// `RelRef` needs some unsafe methods in order to function properly. This section is where
/// they are all located.
impl RelRef {
    /// Converts a string slice to a `RelRef` slice without checking
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
    pub unsafe fn from_str_unchecked(s: &str) -> &RelRef {
        &*(s as *const str as *const RelRef)
    }

    /// Converts a string slice to a `RelRef` slice without checking
    /// that the string contains valid URI-Reference; mutable version.
    ///
    /// See the immutable version, [`from_str_unchecked`](#method.from_str), for more information.
    #[inline(always)]
    pub unsafe fn from_str_unchecked_mut(s: &mut str) -> &mut RelRef {
        &mut *(s as *mut str as *mut RelRef)
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
        self.0.as_mut_str()
    }

    /// Directly converts this `&RelRef` to a `&UriRef`, without performing the
    /// checks that [`as_uri_ref()`](#method.as_uri_ref) does.
    ///
    /// This is unsafe for the reasons described [here](#relref-and-deref).
    #[inline(always)]
    pub const unsafe fn as_uri_ref_unchecked(&self) -> &UriRef {
        &self.0
    }

    /// Mutable version of [`RelRef::path_as_rel_ref`]. Trims the query and fragment from this
    /// relative reference, leaving only the path.
    #[doc(hidden)]
    #[must_use = "this returns a new slice, without modifying the original"]
    pub unsafe fn path_as_rel_ref_mut(&mut self) -> &mut RelRef {
        let i = self.trim_query().len();
        let str_mut: &mut str = core::mem::transmute(self.as_mut_str());

        RelRef::from_str_unchecked_mut(&mut str_mut[..i])
    }

    /// See [`UriRef::query_as_rel_ref_mut`] for more information.
    #[doc(hidden)]
    #[must_use = "this returns a new slice, without modifying the original"]
    pub unsafe fn query_as_rel_ref_mut(&mut self) -> Option<&mut RelRef> {
        self.0.query_as_rel_ref_mut()
    }

    /// **Experimental**: Similar to [`raw_path_segment_iter()`], but uses the space of the mutable `UriRef`
    /// to individually unescape the items.
    ///
    /// ## Safety
    ///
    /// This method is marked as unsafe because the contents of `self` is undefined
    /// after it terminates. The method can be used safely as long the buffer which
    /// held `self` is no longer accessed directly. See [`UriUnescapeBuf`] for an example.
    ///
    /// [`raw_path_segment_iter()`]: #method.raw_path_segment_iter
    pub unsafe fn unsafe_path_segment_iter(&mut self) -> impl Iterator<Item = &str> {
        let path = self.path_as_rel_ref_mut();
        let is_empty = path.is_empty();
        let starts_with_slash = path.starts_with('/');

        let mut_bytes = path.as_mut_str().as_bytes_mut();

        let mut ret = mut_bytes.split_mut(|b| *b == b'/').filter_map(|seg| {
            let seg = std::str::from_utf8_unchecked_mut(seg);
            if seg == "." {
                None
            } else {
                Some(&*seg.unescape_uri_in_place())
            }
        });

        if is_empty || starts_with_slash {
            // Skip non-existant segments or leading slash
            let _ = ret.next();
        }

        ret
    }

    /// **Experimental**: Similar to [`raw_query_item_iter()`], but uses the space of the mutable `UriRef`
    /// to individually unescape the query items.
    ///
    /// ## Safety
    ///
    /// This method is marked as unsafe because the contents of `self` is undefined
    /// after it terminates. The method can be used safely as long the `&mut UriRef` (and its
    /// owner) is never directly used again. See [`UriUnescapeBuf`] for an example.
    ///
    /// [`raw_query_item_iter()`]: #method.raw_query_item_iter
    pub unsafe fn unsafe_query_item_iter(&mut self) -> impl Iterator<Item = &str> {
        let query = self.query_as_rel_ref_mut().unwrap_or_default();
        let is_empty = query.is_empty();
        let starts_with_delim = query.starts_with(|c| c == '&' || c == ';');

        let mut mut_bytes = query.as_mut_str().as_bytes_mut();

        if !is_empty && mut_bytes[0] == b'?' {
            mut_bytes = &mut mut_bytes[1..];
        }

        let mut ret = mut_bytes
            .split_mut(|b| *b == b'&' || *b == b';')
            .map(|seg| {
                let seg = std::str::from_utf8_unchecked_mut(seg);
                &*seg.unescape_uri_in_place()
            });

        if is_empty || starts_with_delim {
            // Skip non-existant segments or leading slash
            let _ = ret.next();
        }

        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path() {
        assert_eq!(
            irel_ref!("example/"),
            irel_ref!("example/").path_as_rel_ref()
        );
        assert_eq!(
            irel_ref!(unsafe "http:example.com/blah/"),
            irel_ref!(unsafe "http:example.com/blah/?q").path_as_rel_ref()
        );
    }

    #[test]
    fn path_segment_iter() {
        assert_eq!(
            vec!["example", ""],
            irel_ref!("example/")
                .raw_path_segments()
                .collect::<Vec::<_>>()
        );
        assert_eq!(
            vec!["http:example.com", "blah", ""],
            irel_ref!(unsafe "http:example.com/blah/?q")
                .raw_path_segments()
                .collect::<Vec::<_>>()
        );
    }

    #[test]
    fn avoid_scheme_confusion() {
        assert_eq!(None, irel_ref!("this/that").colon_in_first_path_segment());
        assert_eq!(None, irel_ref!("1this:that").colon_in_first_path_segment());
        assert_eq!(None, irel_ref!("/this:that").colon_in_first_path_segment());
        assert_eq!(
            None,
            irel_ref!("%20this:that").colon_in_first_path_segment()
        );
        assert_eq!(
            Some(4),
            irel_ref!(unsafe "this:that").colon_in_first_path_segment()
        );
        assert_eq!(
            Some(4),
            irel_ref!(unsafe "th1s:that").colon_in_first_path_segment()
        );
        assert_eq!(
            None,
            irel_ref!(unsafe "this:that").to_uri_ref_buf().scheme()
        );
        assert_eq!(None, irel_ref!(unsafe "this:that").try_as_uri_ref());
        assert_eq!(
            &irel_ref!(unsafe "this:that").to_uri_ref_buf(),
            irel_ref!("this%3Athat"),
        );
    }

    #[test]
    fn trim_leading_path_segment() {
        assert_eq!(
            ("example", irel_ref!("")),
            irel_ref!("example/").trim_leading_path_segment()
        );
        assert_eq!(
            ("example", irel_ref!("")),
            irel_ref!("/example/").trim_leading_path_segment()
        );
        assert_eq!(
            ("a", irel_ref!("b/c/d/")),
            irel_ref!("a/b/c/d/").trim_leading_path_segment()
        );
        assert_eq!(
            ("a", irel_ref!("?query")),
            irel_ref!("a?query").trim_leading_path_segment()
        );
        assert_eq!(
            ("a", irel_ref!("#frag")),
            irel_ref!("a#frag").trim_leading_path_segment()
        );
        assert_eq!(
            ("fool:ish", irel_ref!("/thoughts?")),
            irel_ref!(unsafe "fool:ish//thoughts?").trim_leading_path_segment()
        );
        assert_eq!(
            ("", irel_ref!("")),
            irel_ref!("").trim_leading_path_segment()
        );
    }

    #[test]
    fn trim_leading_n_path_segments() {
        assert_eq!(
            ("", irel_ref!("a/b/c/d")),
            irel_ref!("a/b/c/d").trim_leading_n_path_segments(0)
        );
        assert_eq!(
            ("a", irel_ref!("b/c/d")),
            irel_ref!("a/b/c/d").trim_leading_n_path_segments(1)
        );
        assert_eq!(
            ("a/b", irel_ref!("c/d")),
            irel_ref!("a/b/c/d").trim_leading_n_path_segments(2)
        );
        assert_eq!(
            ("a/b/c", irel_ref!("d")),
            irel_ref!("a/b/c/d").trim_leading_n_path_segments(3)
        );
        assert_eq!(
            ("a/b/c/d", irel_ref!("")),
            irel_ref!("a/b/c/d").trim_leading_n_path_segments(4)
        );
        assert_eq!(
            ("a/b/c/d", irel_ref!("")),
            irel_ref!("a/b/c/d").trim_leading_n_path_segments(5)
        );

        assert_eq!(
            ("a/b/c", irel_ref!("d?blah")),
            irel_ref!("a/b/c/d?blah").trim_leading_n_path_segments(3)
        );
        assert_eq!(
            ("a/b/c/d", irel_ref!("?blah")),
            irel_ref!("a/b/c/d?blah").trim_leading_n_path_segments(4)
        );
        assert_eq!(
            ("a/b/c/d", irel_ref!("?blah")),
            irel_ref!("a/b/c/d?blah").trim_leading_n_path_segments(5)
        );

        assert_eq!(
            ("a/b/c", irel_ref!("d?blah")),
            irel_ref!("/a/b/c/d?blah").trim_leading_n_path_segments(3)
        );
    }
}
