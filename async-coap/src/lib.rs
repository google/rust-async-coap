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

//! An experimental, asynchronous implementation of the Constrained Application Protocol (CoAP).
//!
//! This library provides a flexible, [asynchronous](https://rust-lang-nursery.github.io/futures-rs/)
//! interface for using and serving CoAP resources. You can either use the [included datagram-based
//! back-end](datagram) or you can write your own back-end by implementing [`LocalEndpoint`].
//!
//! By implementing [`datagram::AsyncDatagramSocket`], you can use the [provided datagram-based
//! back-end](datagram) with whatever datagram-based network layer you might want, be it UDP,
//! DTLS, or even SMS. A [Tokio](https://tokio.rs)-based `UdpSocket` implementation can be found
//! [here](https://docs.rs/async-coap-tokio)[^AllowStdUdpSocket].
//!
//! [^AllowStdUdpSocket]: A naive wrapper around Rust's standard [`std::net::UdpSocket`]
//! ([`datagram::AllowStdUdpSocket`]) is included in this crate, but it should usually be avoided
//! in favor of better-performing options, like [`async-coap-tokio::TokioAsyncUdpSocket`].
//!
//! ## Design
//!
//! Async-coap works differently than other CoAP libraries, making heavy use of combinators and
//! [Futures v0.3].
//!
//! [Futures v0.3]: https://rust-lang-nursery.github.io/futures-api-docs/0.3.0-alpha.18/futures/
//!
//! ### Simple Unicast
//!
//! For the most part, CoAP was designed for the typical [RESTful] paradigm of sending requests and
//! receiving responses.
//! In typical CoAP libraries, you have a message type that you would create, populate with your request,
//! and then pass to a method to send that request somewhere. Once the response had been received, the result
//! would be returned as a single message. Simple. Straightforward.
//!
//! [RESTful]: https://en.wikipedia.org/wiki/Representational_state_transfer
//!
//! This similarly straightforward to do with *async-coap*:
//!
//! ```
//! # #![feature(async_await)]
//! # use std::sync::Arc;
//! # use futures::{prelude::*,executor::LocalPool,task::LocalSpawnExt};
//! # use async_coap::prelude::*;
//! # use async_coap::datagram::{DatagramLocalEndpoint, AllowStdUdpSocket, LoopbackSocket};
//! # use async_coap::null::NullLocalEndpoint;
//! # let socket = AllowStdUdpSocket::bind("[::]:0").expect("UDP bind failed");
//! # let local_endpoint = Arc::new(DatagramLocalEndpoint::new(socket));
//! # let mut pool = LocalPool::new();
//! # pool.spawner().spawn_local(local_endpoint.clone().receive_loop_arc(null_receiver!()).map(|_|unreachable!()));
//! # let future = async move {
//! // Create a `remote_endpoint` instance representing the destination
//! // identified by the URI "coap://coap.me/test". This tends to be
//! // more convenient than using `local_endpoint` directly.
//! let mut remote_endpoint = local_endpoint
//!     .remote_endpoint_from_uri(uri!("coap://coap.me/test"))
//!     .expect("Remote endpoint lookup failed");
//!
//! // Create a future representing for our request.
//! let future = remote_endpoint.send(CoapRequest::get().emit_any_response());
//!
//! // Send the request and await the response.
//! let response = future.await.expect("CoAP request failed");
//!
//! // Print out the response message to standard output.
//! println!("Got response: {}", response);
//! # };
//! # pool.run_until(future);
//! ```
//!
//! ### Block2 Reconstruction
//!
//! However, there are cases where a single request can result in multiple
//! responses (i.e. [Multicast] and [Observing]), as well as cases where a single "logical"
//! request/response can be spread out across many smaller requests and responses
//! (i.e. [Block transfer]).
//!
//! [Multicast]: https://tools.ietf.org/html/rfc7252#section-8
//! [Observing]: https://tools.ietf.org/html/rfc7641
//! [Block transfer]: https://tools.ietf.org/html/rfc7959
//!
//! Let's take Block2 transfers, for example. Many libraries support Block2 transfers,
//! often by implementing message reconstruction under-the-hood, which can be very
//! convenient. The *async-coap* way to do it is similarly convenient:
//!
//! ```
//! # #![feature(async_await)]
//! # use std::sync::Arc;
//! # use futures::{prelude::*,executor::LocalPool,task::LocalSpawnExt};
//! # use async_coap::prelude::*;
//! # use async_coap::datagram::{DatagramLocalEndpoint, AllowStdUdpSocket, LoopbackSocket};
//! # use async_coap::null::NullLocalEndpoint;
//! # let socket = AllowStdUdpSocket::bind("[::]:0").expect("UDP bind failed");
//! # let local_endpoint = Arc::new(DatagramLocalEndpoint::new(socket));
//! # let mut pool = LocalPool::new();
//! # pool.spawner().spawn_local(local_endpoint.clone().receive_loop_arc(null_receiver!()).map(|_|unreachable!()));
//! # let future = async move {
//! # let mut remote_endpoint = local_endpoint
//! #    .remote_endpoint_from_uri(uri!("coap://coap.me/test"))
//! #    .expect("Remote endpoint lookup failed");
//! // We can change the path on the above remote_endpoint using
//! // the `clone_using_rel_ref` method:
//! let mut remote_endpoint = remote_endpoint.clone_using_rel_ref(rel_ref!("/large"));
//!
//! // Create a send descriptor that will reconstruct the block2 parts
//! // and return the reconstituted message.
//! let send_descriptor = CoapRequest::get()
//!     .block2(None)
//!     .emit_successful_collected_response();
//!
//! let response = remote_endpoint
//!     .send(send_descriptor)
//!     .await
//!     .expect("CoAP request failed");
//!
//! // Print out the response message to standard output.
//! println!("Reconstructed response: {}", response);
//! # };
//! # pool.run_until(future);
//! ```
//!
//! ### Inspection
//!
//! The problem with how this is implemented by other CoAP libraries is that it is difficult
//! or impossible to implement things like progress meters. However, with *async-coap*, we
//! can add some feedback to the above example very easily using [`inspect`]:
//!
//! [`inspect`]: send_desc::SendDescExt::inspect
//!
//! ```
//! # #![feature(async_await)]
//! # use std::sync::Arc;
//! # use futures::{prelude::*,executor::LocalPool,task::LocalSpawnExt};
//! # use async_coap::prelude::*;
//! # use async_coap::datagram::{DatagramLocalEndpoint, AllowStdUdpSocket, LoopbackSocket};
//! # use async_coap::null::NullLocalEndpoint;
//! # use async_coap::message::MessageDisplay;
//! # let socket = AllowStdUdpSocket::bind("[::]:0").expect("UDP bind failed");
//! # let local_endpoint = Arc::new(DatagramLocalEndpoint::new(socket));
//! # let mut pool = LocalPool::new();
//! # pool.spawner().spawn_local(local_endpoint.clone().receive_loop_arc(null_receiver!()).map(|_|unreachable!()));
//! # let future = async move {
//! # let mut remote_endpoint = local_endpoint
//! #    .remote_endpoint_from_uri(uri!("coap://coap.me/test"))
//! #    .expect("Remote endpoint lookup failed");
//! # let mut remote_endpoint = remote_endpoint.clone_using_rel_ref(rel_ref!("/large"));
//! // Create a send descriptor that will reconstruct the block2 parts
//! // and return the reconstituted message, printing out each individual
//! // message as we go.
//! let send_descriptor = CoapRequest::get()
//!     .block2(None)
//!     .emit_successful_collected_response()
//!     .inspect(|context| {
//!         println!("inspect: Got {}", MessageDisplay(context.message()));
//!     });
//!
//! let response = remote_endpoint
//!     .send(send_descriptor)
//!     .await
//!     .expect("CoAP request failed");
//!
//! // Print out the response message to standard output.
//! println!("Reconstructed response: {}", response);
//! # };
//! # pool.run_until(future);
//! ```
//!
//! ### Multiple Responses
//!
//! That's all good and well, but what about requests that generate multiple responses, like
//! multicast requests? For that we use a different
//! send method: [`send_as_stream`]. Instead of returning a [`Future`], it returns a [`Stream`].
//! This allows us to collect all of the responses:
//!
//! ```no_run
//! # #![feature(async_await)]
//! # use std::sync::Arc;
//! # use futures::{prelude::*,executor::LocalPool,task::LocalSpawnExt};
//! # use async_coap::prelude::*;
//! # use async_coap::datagram::{DatagramLocalEndpoint, AllowStdUdpSocket, LoopbackSocket};
//! # use async_coap::null::NullLocalEndpoint;
//! # use async_coap::message::MessageDisplay;
//! # use async_coap::Error;
//! # use futures_timer::TryFutureExt;
//! # use std::time::Duration;
//! # let socket = AllowStdUdpSocket::bind("[::]:0").expect("UDP bind failed");
//! # let local_endpoint = Arc::new(DatagramLocalEndpoint::new(socket));
//! # let mut pool = LocalPool::new();
//! # pool.spawner().spawn_local(local_endpoint.clone().receive_loop_arc(null_receiver!()).map(|_|unreachable!()));
//! # let future = async move {
//! let mut remote_endpoint = local_endpoint
//!     .remote_endpoint_from_uri(uri!("coap://[FF02::FD]/.well-known/core"))
//!     .expect("Remote endpoint lookup failed");
//!
//! // Don't let the remote_endpoint include
//! // a `Uri-Host` host option.
//! remote_endpoint.remove_host_option();
//!
//! let send_descriptor = CoapRequest::get()
//!     .multicast()
//!     .accept(ContentFormat::APPLICATION_LINK_FORMAT)
//!     .emit_successful_response()
//!     .include_socket_addr();
//!
//! let mut stream = remote_endpoint.send_as_stream(send_descriptor);
//!
//! while let Some((msg, socket_addr))
//!     = stream.next().await.transpose().expect("Error on get")
//! {
//!     println!("From {} got {}", socket_addr, msg);
//! }
//! # };
//! # pool.run_until(future);
//! ```
//!
//! [`send_as_stream`]: RemoteEndpointExt::send_as_stream
//! [`Future`]: std::future::Future
//! [`Stream`]: futures-preview::stream::Stream
//!
//! ## Future Work
//!
//! This library is currently in the experimental stage, so there are a lot of additional features
//! and mechanisms that aren't yet implemented. Here is a short list:
//!
//! * Support for "effortless" serving of [observable resources][Observing]
//! * Support for [Block1][Block transfer] transfers.
//! * Improved support for [observing][Observing] remote resources.
//! * Make serving resources easier-to-use.
//! * [OSCORE](https://tools.ietf.org/html/draft-ietf-core-object-security) support.
//! * Support for supplying alternate [transmission parameters](https://tools.ietf.org/html/rfc7252#section-4.8).
//! * Support for burst transmissions for nonconfirmable and multicast requests.
//! * Make sending asynchronous responses easier.
//!
//! ### Support for deeply embedded devices
//!
//! To the extent possible, the API is designed
//! to minimize the amount of memory allocation. While it does currently require the `alloc` crate,
//! that requirement will (hopefully) become optional once the [Generic Associated Types][GAT]
//! feature lands, without significantly influencing how the API works. This will allow for the
//! same API to be used for deeply embedded, resource-constrained devices as would be used for
//! other types of non-resource-constrained devices.
//!
//! [GAT]: https://github.com/rust-lang/rust/issues/44265
//! [`AllowStdUdpSocket`]: crate::datagram::AllowStdUdpSocket
//!
//! ## Full Example
//!
//! ```
//! # #![feature(async_await)]
//! #
//! use std::sync::Arc;
//! use futures::{prelude::*,executor::LocalPool,task::LocalSpawnExt};
//! use async_coap::prelude::*;
//! use async_coap::datagram::{DatagramLocalEndpoint,AllowStdUdpSocket};
//!
//! // Create our asynchronous socket. In this case, it is just an
//! // (inefficient) wrapper around the standard rust `UdpSocket`,
//! // but that is quite adequate in this case.
//! let socket = AllowStdUdpSocket::bind("[::]:0").expect("UDP bind failed");
//!
//! // Create a new local endpoint from the socket we just created,
//! // wrapping it in a `Arc<>` to ensure it can live long enough.
//! let local_endpoint = Arc::new(DatagramLocalEndpoint::new(socket));
//!
//! // Create a local execution pool for running our local endpoint.
//! let mut pool = LocalPool::new();
//!
//! // Quick aside: The `Local` in `LocalPool` is completely unrelated
//! // to the `Local` in `LocalEndpoint`: a `LocalEndpoint` refers to
//! // the local side of a CoAP connection. A `LocalPool` is just a
//! // single-threaded execution pool. A `LocalEndpoint` will run
//! // just fine on a `ThreadedPool`.
//!
//! // Add our local endpoint to the pool, so that it
//! // can receive packets.
//! pool.spawner().spawn_local(local_endpoint
//!     .clone()
//!     .receive_loop_arc(null_receiver!())
//!     .map(|err| panic!("Receive loop terminated: {}", err))
//! );
//!
//! // Create a remote endpoint instance to represent the
//! // device we wish to interact with.
//! let remote_endpoint = local_endpoint
//!     .remote_endpoint_from_uri(uri!("coap://coap.me"))
//!     .unwrap(); // Will only fail if the URI scheme or authority is unrecognizable
//!
//! // Create a future that sends a request to a specific path
//! // on the remote endpoint, collecting any blocks in the response
//! // and returning `Ok(OwnedImmutableMessage)` upon success.
//! let future_result = remote_endpoint.send_to(
//!     rel_ref!("large"),
//!     CoapRequest::get()                          // This is a CoAP GET request
//!         .accept(ContentFormat::TEXT_PLAIN_UTF8) // We only want plaintext
//!         .block2(Some(Default::default()))       // Enable block2 processing
//!         .emit_successful_collected_response()   // Collect all blocks into a single message
//! );
//!
//! // Wait until we get the result of our request.
//! let result = pool.run_until(future_result);
//!
//! println!("result: {:?}", result);
//! ```
//!
//! Additional examples can be found in the [module documentation for send descriptors][send-desc]
//! and the [documentation for `LocalEndpoint`][LocalEndpoint].
//!
//! [send-desc]: send_desc/index.html

