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
use core::fmt::Display;
use core::ops::Deref;

/// Trait for objects that represent logical URI-references. Useful for generic programming.
///
pub trait AnyUriRef {
    /// Returns a `UriRawComponents` instance which contains all of the components for this
    /// URI reference.
    ///
    /// This is the only method that is required to be implemented---all other methods have
    /// defaults in place which use this method, but they may be inefficient.
    #[must_use]
    fn components(&self) -> UriRawComponents<'_>;

    /// Returns true if the underlying URI-reference is actually the empty reference.
    #[must_use]
    fn is_empty(&self) -> bool {
        self.components().is_empty()
    }

    /// Gets the [`UriType`] of the underlying URI-reference.
    ///
    /// [`UriType`]: enum.UriType.html
    #[must_use]
    fn uri_type(&self) -> UriType {
        self.components().uri_type()
    }

    /// Creates a new [`UriRefBuf`] from this [`AnyUriRef`].
    ///
    /// The default implementation uses [`AnyUriRef::write_to_unsafe`] to render out
    /// the content of the URI-reference.
    #[cfg(feature = "std")]
    #[must_use]
    fn to_uri_ref_buf(&self) -> UriRefBuf {
        unsafe { UriRefBuf::from_string_unchecked(self.display().to_string()) }
    }

    /// Hook for custom URI serialization implementation via [`AnyUriRefExt::write_to`].
    /// **Override with care!**
    ///
    /// In general, you should never need to call this method directly: use
    /// [`AnyUriRefExt::write_to`] instead. See the documentation for [`AnyUriRefExt::write_to`]
    /// for more details on usage.
    ///
    /// This method is marked as `unsafe`. However, *only overriding the default implementation is
    /// considered unsafe*: calling the method is actually safe. You can indirectly call this method
    /// from safe code by calling [`AnyUriRefExt::write_to`], which is a non-overridable wrapper
    /// around this method.
    ///
    /// # Override Safety
    ///
    /// Calling this method is not unsafe, *but overriding it is*! The underlying
    /// guarantee is that the written URI reference SHALL be well-formed. Lots of
    /// code depends on this guarantee in order to avoid undefined behavior.
    ///
    /// Bottom line: **If this method's implementation writes out something that is
    /// not a valid, well-formed URI-reference, the resulting behavior is undefined.**
    unsafe fn write_to_unsafe<T: core::fmt::Write + ?Sized>(
        &self,
        write: &mut T,
    ) -> Result<(), core::fmt::Error> {
        self.components().write_to(write)
    }
}

