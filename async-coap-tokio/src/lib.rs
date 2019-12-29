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

//! This crate provides [`TokioAsyncUdpSocket`]\: an asynchronous, [Tokio][]-based
//! implementation of [`AsyncDatagramSocket`] for use with [`DatagramLocalEndpoint`].
//!
//! # Example
//!
//! ```no_run
//! use async_coap::prelude::*;
//! use async_coap::datagram::DatagramLocalEndpoint;
//! use async_coap_tokio::TokioAsyncUdpSocket;
//! use futures::prelude::*;
//! use std::sync::Arc;
//! use tokio::spawn;
//!
//! #[tokio::main]
//! async fn main() {
//!     let socket = TokioAsyncUdpSocket::bind("[::]:0")
//!         .expect("UDP bind failed");
//!
//!     // Create a new local endpoint from the socket we just created,
//!     // wrapping it in a `Arc<>` to ensure it can live long enough.
//!     let local_endpoint = Arc::new(DatagramLocalEndpoint::new(socket));
//!
//!     // Add our local endpoint to the pool, so that it
//!     // can receive packets.
//!     spawn(
//!         local_endpoint
//!             .clone()
//!             .receive_loop_arc(null_receiver!())
//!             .map(|err| panic!("Receive loop terminated: {}", err)),
//!     );
//!
//!     // Create a remote endpoint instance to represent the
//!     // device we wish to interact with.
//!     let remote_endpoint = local_endpoint
//!         .remote_endpoint_from_uri(uri!("coap://coap.me"))
//!         .expect("Unacceptable scheme or authority in URL");
//!
//!     // Create a future that sends a request to a specific path
//!     // on the remote endpoint, collecting any blocks in the response
//!     // and returning `Ok(OwnedImmutableMessage)` upon success.
//!     let future = remote_endpoint.send_to(
//!         rel_ref!("large"),
//!         CoapRequest::get() // This is a CoAP GET request
//!             .accept(ContentFormat::TEXT_PLAIN_UTF8) // We only want plaintext
//!             .block2(Some(Default::default())) // Enable block2 processing
//!             .emit_successful_collected_response(), // Collect all blocks
//!     );
//!
//!     // Wait until we get the result of our request.
//!     let result = future.await;
//!
//!     assert!(result.is_ok(), "Error: {:?}", result.err().unwrap());
//! }
//! ```
//!
//! [`AsyncDatagramSocket`]: async-coap::datagram::AsyncDatagramSocket
//! [`DatagramLocalEndpoint`]: async-coap::datagram::DatagramLocalEndpoint
//! [Tokio]: https://tokio.rs/

mod tokio_async_udp_socket;
pub use tokio_async_udp_socket::TokioAsyncUdpSocket;
