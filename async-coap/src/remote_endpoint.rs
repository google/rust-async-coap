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
use crate::UriBuf;

/// An object that represents a remote CoAP endpoint with a default, overridable path.
///
/// # Example
///
/// ```
/// # #![feature(async_await)]
/// #
/// # use std::sync::Arc;
/// # use futures::{prelude::*,executor::LocalPool,task::LocalSpawnExt};
/// # use async_coap::prelude::*;
/// # use async_coap::datagram::{DatagramLocalEndpoint,AllowStdUdpSocket};
/// #
/// # // Create our asynchronous socket. In this case, it is just an
/// # // (inefficient) wrapper around the standard rust `UdpSocket`,
/// # // but that is quite adequate in this case.
/// # let socket = AllowStdUdpSocket::bind("[::]:0").expect("UDP bind failed");
/// #
/// # // Create a new local endpoint from the socket we just created,
/// # // wrapping it in a `Arc<>` to ensure it can live long enough.
/// # let local_endpoint = Arc::new(DatagramLocalEndpoint::new(socket));
/// #
/// # // Create a local execution pool for running our local endpoint.
/// # let mut pool = LocalPool::new();
/// #
/// # // Add our local endpoint to the pool, so that it
/// # // can receive packets.
/// # pool.spawner().spawn_local(local_endpoint
/// #     .clone()
/// #     .receive_loop_arc(null_receiver!())
/// #     .map(|err| panic!("Receive loop terminated: {}", err))
/// # );
/// #
/// # let future = async move {
/// // Create a remote endpoint instance to represent the
/// // device we wish to interact with.
/// let remote_endpoint = local_endpoint
///     .remote_endpoint_from_uri(uri!("coap://coap.me"))
///     .unwrap(); // Will only fail if the URI scheme or authority is unrecognizable
///
/// // Create a future that sends a request to a specific path
/// // on the remote endpoint, collecting any blocks in the response
/// // and returning `Ok(OwnedImmutableMessage)` upon success.
/// let future = remote_endpoint.send_to(
///     rel_ref!("large"),
///     CoapRequest::get()       // This is a CoAP GET request
///         .accept(ContentFormat::TEXT_PLAIN_UTF8) // We only want plaintext
///         .block2(Some(Default::default()))       // Enable block2 processing
///         .emit_successful_collected_response()                 // Collect all blocks into a single message
/// );
///
/// // Wait for the final result and print it.
/// println!("result: {:?}", future.await.unwrap());
/// # };
/// #
/// # pool.run_until(future);
/// ```
///
pub trait RemoteEndpoint {
    /// The `SocketAddr` type to use with this local endpoint. This is usually
    /// simply `std::net::SocketAddr`, but may be different in some cases (like for CoAP-SMS
    /// endpoints).
    type SocketAddr: SocketAddrExt;

    /// Type used by closure that is passed into `send()`, representing the context for the
    /// response.
    type InboundContext: InboundContext<SocketAddr = Self::SocketAddr>;

    /// Returns a [`UriBuf`] describing the underlying destination of this remote endpoint.
    fn uri(&self) -> UriBuf;

    /// Returns a string slice containing the scheme for this `RemoteEndpoint`.
    fn scheme(&self) -> &'static str;

    /// Prevents this remote endpoint from including a `Uri-Host` option.
    fn remove_host_option(&mut self);

    /// Creates a clone of this `RemoteEndpoint` with a different relative path.
    fn clone_using_rel_ref(&self, uri: &RelRef) -> Self;

    /// Uses `send_desc` to send a request to the endpoint and path described by this
    /// `RemoteEndpoint` instance.
    fn send<'a, R, SD>(&'a self, send_desc: SD) -> BoxFuture<'_, Result<R, Error>>
    where
        SD: SendDesc<Self::InboundContext, R> + 'a,
        R: Send + 'a;

    /// Uses `send_desc` to send a request to the given relative path on the endpoint described
    /// by this `RemoteEndpoint` instance.
    fn send_to<'a, R, SD, UF>(&'a self, path: UF, send_desc: SD) -> BoxFuture<'_, Result<R, Error>>
    where
        SD: SendDesc<Self::InboundContext, R> + 'a,
        R: Send + 'a,
        UF: AsRef<RelRef>;
}

/// Extension trait which implements additional helper methods.
pub trait RemoteEndpointExt: RemoteEndpoint {
    /// Sends an application-level ping to to one or more addresses specified by `dest`.
    /// The first response received causes the future to emit `Ok(())`.
    fn ping(&self) -> BoxFuture<'_, Result<(), Error>> {
        self.send(Ping::new())
    }

    /// Analogous to [`LocalEndpointExt::send_as_stream`], except using this `RemoteEndpoint` for
    /// the destination SocketAddr and path.
    fn send_as_stream<'a, R, SD>(&'a self, send_desc: SD) -> SendAsStream<'a, R>
    where
        SD: SendDesc<Self::InboundContext, R> + 'a,
        R: Send + 'a,
    {
        let (sender, receiver) = futures::channel::mpsc::channel::<Result<R, Error>>(10);

        SendAsStream {
            receiver,
            send_future: self.send(SendAsStreamDesc::new(send_desc, sender)),
        }
    }

    /// Analogous to [`LocalEndpointExt::send_as_stream`], except using this `RemoteEndpoint` for
    /// the destination SocketAddr and using a path relative to this `RemoteEndpoint`.
    fn send_to_as_stream<'a, R, SD, UF>(&'a self, path: UF, send_desc: SD) -> SendAsStream<'a, R>
    where
        SD: SendDesc<Self::InboundContext, R> + 'a,
        R: Send + 'a,
        UF: AsRef<RelRef>,
    {
        let (sender, receiver) = futures::channel::mpsc::channel::<Result<R, Error>>(10);

        SendAsStream {
            receiver,
            send_future: self.send_to(path, SendAsStreamDesc::new(send_desc, sender)),
        }
    }
}

/// Blanket implementation of `RemoteEndpointExt` for all `RemoteEndpoint` instances.
impl<T: RemoteEndpoint> RemoteEndpointExt for T {}