/// Extension trait for [`AnyUriRef`] that provides methods that cannot be overridden from
/// their default implementations.
///
/// This trait is automatically implemented for all types that implement [`AnyUriRef`].
pub trait AnyUriRefExt: AnyUriRef {
    /// Wraps this `AnyUriRef` instance in a [`UriDisplay`] object for use with formatting
    /// macros like `write!` and `format!`.
    ///
    /// The resulting instance will ultimately use the [`AnyUriRef::write_to_unsafe`] method
    /// to render the URI-reference.
    ///
    /// This method is similar to the [`display`][display-path] method on [`std::path::Path`].
    ///
    /// [display-path]: std::path::Path::display
    ///
    /// ## Example
    ///
    /// ```
    /// use async_coap_uri::prelude::*;
    ///
    /// let uri_ref = uri_ref!("http://example.com/");
    ///
    /// println!("uri_ref = {}", uri_ref.display());
    /// ```
    ///
    /// [`UriDisplay`]: struct.UriDisplay.html
    #[must_use]
    fn display(&self) -> UriDisplay<'_, Self> {
        UriDisplay(self)
    }

    /// Serializes this URI to anything implementing [`core::fmt::Write`].
    ///
    /// The purpose of this method is to provide a uniform way for a type that implements
    /// [`AnyUriRef`] to write out a well-formed URI; without making any assumptions on what
    /// that type might write out for [`std::fmt::Display`] (or even if it implements it at all).
    ///
    /// See the documentation for [`display`](AnyUriRefExt::display) and [`UriDisplay`] for
    /// examples of usage.
    ///
    /// You can't change the implementation of this method directly, this method simply calls
    /// [`AnyUriRef::write_to_unsafe`], which can be overridden (albeit unsafely). Be sure to
    /// follow the warnings in the associated documentation.
    fn write_to<T: core::fmt::Write + ?Sized>(
        &self,
        write: &mut T,
    ) -> Result<(), core::fmt::Error> {
        unsafe { self.write_to_unsafe(write) }
    }

    /// Writes out to a [`core::fmt::Write`] instance the result of performing URI resolution
    /// against `target`, with `self` being the base URI.
    fn write_resolved<T: core::fmt::Write + ?Sized, D: AnyUriRef + ?Sized>(
        &self,
        target: &D,
        f: &mut T,
    ) -> Result<(), ResolveError> {
        // This implementation is kind of a mess, but it does work and it does
        // pass the rather large corpus of unit tests. It eventually needs to be
        // rewritten to avoid memory allocation.
        // TODO(#9): Rewrite `AnyUriRef::write_resolved` to not use any memory allocation.

        if target.is_empty() {
            self.write_to(f)?;
            return Ok(());
        }

        let target_type = target.uri_type();

        let target_components = target.components();

        let base_type = self.uri_type();

        // Handle some exceptions.
        if base_type.cannot_be_a_base() {
            match target_type {
                UriType::Fragment => {
                    self.components().trim_fragment().write_to(f)?;
                    target.write_to(f)?;
                    return Ok(());
                }
                UriType::Query => {
                    self.components().trim_query().write_to(f)?;
                    target.write_to(f)?;
                    return Ok(());
                }
                x if x.is_ietf_rfc3986_relative_reference() => {
                    return Err(ResolveError::CannotBeABase);
                }
                _ => (),
            }
        }

        if target_components.scheme.is_some() {
            target.write_to(f)?;
            return Ok(());
        }

        let mut components = self.components();

        if target_components.authority.is_some() {
            components.authority = target_components.authority;
        }

        // Target fragment always gets used.
        components.fragment = target_components.fragment;
        if target_components.query.is_some() {
            components.query = target_components.query;
        } else if !target_components.path.is_empty() || target_components.authority.is_some() {
            components.query = None;
        }

        if let Some(scheme) = components.scheme {
            f.write_str(scheme)?;
            f.write_char(':')?;
        }

        if let Some(authority) = components.authority {
            f.write_str("//")?;
            f.write_str(authority)?;
        }

        let mut base_path = components.path_as_rel_ref();
        let target_path = target_components.path_as_rel_ref();

        if !target_path.is_empty() || !target_type.has_absolute_path() {
            let target_starts_with_slash = target_path.starts_with('/');
            let base_starts_with_slash = base_path.starts_with('/');

            if target_type.has_absolute_path() {
                if base_starts_with_slash {
                    base_path = irel_ref!("");
                } else {
                    base_path = irel_ref!("/");
                }
            } else if !target_path.is_empty() {
                base_path = base_path.trim_resource();
            }

            let mut out_path_vec = Vec::new();

            let seg_iter = base_path
                .raw_path_segments()
                .chain(target_path.raw_path_segments());

            let path_will_be_absolute = target_starts_with_slash
                || base_starts_with_slash
                || (base_type.has_absolute_path() && !target_path.is_empty());

            for seg in seg_iter {
                match seg {
                    "." => {
                        let last = out_path_vec.last().copied();

                        if last.map(str::is_empty) == Some(false) {
                            out_path_vec.push("");
                        }
                        continue;
                    }
                    ".." => {
                        let mut last = out_path_vec.pop();

                        if last == Some("") {
                            last = out_path_vec.pop();
                        }

                        match (last, path_will_be_absolute, out_path_vec.is_empty()) {
                            (Some("."), false, _) => out_path_vec.push(".."),
                            (Some(".."), false, _) => {
                                out_path_vec.push("..");
                                out_path_vec.push("..");
                            }
                            (Some(_), true, _) => out_path_vec.push(""),
                            (Some(_), false, false) => out_path_vec.push(""),
                            (Some(_), false, true) => out_path_vec.push("."),
                            (None, _, _) => (),
                        };
                    }
                    seg => {
                        match out_path_vec.last().copied() {
                            Some(".") if seg.is_empty() => continue,
                            Some(".") | Some("") => {
                                out_path_vec.pop();
                            }
                            _ => (),
                        };
                        out_path_vec.push(seg)
                    }
                }
            }

            if path_will_be_absolute {
                f.write_char('/')?;
            }

            for (n, seg) in out_path_vec.into_iter().enumerate() {
                if n != 0 {
                    f.write_char('/')?;
                }
                f.write_str(seg)?;
            }
        }

        if let Some(query) = components.query {
            f.write_char('?')?;
            f.write_str(query)?;
        }

        if let Some(fragment) = components.fragment {
            f.write_char('#')?;
            f.write_str(fragment)?;
        }

        Ok(())
    }

    /// Creates a new [`UriRefBuf`] that contains the result of performing URI resolution with
    /// `dest`.
    #[cfg(feature = "std")]
    fn resolved<T: AnyUriRef + ?Sized>(&self, dest: &T) -> Result<UriRefBuf, ResolveError> {
        if dest.is_empty() {
            return Ok(self.to_uri_ref_buf());
        }

        let mut ret = String::new();

        self.write_resolved(dest, &mut ret)?;

        // SAFETY: `write_resolved` is guaranteed to write well-formed UriRefs.
        Ok(unsafe { UriRefBuf::from_string_unchecked(ret) })
    }
}

