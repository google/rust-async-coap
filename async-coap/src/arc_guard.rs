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

//! # Arc Guard
//!
//! This crate[^1] provides the [`ArcGuard`] class, which is a container for a single object
//! with lifetime that is bound to that of an `Arc`. This is useful for passing around boxed
//! futures that have a lifetime that is limited to that of the object that created it.
//!
//! For example, the following does not compile:
//!
//! ```compile_fail
//! # use async_coap::arc_guard; // Remove if spun off into own crate
//! use futures::{future::ready,future::BoxFuture,prelude::*};
//! use std::sync::{Arc,Weak};
//! use arc_guard::{ArcGuard,ArcGuardExt};
//!
//! trait PropertyFetcher {
//!         fn fetch(
//!             &self,
//!             key: &str,
//!         ) -> BoxFuture<Option<String>>;
//! }
//!
//! struct WeakFetcher {
//!     sub_obj: Weak<Box<PropertyFetcher>>,
//! }
//!
//! impl PropertyFetcher for WeakFetcher {
//!     fn fetch(&self, key: &str) -> BoxFuture<Option<String>> {
//!         if let Some(arc) = self.sub_obj.upgrade() {
//!             // error[E0515]: cannot return value referencing local variable `arc`
//!             arc.fetch(key).boxed()
//!         } else {
//!             ready(None).boxed()
//!         }
//!     }
//! }
//! ```
//!
//! If you think about it, the fact that `rustc` doesn't like this code makes perfect sense:
//! because `sub_obj` is a weak reference, it could be dropped at any moment, violating the
//! lifetime guarantee for the return value of `fetch()`. To fix this, we need to ensure that
//! the value we return internally keeps an `Arc` reference to the object that created it. That's
//! where `ArcGuard` comes in:
//!
//! ```
//! # use async_coap::arc_guard; // Remove if spun off into own crate
//! # use futures::{future::ready,future::BoxFuture,prelude::*};
//! # use std::sync::{Arc,Weak};
//! # use arc_guard::{ArcGuard,ArcGuardExt};
//! #
//! # trait PropertyFetcher {
//! #         fn fetch(
//! #             &self,
//! #             key: &str,
//! #         ) -> BoxFuture<Option<String>>;
//! # }
//! #
//! # struct WeakFetcher {
//! #     sub_obj: Weak<Box<PropertyFetcher>>,
//! # }
//!
//! impl PropertyFetcher for WeakFetcher {
//!     fn fetch(&self, key: &str) -> BoxFuture<Option<String>> {
//!         if let Some(arc) = self.sub_obj.upgrade() {
//!             // Compiles and works!
//!             arc.guard(|x|x.fetch(key)).boxed()
//!         } else {
//!             ready(None).boxed()
//!         }
//!     }
//! }
//! ```
//!
//! ## Additional Examples
//!
//! ```
//! # use async_coap::arc_guard; // Remove if spun off into own crate
//! # use std::sync::{Arc,Weak};
//! # use arc_guard::{ArcGuard,ArcGuardExt};
//!
//! let mut arc = Arc::new("foobar".to_string());
//!
//! let guarded = arc.guard(|s| &s.as_str()[3..]);
//!
//! assert_eq!(guarded, "bar");
//!
//! // We can't get a mutable instance to the
//! // string while `guarded` is still around.
//! assert_eq!(Arc::get_mut(&mut arc), None);
//!
//! core::mem::drop(guarded);
//!
//! assert!(Arc::get_mut(&mut arc).is_some());
//! ```
//!
//! [^1]: I would have loved to call this crate `lifeguard`, because it is a "guard" on the
//!       lifetime of the contained "head" instance, but sadly that name was
//!       [already taken](https://crates.io/crates/lifeguard).
//!

#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![warn(clippy::all)]

use futures::prelude::*;
use pin_utils::unsafe_pinned;
use std::cmp::Ordering;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;

