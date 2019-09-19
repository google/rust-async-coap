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

//! # Safe, In-place URI Abstraction
//!
//! This crate provides safe, efficient, full-featured support for using and manipulating
//! [Uniform Resource Identifiers][IETF-RFC3986].
//!
//! What makes this crate unique is that it provides URI-specific types that have the same
//! unsized/sized[^1] type duality that is used for [`&str`]/[`String`], except with specific
//! guarantees on the content, as well as convenient domain-specific methods to access the
//! URI components. The API was designed to be easy-to-use while making as few
//! heap allocations as possible. Most common operations require no allocations at all,
//! and those that do are provided as a convenience rather than a fundamental requirement.
//! For example, you can parse and fully percent-decode a URI without doing a single allocation.
//!
//! Similar to how you can specify a [`&'static str`](str) inline as a *string literal*,
//! you can specify in-line "URI literals" that are checked for well-formedness at compile
//! time.
//!
//! ## Important Types
//!
//! This crate provides three fundamental types named after their [IETF-RFC3986] counterparts:
//!
//! [**URI-references**][URI-reference] are contained in the unsized string slice
//! subtype [`&UriRef`] ,with [`UriRefBuf`] being the sized, heap-allocated version. This
//! is the most flexible and commonly used type, since it can contain either a [URI]
//! (like "`http://example.com/`") or a [relative-reference] (Like "`/a/b/c?q=foo`").
//! URI-reference literals for this type can be created using the [`uri_ref!`] macro.
//!
//! Actual full [**URIs**][URI] (like "`http://example.com/`") can be contained in the unsized
//! string slice subtype [`&Uri`] ,with [`UriBuf`] being the sized, heap-allocated version.
//! This type is less flexible than [`UriRef`] because it cannot hold a [relative-reference]:
//! if you have a `&Uri`, you are guaranteed that it does not contain a [relative-reference].
//! URI literals for this type can be created using the [`uri!`] macro.
//!
//! [**Relative-references**][relative-reference] (Like "`/a/b/c?q=foo`") are contained in the
//! unsized string slice
//! subtype [`&RelRef`] ,with [`RelRefBuf`] being the sized, heap-allocated version.
//! This type is less flexible than [`UriRef`] because it cannot hold a full [URI].
//! If you have a `&RelRef`, you are guaranteed that it can only contain a *path*, *query*,
//! and/or *fragment*.
//! Relative-reference literals for this type can be created using the [`rel_ref!`] macro.
//!
//! Each type above provides methods for accessing the individual URI components in both
//! raw and percent-decoded form. Additionally, they also provide iterator accessors for
//! parsing path segments and query items—again both in raw and percent-decoded forms.
//!
//! In some cases it can be more efficient to pre-compute the offsets  of all of the URI components
//! rather than recalculate them individually, as the methods on the above types do.
//! For such cases, [`UriRawComponents`] pre-computes each component of the URI internally,
//! allowing for more efficient repeated access. The type uses no memory allocations and is
//! scoped to the lifetime of the type that was used to create it.
//!
//! A common trait—[`AnyUriRef`]—for all of these types (Including [`UriRawComponents`]) is
//! provided to make usage in generic contexts easier by allowing you to pass a borrowed reference
//! to of the above types as an argument.
//!
//! ## Network Path Support
//!
//! This crate aims for complete [IETF-RFC3986] compliance while still being fast and efficient,
//! but there is one part where it deviates very slightly: *network paths*.
//!
//! A network path is essentially a full URI without a **scheme**, *but with an* **authority**.
//! For example, `//example.com/a/b/c?q=123#body` is a *network path*.
//!
//! According to [IETF-RFC3986] section 4.2, network paths are *relative-references*.
//! However, this crate considers them to belong to
//! [`&Uri`]/[`UriBuf`], not [`&RelRef`]/[`RelRefBuf`] as IETF-RFC3986 would imply.
//! This was done to simplify typical usage patterns by guaranteeing that a
//! [`&RelRef`]/[`RelRefBuf`] will never have a scheme or an authority component.
//!
//! ## Casting and Deref
//!
//! [`UriRef`] implements [`Deref<Target=str>`], allowing you to use all of the
//! non-mutating methods from [`str`], like [`len()`].  as well as create new string slices using
//! the `[begin..end]` syntax. A [`&UriRef`] can be cast to a [`&str`] for free via the method
//! [`UriRef.as_str()`].
//!
//! [`Uri`] implements [`Deref<Target=UriRef>`], allowing you to use a [`&Uri`]
//! anywhere a [`&UriRef`] is called for, and since [`UriRef`] implements [`Deref<Target=str>`],
//! you can also use all of the [`str`] methods, too. A [`&Uri`] can be cast to a [`&UriRef`] for
//! free via the method [`Uri.as_uri_ref()`], and likewise to a [`&str`] via the method [`Uri.as_str()`].
//!
//! You might think that [`RelRef`] would implement [`Deref<Target=UriRef>`], too, but this actually
//! isn't safe. So while there is a [`RelRef.as_uri_ref()`], it returns a `Cow<UriRef>` instead of
//! a [`&UriRef`]. For more information, see [this section](struct.RelRef.html#relref-and-deref).
//!
//! ## URI "Literals"
//!
//! For cases where you need a URI "literal", you can use the [`uri_ref!`], [`rel_ref!`],
//! and/or [`uri!`] macros:
//!
//! ```
//! use async_coap_uri::prelude::*;
//!
//! let uri: &Uri = uri!("http://example.com/foo/bar/");
//! let (abs_part, rel_part) = uri.split();
//!
//! assert_eq!(uri!("http://example.com"), abs_part);
//! assert_eq!(rel_ref!("/foo/bar/"), rel_part);
//! ```
//!
//! These "literals" are checked for correctness at compile time:
//!
//! ```compile_fail
//! # use async_coap_uri::prelude::*;
//! // This will not compile.
//! let x = uri!("%00 invalid %ff");
//! ```
//!
//! [^1]: This unsized/sized pattern is useful because there are often cases where you would want to have
//! a method or function take a URI as an argument. If you just passed a `&str` or a [`String`],
//! you would need to verify that the URI was well-formed each time the method or function was
//! called. You could fix this by creating a wrapper struct (something like `UriRef(String)`,
//! which is similar to  [rust-url](https://docs.rs/url/2.0.0/src/url/lib.rs.html#162) does it),
//! but this requires the use of alloc and is inefficient in many cases, so this crate uses the
//! unsized/sized pattern.
//!
//! [IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
//! [URI-reference]: https://tools.ietf.org/html/rfc3986#section-4.1
//! [relative-reference]: https://tools.ietf.org/html/rfc3986#section-4.2
//! [URI]: https://tools.ietf.org/html/rfc3986#section-3
//! [`UriRef.as_str()`]: struct.UriRef.html#method.as_str
//! [`RelRef.as_uri_ref()`]: struct.RelRef.html#method.as_uri
//! [`Uri.as_uri_ref()`]: struct.Uri.html#method.as_uri
//! [`Uri.as_str()`]: struct.Uri.html#method.as_str
//! [`&Uri`]: Uri
//! [`&RelRef`]: RelRef
//! [`&UriRef`]: UriRef
//! [`&str`]: str
//! [`Deref<Target=str>`]: core::ops::Deref
//! [`Deref<Target=UriRef>`]: core::ops::Deref
//! [`len()`]: https://doc.rust-lang.org/nightly/std/primitive.str.html#method.len
//! [`&Path`]: std::path::Path
//! [`PathBuf`]: std::path::PathBuf
//! [`&OsStr`]: std::ffi::OsStr
//! [`OsString`]: std::ffi::OsString
//!

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![warn(clippy::all)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
#[macro_use]
extern crate lazy_static;

pub mod escape;
use escape::*;

mod uri_ref;
pub use uri_ref::UriRef;

#[cfg(feature = "std")]
mod uri_ref_buf;
#[cfg(feature = "std")]
pub use uri_ref_buf::UriRefBuf;

mod uri_raw_components;
pub use uri_raw_components::UriRawComponents;

mod rel_ref;
pub use rel_ref::RelRef;

mod uri;
pub use uri::Uri;

mod uri_type;
pub use uri_type::UriType;

mod any_uri_ref;
pub use any_uri_ref::AnyUriRef;
pub use any_uri_ref::AnyUriRefExt;
pub use any_uri_ref::UriDisplay;

mod error;
pub use error::{ParseError, ResolveError};

#[cfg(feature = "std")]
mod rel_ref_buf;
#[cfg(feature = "std")]
pub use rel_ref_buf::RelRefBuf;

#[cfg(feature = "std")]
mod uri_buf;
#[cfg(feature = "std")]
pub use uri_buf::UriBuf;

#[cfg(feature = "std")]
mod uri_unescape_buf;
#[cfg(feature = "std")]
pub use uri_unescape_buf::UriUnescapeBuf;

#[cfg(feature = "std")]
mod regexes;
#[cfg(feature = "std")]
pub(crate) use regexes::*;

#[cfg(test)]
mod test;

#[doc(hidden)]
pub mod macros;

#[cfg(feature = "std")]
use std::borrow::Cow;

/// Convenience type for `Cow<'a, Uri>`.
#[cfg(feature = "std")]
pub type UriCow<'a> = Cow<'a, Uri>;

/// Convenience type for `Cow<'a, UriRef>`.
#[cfg(feature = "std")]
pub type UriRefCow<'a> = Cow<'a, UriRef>;

/// Convenience type for `Cow<'a, RelRef>`.
#[cfg(feature = "std")]
pub type RelRefCow<'a> = Cow<'a, RelRef>;

use proc_macro_hack::proc_macro_hack;

/// Used by the `uri` macro to verify correctness at compile-time.
#[doc(hidden)]
#[proc_macro_hack]
pub use async_coap_uri_macros::assert_uri_literal;

/// Used by the `uri_ref` macro to verify correctness at compile-time.
#[doc(hidden)]
#[proc_macro_hack]
pub use async_coap_uri_macros::assert_uri_ref_literal;

/// Used by the `rel_ref` macro to verify correctness at compile-time.
#[doc(hidden)]
#[proc_macro_hack]
pub use async_coap_uri_macros::assert_rel_ref_literal;

#[doc(hidden)]
pub mod prelude {
    pub use super::escape::StrExt;
    pub use super::UriRawComponents;
    pub use super::{rel_ref, uri, uri_ref};
    pub use super::{AnyUriRef, AnyUriRefExt};
    pub use super::{RelRef, Uri, UriRef};

    pub use {assert_rel_ref_literal, assert_uri_literal, assert_uri_ref_literal};

    #[cfg(feature = "std")]
    pub use super::{RelRefBuf, UriBuf, UriRefBuf};

    #[cfg(feature = "std")]
    pub use super::{RelRefCow, UriCow, UriRefCow};

    #[cfg(feature = "std")]
    pub use super::{rel_ref_format, uri_format, uri_ref_format};
}