/// Blanket implementation of `AnyUriRefExt` for all `AnyUriRef` instances.
impl<T: AnyUriRef + ?Sized> AnyUriRefExt for T {}

/// Blanket implementation for Copy-On-Write types.
impl<'a, T: AnyUriRef + Clone + ?Sized> AnyUriRef for Cow<'a, T> {
    fn components(&self) -> UriRawComponents<'_> {
        self.deref().components()
    }

    fn is_empty(&self) -> bool {
        self.deref().is_empty()
    }

    fn uri_type(&self) -> UriType {
        self.deref().uri_type()
    }

    #[cfg(feature = "std")]
    fn to_uri_ref_buf(&self) -> UriRefBuf {
        self.deref().to_uri_ref_buf()
    }

    unsafe fn write_to_unsafe<W: core::fmt::Write + ?Sized>(
        &self,
        write: &mut W,
    ) -> Result<(), core::fmt::Error> {
        self.deref().write_to_unsafe(write)
    }
}

/// Helper class to assist with using [`AnyUriRef`] with formatters; instantiated by
/// [`AnyUriRefExt::display`].
///
/// This type is similar to [`std::path::Display`].
#[derive(Debug, Copy, Clone)]
pub struct UriDisplay<'a, T: AnyUriRef + ?Sized>(&'a T);