/// A container for a single object with lifetime that is bound to that of an `Arc`.
///
/// See [Module Documentation](index.html) for more information.
#[derive(Debug, Clone)]
pub struct ArcGuard<RC, T> {
    inner: T,
    head: Arc<RC>,
}

impl<RC, T> ArcGuard<RC, T> {
    unsafe_pinned!(inner: T);

    /// Constructs a new `ArcGuard<>` instance using the given `Arc<>` and getter closure.
    /// The use of a closure for the getter allows for a more convenient syntax while ensuring
    /// the lifetimes are properly accounted for.
    ///
    /// See the main documentation for `ArcGuard<>` for a usage example.
    pub fn new<'head, F>(head: Arc<RC>, getter: F) -> ArcGuard<RC, T>
    where
        F: FnOnce(&'head RC) -> T,
        RC: 'head,
        T: 'head,
    {
        // SAFETY: This is safe because we are only using this reference to create our object,
        // and, by holding a reference to `head`, this class ensures that it does not live longer
        // than the contained reference.
        ArcGuard {
            inner: getter(unsafe { std::mem::transmute::<&RC, &RC>(&head) }),
            head,
        }
    }

    /// Borrows a reference to the `Arc` that is being held to preserve the underlying value.
    pub fn head(&self) -> &Arc<RC> {
        &self.head
    }
}

unsafe impl<RC, T: Send> Send for ArcGuard<RC, T> {}
unsafe impl<RC, T: Sync> Sync for ArcGuard<RC, T> {}

impl<RC, T> Deref for ArcGuard<RC, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<RC, T: std::fmt::Display> std::fmt::Display for ArcGuard<RC, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.inner.fmt(f)
    }
}

impl<RC, T: AsRef<R>, R> AsRef<R> for ArcGuard<RC, T> {
    fn as_ref(&self) -> &R {
        self.inner.as_ref()
    }
}

impl<RC, T> std::borrow::Borrow<T> for ArcGuard<RC, T> {
    fn borrow(&self) -> &T {
        &self.inner
    }
}

impl<RC, T: PartialEq<R>, R> PartialEq<R> for ArcGuard<RC, T> {
    fn eq(&self, other: &R) -> bool {
        self.inner.eq(other)
    }
}

impl<RC, T: PartialOrd<R>, R> PartialOrd<R> for ArcGuard<RC, T> {
    fn partial_cmp(&self, other: &R) -> Option<Ordering> {
        self.inner.partial_cmp(other)
    }
}

impl<RC, T: Future> Future for ArcGuard<RC, T> {
    type Output = T::Output;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut futures::task::Context<'_>,
    ) -> futures::task::Poll<Self::Output> {
        self.as_mut().inner().poll(cx)
    }
}

impl<RC, T: Stream> Stream for ArcGuard<RC, T> {
    type Item = T::Item;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut futures::task::Context<'_>,
    ) -> futures::task::Poll<Option<Self::Item>> {
        self.as_mut().inner().poll_next(cx)
    }
}

/// A convenience trait for `Arc<>` that makes it easier to construct `ArcGuard<>` instances.
///
/// See [Module Documentation](index.html) for more information.
pub trait ArcGuardExt<RC> {
    /// Convenience method for constructing `ArcGuard<>` instances.
    ///
    /// See [Module Documentation](index.html) for more information.
    fn guard<'head, F, T>(&self, getter: F) -> ArcGuard<RC, T>
    where
        F: FnOnce(&'head RC) -> T,
        RC: 'head,
        T: 'head;
}

impl<RC> ArcGuardExt<RC> for Arc<RC> {
    fn guard<'head, F, T>(&self, getter: F) -> ArcGuard<RC, T>
    where
        F: FnOnce(&'head RC) -> T,
        RC: 'head,
        T: 'head,
    {
        ArcGuard::new(self.clone(), getter)
    }
}