#![feature(async_await)]
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(rust_2018_idioms)]
#![warn(missing_debug_implementations)]
#![warn(clippy::all)]
#![warn(missing_docs)]

#[macro_use]
extern crate log;

pub mod arc_guard;
use arc_guard::*;

#[doc(hidden)]
pub use async_coap_uri;

pub mod uri {
    //! A limited subset of items from the URI-handling [`async-coap-uri`] crate.
    //!
    //! See the [`async-coap-uri` crate documentation][`async-coap-uri`] for more details.
    //!
    //! [`async-coap-uri`]: ../async_coap_uri/index.html
    pub use async_coap_uri::escape;

    pub use async_coap_uri::{rel_ref, uri, uri_ref};
    pub use async_coap_uri::{RelRef, Uri, UriRef};
    pub use async_coap_uri::{RelRefBuf, UriBuf, UriRefBuf};

    pub use async_coap_uri::{AnyUriRef, UriDisplay, UriType};

    pub use async_coap_uri::{ParseError, ResolveError};

    pub use async_coap_uri::UriRawComponents;

    #[doc(hidden)]
    pub(super) use async_coap_uri::prelude;

    #[doc(hidden)]
    pub use async_coap_uri::{assert_rel_ref_literal, assert_uri_literal, assert_uri_ref_literal};
}