impl<'a, T: AnyUriRef + ?Sized> Display for UriDisplay<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        self.0.write_to(f)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn resolve_simple() {
        let uri_test_table = vec![
            (
                "http://x/a/b/c",
                "/abs-path",
                Some(iuri_ref!("http://x/abs-path")),
            ),
            (
                "http://x/a/b/c",
                "f/s?c",
                Some(iuri_ref!("http://x/a/b/f/s?c")),
            ),
            (
                "http://x/a/b/c",
                "/abs-path",
                Some(iuri_ref!("http://x/abs-path")),
            ),
            (
                "http://x/a/b/c",
                "path",
                Some(iuri_ref!("http://x/a/b/path")),
            ),
            (
                "http://x/a/b/c/",
                "path",
                Some(iuri_ref!("http://x/a/b/c/path")),
            ),
            (
                "http://x/a/b/c/",
                "//y/d/e/f/",
                Some(iuri_ref!("http://y/d/e/f/")),
            ),
            ("http://x/a/b/c", "?", Some(iuri_ref!("http://x/a/b/c?"))),
            ("http://x", "a/b/c", Some(iuri_ref!("http://x/a/b/c"))),
            ("http://x", "/a/b/c", Some(iuri_ref!("http://x/a/b/c"))),
            ("http://x/", "a/b/c", Some(iuri_ref!("http://x/a/b/c"))),
            ("http://x/a/b/c", "coap://x", Some(iuri_ref!("coap://x"))),
        ];

        for (a, b, c) in uri_test_table {
            let uri_a = UriRef::from_str(a).expect(a);
            let uri_b = UriRef::from_str(b).expect(b);
            assert_eq!(
                uri_a.resolved(uri_b).ok(),
                c.map(|x| x.to_owned()),
                "uri_a.resolved(): a:{} b:{} c:{:?}",
                a,
                b,
                c
            );
        }
    }

    #[test]
    fn resolve_relative_base() {
        let uri_test_table = vec![
            ("b/c/d;p?q", "g:h", Some(iuri_ref!("g:h"))),
            ("b/c/d;p?q", "g", Some(iuri_ref!("b/c/g"))),
            ("b/c/d;p?q", "./g", Some(iuri_ref!("b/c/g"))),
            ("b/c/d;p?q", "g/", Some(iuri_ref!("b/c/g/"))),
            ("b/c/d;p?q", "/g", Some(iuri_ref!("/g"))),
            ("b/c/d;p?q", "//g", Some(iuri_ref!("//g"))),
            ("b/c/d;p?q", "?y", Some(iuri_ref!("b/c/d;p?y"))),
            ("b/c/d;p?q", "g?y", Some(iuri_ref!("b/c/g?y"))),
            ("b/c/d;p?q", "#s", Some(iuri_ref!("b/c/d;p?q#s"))),
            ("b/c/d;p?q", "g#s", Some(iuri_ref!("b/c/g#s"))),
            ("b/c/d;p?q", "g?y#s", Some(iuri_ref!("b/c/g?y#s"))),
            ("b/c/d;p?q", ";x", Some(iuri_ref!("b/c/;x"))),
            ("b/c/d;p?q", "g;x", Some(iuri_ref!("b/c/g;x"))),
            ("b/c/d;p?q", "g;x?y#s", Some(iuri_ref!("b/c/g;x?y#s"))),
            ("b/c/d;p?q", "", Some(iuri_ref!("b/c/d;p?q"))),
            ("b/c/d;p?q", ".", Some(iuri_ref!("b/c/"))),
            ("b/c/d;p?q", "./", Some(iuri_ref!("b/c/"))),
            ("b/c/d;p?q", "/./g", Some(iuri_ref!("/g"))),
            ("b/c/d;p?q", "g.", Some(iuri_ref!("b/c/g."))),
            ("b/c/d;p?q", ".g", Some(iuri_ref!("b/c/.g"))),
            ("b/c/d;p?q", "g..", Some(iuri_ref!("b/c/g.."))),
            ("b/c/d;p?q", "..g", Some(iuri_ref!("b/c/..g"))),
            ("b/c/d;p?q", "g?y/./x", Some(iuri_ref!("b/c/g?y/./x"))),
            ("b/c/d;p?q", "g?y/../x", Some(iuri_ref!("b/c/g?y/../x"))),
            ("b/c/d;p?q", "g#s/./x", Some(iuri_ref!("b/c/g#s/./x"))),
            ("b/c/d;p?q", "g#s/../x", Some(iuri_ref!("b/c/g#s/../x"))),
            ("b/c/d;p?q", "..", Some(iuri_ref!("b/"))),
            ("b/c/d;p?q", "../", Some(iuri_ref!("b/"))),
            ("b/c/d;p?q", "../g", Some(iuri_ref!("b/g"))),
            ("b/c/d;p?q", "../..", Some(iuri_ref!("."))),
            ("b/c/d;p?q", "../../", Some(iuri_ref!("."))),
            ("b/c/d;p?q", "../../g", Some(iuri_ref!("g"))),
            ("b/c/d;p?q", "../../../g", Some(iuri_ref!("../g"))),
            ("b/c/d;p?q", "../../../../g", Some(iuri_ref!("../../g"))),
            ("b/c/d;p?q", "/../g", Some(iuri_ref!("/g"))),
            ("b/c/d;p?q", "./../g", Some(iuri_ref!("b/g"))),
            ("b/c/d;p?q", "./g/.", Some(iuri_ref!("b/c/g/"))),
            ("b/c/d;p?q", "g/./h", Some(iuri_ref!("b/c/g/h"))),
            ("b/c/d;p?q", "g/../h", Some(iuri_ref!("b/c/h"))),
            ("b/c/d;p?q", "g;x=1/./y", Some(iuri_ref!("b/c/g;x=1/y"))),
            ("b/c/d;p?q", "g;x=1/../y", Some(iuri_ref!("b/c/y"))),
        ];

        for (a, b, c) in uri_test_table {
            let uri_a = UriRef::from_str(a).expect(a);
            let uri_b = UriRef::from_str(b).expect(b);
            assert_eq!(
                uri_a.resolved(uri_b).ok(),
                c.map(|x| x.to_owned()),
                "uri_a.resolved(): a:{} b:{} c:{:?}",
                a,
                b,
                c
            );
        }
    }

    #[test]
    fn resolve_cannot_be_a_base() {
        let uri_test_table = vec![
            ("s:123", "/a/b/c", None),
            ("s:123", "//a/b/c", None),
            ("s:123", ".", None),
            ("s:123", "", Some(iuri_ref!("s:123"))),
            ("s:123", "?q=123", Some(iuri_ref!("s:123?q=123"))),
            ("s:123", "#frag", Some(iuri_ref!("s:123#frag"))),
            ("s:123", "#frag", Some(iuri_ref!("s:123#frag"))),
            ("s:123", "file:/d/e/f", Some(iuri_ref!("file:/d/e/f"))),
        ];

        for (a, b, c) in uri_test_table {
            let uri_a = UriRef::from_str(a).expect(a);
            let uri_b = UriRef::from_str(b).expect(b);
            assert_eq!(
                uri_a.resolved(uri_b).ok(),
                c.map(|x| x.to_owned()),
                "uri_a.resolved(): a:{} b:{} c:{:?}",
                a,
                b,
                c
            );
        }
    }

    #[test]
    fn resolve_no_authority() {
        let uri_test_table = vec![
            ("file:/d/e/f", "//a/b/c", Some(iuri_ref!("file://a/b/c"))),
            ("file:/d/e/f", "g", Some(iuri_ref!("file:/d/e/g"))),
        ];

        for (a, b, c) in uri_test_table {
            let uri_a = UriRef::from_str(a).expect(a);
            let uri_b = UriRef::from_str(b).expect(b);
            assert_eq!(
                uri_a.resolved(uri_b).ok(),
                c.map(|x| x.to_owned()),
                "uri_a.resolved(): a:{} b:{} c:{:?}",
                a,
                b,
                c
            );
        }
    }

    #[test]
    fn resolve_rfc3986_simple() {
        let uri_test_table = vec![
            // The following test vectors came directly from RFC3986
            ("http://a/b/c/d;p?q", "g:h", Some(iuri_ref!("g:h"))),
            ("http://a/b/c/d;p?q", "g", Some(iuri_ref!("http://a/b/c/g"))),
            (
                "http://a/b/c/d;p?q",
                "./g",
                Some(iuri_ref!("http://a/b/c/g")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g/",
                Some(iuri_ref!("http://a/b/c/g/")),
            ),
            ("http://a/b/c/d;p?q", "/g", Some(iuri_ref!("http://a/g"))),
            ("http://a/b/c/d;p?q", "//g", Some(iuri_ref!("http://g"))),
            (
                "http://a/b/c/d;p?q",
                "?y",
                Some(iuri_ref!("http://a/b/c/d;p?y")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g?y",
                Some(iuri_ref!("http://a/b/c/g?y")),
            ),
            (
                "http://a/b/c/d;p?q",
                "#s",
                Some(iuri_ref!("http://a/b/c/d;p?q#s")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g#s",
                Some(iuri_ref!("http://a/b/c/g#s")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g?y#s",
                Some(iuri_ref!("http://a/b/c/g?y#s")),
            ),
            (
                "http://a/b/c/d;p?q",
                ";x",
                Some(iuri_ref!("http://a/b/c/;x")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g;x",
                Some(iuri_ref!("http://a/b/c/g;x")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g;x?y#s",
                Some(iuri_ref!("http://a/b/c/g;x?y#s")),
            ),
            (
                "http://a/b/c/d;p?q",
                "",
                Some(iuri_ref!("http://a/b/c/d;p?q")),
            ),
            ("http://a/b/c/d;p?q", ".", Some(iuri_ref!("http://a/b/c/"))),
            ("http://a/b/c/d;p?q", "./", Some(iuri_ref!("http://a/b/c/"))),
            ("http://a/b/c/d;p?q", "/./g", Some(iuri_ref!("http://a/g"))),
            (
                "http://a/b/c/d;p?q",
                "g.",
                Some(iuri_ref!("http://a/b/c/g.")),
            ),
            (
                "http://a/b/c/d;p?q",
                ".g",
                Some(iuri_ref!("http://a/b/c/.g")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g..",
                Some(iuri_ref!("http://a/b/c/g..")),
            ),
            (
                "http://a/b/c/d;p?q",
                "..g",
                Some(iuri_ref!("http://a/b/c/..g")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g?y/./x",
                Some(iuri_ref!("http://a/b/c/g?y/./x")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g?y/../x",
                Some(iuri_ref!("http://a/b/c/g?y/../x")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g#s/./x",
                Some(iuri_ref!("http://a/b/c/g#s/./x")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g#s/../x",
                Some(iuri_ref!("http://a/b/c/g#s/../x")),
            ),
        ];

        for (a, b, c) in uri_test_table {
            let uri_a = UriRef::from_str(a).expect(a);
            let uri_b = UriRef::from_str(b).expect(b);
            assert_eq!(
                uri_a.resolved(uri_b).ok(),
                c.map(|x| x.to_owned()),
                "uri_a.resolved(): a:{} b:{} c:{:?}",
                a,
                b,
                c
            );
        }
    }

    #[test]
    fn resolve_rfc3986_dot_dot() {
        let uri_test_table = vec![
            // The following test vectors came directly from RFC3986
            ("http://a/b/c/d;p?q", "..", Some(iuri_ref!("http://a/b/"))),
            ("http://a/b/c/d;p?q", "../", Some(iuri_ref!("http://a/b/"))),
            (
                "http://a/b/c/d;p?q",
                "../g",
                Some(iuri_ref!("http://a/b/g")),
            ),
            ("http://a/b/c/d;p?q", "../..", Some(iuri_ref!("http://a/"))),
            ("http://a/b/c/d;p?q", "../../", Some(iuri_ref!("http://a/"))),
            (
                "http://a/b/c/d;p?q",
                "../../g",
                Some(iuri_ref!("http://a/g")),
            ),
            (
                "http://a/b/c/d;p?q",
                "../../../g",
                Some(iuri_ref!("http://a/g")),
            ),
            (
                "http://a/b/c/d;p?q",
                "../../../../g",
                Some(iuri_ref!("http://a/g")),
            ),
            ("http://a/b/c/d;p?q", "/../g", Some(iuri_ref!("http://a/g"))),
            (
                "http://a/b/c/d;p?q",
                "./../g",
                Some(iuri_ref!("http://a/b/g")),
            ),
            (
                "http://a/b/c/d;p?q",
                "./g/.",
                Some(iuri_ref!("http://a/b/c/g/")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g/./h",
                Some(iuri_ref!("http://a/b/c/g/h")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g/../h",
                Some(iuri_ref!("http://a/b/c/h")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g;x=1/./y",
                Some(iuri_ref!("http://a/b/c/g;x=1/y")),
            ),
            (
                "http://a/b/c/d;p?q",
                "g;x=1/../y",
                Some(iuri_ref!("http://a/b/c/y")),
            ),
        ];

        for (a, b, c) in uri_test_table {
            let uri_a = UriRef::from_str(a).expect(a);
            let uri_b = UriRef::from_str(b).expect(b);
            assert_eq!(
                uri_a.resolved(uri_b).ok(),
                c.map(|x| x.to_owned()),
                "uri_a.resolved(): a:{} b:{} c:{:?}",
                a,
                b,
                c
            );
        }
    }
}
