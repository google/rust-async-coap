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

/// # **Experimental**: In-place unescaping iteration helper
///
/// This object does in-place unescaping, so it takes ownership of a mutable
/// [`RelRefBuf`] in order to prevent access to it while it is being mutated in-place for
/// unescaping. This approach allows you to avoid an extra memory allocation per path segment.
///
/// This object is created via [`RelRefBuf::into_unescape_buf`].
///
/// The first attempt at writing this class tried to have the iterator itself own the
/// buffer, but that doesn't actually work. Further explanation regarding why can be
/// found on Jordan MacDonald's excellent blog post on the topic: [Reference Iterators in Rust][ref-iter].
///
/// [ref-iter]: https://medium.com/@jordan_98525/reference-iterators-in-rust-5603a51b5192
///
/// ## Example
///
/// ```
/// use async_coap_uri::prelude::*;
/// let rel_ref_buf = rel_ref!(unsafe "g:a/%2F/bl%c3%a5b%c3%a6r?q=g:a&q=%26&q=syltet%c3%b8y").to_owned();
/// let mut unescape_buf = rel_ref_buf.into_unescape_buf();
///
/// let mut query_item_iter = unescape_buf.query_items();
///
/// assert_eq!(query_item_iter.next(), Some("q=g:a"));
/// assert_eq!(query_item_iter.next(), Some("q=&"));
/// assert_eq!(query_item_iter.next(), Some("q=syltetøy"));
/// assert_eq!(query_item_iter.next(), None);
///
/// core::mem::drop(query_item_iter);
///
/// let mut path_seg_iter = unescape_buf.path_segments();
///
/// assert_eq!(path_seg_iter.next(), Some("g:a"));
/// assert_eq!(path_seg_iter.next(), Some("/"));
/// assert_eq!(path_seg_iter.next(), Some("blåbær"));
/// assert_eq!(path_seg_iter.next(), None);
/// ```
#[doc(unstable)]
#[derive(Default, Clone, Debug)]
pub struct UriUnescapeBuf {
    buf: RelRefBuf,
    path_iter_did_fire: bool,
    query_iter_did_fire: bool,
}

impl UriUnescapeBuf {
    pub(super) fn new(buf: RelRefBuf) -> UriUnescapeBuf {
        UriUnescapeBuf {
            buf,
            path_iter_did_fire: false,
            query_iter_did_fire: false,
        }
    }

    /// Returns an iterator for iterating over all of the path segments.
    /// Currently, this can only be called once.
    pub fn path_segments<'a>(&'a mut self) -> impl Iterator<Item = &'a str> + 'a {
        let mut uri_rel: &mut RelRef = self.buf.as_mut_rel_ref();

        if !self.path_iter_did_fire {
            self.path_iter_did_fire = true;

            // TODO: Allow `UriUnescapeBuf::path_segments` and `UriUnescapeBuf::query_items` to
            //       be called in any order. Currently, `query_items()` must be called first or
            //       not at all---and that is enforced by this next line. This should be fixed.
            self.query_iter_did_fire = true;
        } else {
            debug_assert!(
                !self.path_iter_did_fire,
                "path_segments() can only be called once on UriUnescapeBuf"
            );

            // Here we are creating an empty, mutable &RelRef so that the iterator
            // will simply return `None`. This allows us to keep the same return
            // type without resorting to something uncouth like a boxed iterator.
            // The whole reason this is necessary is because we are mutating the underlying
            // data as we iterate, effectively corrupting it so that we can't rely on it
            // being parsable on subsequent readings (which would end up double-escaping, anyway).
            // This may change in the future.
            uri_rel = Default::default();
        }
        // SAFETY: This is safe because we guarantee with `did_fire` that we
        //         won't ever do this twice on the same data.
        unsafe { uri_rel.unsafe_path_segment_iter() }
    }

    /// Returns an iterator for iterating over all of the query items.
    /// Currently, this can only be called once.
    pub fn query_items<'a>(&'a mut self) -> impl Iterator<Item = &'a str> + 'a {
        let mut uri_rel: &mut RelRef = self.buf.as_mut_rel_ref();
        if !self.query_iter_did_fire {
            self.query_iter_did_fire = true;
        } else {
            debug_assert!(
                !self.query_iter_did_fire,
                "query_items() can only be called once on UriUnescapeBuf"
            );

            // Here we are creating an empty, mutable &RelRef so that the iterator
            // will simply return `None`. This allows us to keep the same return
            // type without resorting to something uncouth like a boxed iterator.
            // The whole reason this is necessary is because we are mutating the underlying
            // data as we iterate, effectively corrupting it so that we can't rely on it
            // being parsable on subsequent readings (which would end up double-escaping, anyway).
            // This may change in the future.
            uri_rel = Default::default();
        }

        // SAFETY: This is safe because we guarantee with `did_fire` that we
        //         won't ever do this twice on the same data.
        unsafe { uri_rel.unsafe_query_item_iter() }
    }
}