pub mod message;
pub mod option;

pub mod send_desc;
use send_desc::*;

mod response_status;
pub use response_status::ResponseStatus;

mod content_format;
pub use content_format::ContentFormat;

mod socketaddr;
pub use socketaddr::SocketAddrExt;
pub use socketaddr::ToSocketAddrs;

mod block;
pub use block::*;

mod trans_params;
pub use trans_params::*;

mod local_endpoint;
pub use local_endpoint::*;

mod remote_endpoint;
pub use remote_endpoint::*;

mod send_as_stream;
pub use send_as_stream::*;

mod receive_as_stream;
pub use receive_as_stream::*;

mod inbound_context;
pub use inbound_context::*;

pub mod consts;
#[doc(hidden)]
pub use consts::*;

mod error;
pub use error::*;

mod util;
use util::*;

pub mod link_format;
#[doc(hidden)]
pub use link_format::*;

pub mod datagram;
pub mod null;

mod etag;
pub use etag::ETag;

use futures::future::BoxFuture;
use message::MessageRead;
use message::MessageWrite;

#[doc(hidden)]
pub mod prelude {
    pub use super::uri::prelude::*;

    pub use super::LocalEndpoint;
    pub use super::LocalEndpointExt;

    pub use super::null_receiver;

    pub use super::RemoteEndpoint;
    pub use super::RemoteEndpointExt;

    pub use super::send_desc::CoapRequest;
    pub use super::send_desc::SendDescExt;
    pub use super::send_desc::SendDescMulticast;
    pub use super::send_desc::SendDescUnicast;

    pub use super::ContentFormat;
    pub use super::ResponseStatus;

    pub use super::message::MsgCode;
    pub use super::message::MsgCodeClass;
    pub use super::message::MsgId;
    pub use super::message::MsgToken;
    pub use super::message::MsgType;

    pub use super::option;
    pub use option::OptionInsert;
    pub use option::OptionInsertExt;
    pub use option::OptionIterator;
    pub use option::OptionIteratorExt;
    pub use option::OptionKey;
    pub use option::OptionNumber;

    pub use super::SocketAddrExt;
}

use futures::prelude::*;
use prelude::*;
