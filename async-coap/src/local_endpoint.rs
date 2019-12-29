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

use super::remote_endpoint::RemoteEndpoint;
use futures::stream::Collect;
use std::sync::Arc;

/// Trait representing a local (as opposed to remote) CoAP endpoint. Allows for sending and
/// receiving CoAP requests.
///
/// # Implementations
///
/// `LocalEndpoint` is a trait, which allows for multiple back-end implementations.
/// `async-coap` comes with two: [`NullLocalEndpoint`] and [`DatagramLocalEndpoint`].
///
/// [`NullLocalEndpoint`] does what you might expect: nothing. Attempts to send
/// requests always results in [`Error::ResponseTimeout`] and [`LocalEndpoint::receive`]
/// will block indefinitely. Creating an instance of it is quite straightforward:
///
/// [`NullLocalEndpoint`]: crate::null::NullLocalEndpoint
/// [`DatagramLocalEndpoint`]: crate::datagram::DatagramLocalEndpoint
///
/// ```
/// use std::sync::Arc;
/// use async_coap::null::NullLocalEndpoint;
///
/// let local_endpoint = Arc::new(NullLocalEndpoint);
/// ```
///
/// If you want to do something more useful, then [`DatagramLocalEndpoint`] is likely
/// what you are looking for. It takes an instance of [`AsyncDatagramSocket`] at construction:
///
/// [`AsyncDatagramSocket`]: crate::datagram::AsyncDatagramSocket
///
/// ```
/// use std::sync::Arc;
/// use async_coap::prelude::*;
/// use async_coap::datagram::{DatagramLocalEndpoint,AllowStdUdpSocket};
///
/// // `AllowStdUdpSocket`, which is a (inefficient) wrapper around the
/// // standard rust `UdpSocket`. It is convenient for testing and for examples
/// // but should not be used in production code.
/// let socket = AllowStdUdpSocket::bind("[::]:0").expect("UDP bind failed");
///
/// // Create a new local endpoint from the socket instance we just created,
/// // wrapping it in a `Arc<>` to ensure it can live long enough.
/// let local_endpoint = Arc::new(DatagramLocalEndpoint::new(socket));
/// ```
///
/// # Client Usage
///
/// Before you can start sending requests and receiving responses, you
/// will need to make sure that the [`LocalEndpoint::receive`] method
/// gets called repeatedly. The easiest way to do that is to add the
/// [`std::future::Future`] returned by [`LocalEndpointExt::receive_loop_arc`]
/// to an execution pool:
///
/// ```
/// # use std::sync::Arc;
/// # use async_coap::prelude::*;
/// # use async_coap::datagram::{DatagramLocalEndpoint, AllowStdUdpSocket, LoopbackSocket};
/// # use async_coap::null::NullLocalEndpoint;
/// #
/// # let local_endpoint = Arc::new(NullLocalEndpoint);
/// #
/// use futures::{prelude::*,executor::ThreadPool,task::Spawn,task::SpawnExt};
///
/// let mut pool = ThreadPool::new().expect("Unable to create thread pool");
///
/// // We use a receiver handler of `null_receiver!()` because this instance
/// // will be used purely as a client, not a server.
/// pool.spawn(local_endpoint
///     .clone()
///     .receive_loop_arc(null_receiver!())
///     .map(|_|unreachable!())
/// );
/// ```
///
/// Once the `Arc<LocalEndpint>` has been added to an execution pool, the `run_until` method
/// on the pool can be used to block execution of the futures emitted by `LocalEndpoint`:
///
/// ```
/// # use std::sync::Arc;
/// # use futures::{prelude::*,executor::LocalPool,task::LocalSpawnExt};
/// # use async_coap::prelude::*;
/// # use async_coap::datagram::{DatagramLocalEndpoint, AllowStdUdpSocket, LoopbackSocket};
/// # use async_coap::null::NullLocalEndpoint;
/// #
/// # // Using a NullLocalEndpoint since this is just a simple usage example.
/// # let local_endpoint = Arc::new(NullLocalEndpoint);
/// # let mut local_pool = LocalPool::new();
/// #
/// # local_pool.spawner().spawn_local(local_endpoint
/// #     .clone()
/// #     .receive_loop_arc(null_receiver!())
/// #     .map(|_|unreachable!())
/// # );
///
/// let result = local_pool.run_until(
///     local_endpoint.send(
///         "coap.me:5683",
///         CoapRequest::get()       // This is a CoAP GET request
///             .emit_any_response() // Return the first response we get
///     )
/// );
///
/// println!("result: {:?}", result);
/// ```
///
/// Or, more naturally, the returned futures can be used directly in `async` blocks:
///
/// ```
/// # use std::sync::Arc;
/// # use futures::{prelude::*,executor::LocalPool,task::LocalSpawnExt};
/// # use async_coap::prelude::*;
/// # use async_coap::datagram::{DatagramLocalEndpoint, AllowStdUdpSocket, LoopbackSocket};
/// # use async_coap::null::NullLocalEndpoint;
/// #
/// # // Using a NullLocalEndpoint since this is just a simple usage example.
/// # let local_endpoint = Arc::new(NullLocalEndpoint);
/// # let mut pool = LocalPool::new();
/// #
/// # pool.spawner().spawn_local(local_endpoint
/// #     .clone()
/// #     .receive_loop_arc(null_receiver!())
/// #     .map(|_|unreachable!())
/// # );
/// #
/// # let future =
/// async move {
///     let future = local_endpoint.send(
///         "coap.me:5683",
///         CoapRequest::get()       // This is a CoAP GET request
///             .emit_any_response() // Return the first response we get
///     );
///
///     // Wait for the final result and print it.
///     println!("result: {:?}", future.await);
/// }
/// # ;
/// #
/// # pool.run_until(future);
/// ```
///
/// # Server Usage
///
/// In order to serve resources for other devices to interact with, you will
/// need to replace the [`null_receiver!`] we were using earlier with something
/// more substantial. The method takes a closure as an argument, and the closure
/// itself has a single argument: a borrowed [`RespondableInboundContext`].
///
/// For example, to have our server return a response for a request instead of
/// just returning an error, we could use the following function as our receive handler:
///
/// ```
/// use async_coap::prelude::*;
/// use async_coap::{RespondableInboundContext, Error};
///
/// fn receive_handler<T: RespondableInboundContext>(context: &T) -> Result<(),Error> {
///     context.respond(|msg_out|{
///         msg_out.set_msg_code(MsgCode::SuccessContent);
///         msg_out.insert_option(option::CONTENT_FORMAT, ContentFormat::TEXT_PLAIN_UTF8)?;
///         msg_out.append_payload_string("Successfully fetched!")?;
///         Ok(())
///     })?;
///     Ok(())
/// }
/// # use std::sync::Arc;
/// # use futures::{prelude::*,executor::LocalPool,task::LocalSpawnExt};
/// # use async_coap::datagram::{DatagramLocalEndpoint, AllowStdUdpSocket, LoopbackSocket, LoopbackSocketAddr};
/// # use async_coap::null::NullLocalEndpoint;
/// # use async_coap::message::MessageRead;
/// #
/// # let local_endpoint = Arc::new(DatagramLocalEndpoint::new(LoopbackSocket::new()));
/// # let mut pool = LocalPool::new();
/// #
/// # pool.spawner().spawn_local(local_endpoint.clone().receive_loop_arc(receive_handler).map(|_|unreachable!()));
/// #
/// # let result = pool.run_until(
/// #     local_endpoint.send(
/// #         LoopbackSocketAddr::Unicast,
/// #         CoapRequest::get()       // This is a CoAP GET request
/// #             .emit_any_response() // Return the first response we get
/// #     )
/// # );
/// # println!("result: {:?}", result);
/// # let result = result.unwrap();
/// # assert_eq!(result.msg_code(), MsgCode::SuccessContent);
/// # assert_eq!(result.msg_type(), MsgType::Ack);
/// ```
///
/// However, that's actually not super useful: it returns a successful result for
/// every possible request: including bogus ones. Let's say that we wanted to expose a
/// resource that lives at "`/test`" on our server, returning a [`4.04 Not Found`](MsgCode::ClientErrorNotFound)
/// for every other request. That might look something like this:
///
/// ```
/// use async_coap::prelude::*;
/// use async_coap::{RespondableInboundContext, Error, LinkFormatWrite, LINK_ATTR_TITLE};
/// use core::fmt::Write; // For `write!()`
/// use core::borrow::Borrow;
/// use option::CONTENT_FORMAT;
///
/// fn receive_handler<T: RespondableInboundContext>(context: &T) -> Result<(),Error> {
///     let msg = context.message();
///     let uri = msg.options().extract_uri()?;
///     let decoded_path = uri.raw_path().unescape_uri().skip_slashes().to_cow();
///
///     match (msg.msg_code(), decoded_path.borrow()) {
///         // Handle GET /test
///         (MsgCode::MethodGet, "test") => context.respond(|msg_out| {
///             msg_out.set_msg_code(MsgCode::SuccessContent);
///             msg_out.insert_option(CONTENT_FORMAT, ContentFormat::TEXT_PLAIN_UTF8);
///             write!(msg_out,"Successfully fetched {:?}!", uri.as_str())?;
///             Ok(())
///         }),
///
///         // Handle GET /.well-known/core, for service discovery.
///         (MsgCode::MethodGet, ".well-known/core") => context.respond(|msg_out| {
///             msg_out.set_msg_code(MsgCode::SuccessContent);
///             msg_out.insert_option(CONTENT_FORMAT, ContentFormat::APPLICATION_LINK_FORMAT);
///             LinkFormatWrite::new(msg_out)
///                 .link(uri_ref!("/test"))
///                     .attr(LINK_ATTR_TITLE, "Test Resource")
///                     .finish()?;
///             Ok(())
///         }),
///
///         // Handle unsupported methods
///         (_, "test") | (_, ".well-known/core") => context.respond(|msg_out| {
///            msg_out.set_msg_code(MsgCode::ClientErrorMethodNotAllowed);
///             write!(msg_out,"Method \"{:?}\" Not Allowed", msg.msg_code())?;
///             Ok(())
///         }),
///
///         // Everything else is a 4.04
///         (_, _) => context.respond(|msg_out| {
///             msg_out.set_msg_code(MsgCode::ClientErrorNotFound);
///             write!(msg_out,"{:?} Not Found", uri.as_str())?;
///             Ok(())
///         }),
///     }
/// }
/// # use std::sync::Arc;
/// # use futures::{prelude::*,executor::LocalPool,task::LocalSpawnExt};
/// # use async_coap::datagram::{DatagramLocalEndpoint, AllowStdUdpSocket, LoopbackSocket, LoopbackSocketAddr};
/// # use async_coap::null::NullLocalEndpoint;
/// # use async_coap::message::MessageRead;
/// # use std::borrow::Cow;
/// #
/// # let local_endpoint = Arc::new(DatagramLocalEndpoint::new(LoopbackSocket::new()));
/// # let mut pool = LocalPool::new();
/// #
/// # pool.spawner().spawn_local(local_endpoint
/// #     .clone()
/// #     .receive_loop_arc(receive_handler)
/// #     .map(|_|unreachable!())
/// # );
/// #
/// # let result = pool.run_until(
/// #     local_endpoint.send(
/// #         LoopbackSocketAddr::Unicast,
/// #         CoapRequest::get()       // This is a CoAP GET request
/// #             .uri_host_path(None, rel_ref!("test")) // Add a path to the message
/// #             .emit_any_response() // Return the first response we get
/// #     )
/// # );
/// # println!("result: {:?}", result);
/// # let result = result.unwrap();
/// # assert_eq!(result.msg_code(), MsgCode::SuccessContent);
/// # assert_eq!(result.msg_type(), MsgType::Ack);
/// #
/// #
/// # let result = pool.run_until(
/// #     local_endpoint.send(
/// #         LoopbackSocketAddr::Unicast,
/// #         CoapRequest::post()       // This is a CoAP POST request
/// #             .uri_host_path(None, rel_ref!("test")) // Add a path to the message
/// #             .emit_successful_response() // Return the first successful response we get
/// #             .inspect(|cx| {
/// #                 // Inspect here since we currently can't do
/// #                 // a detailed check in the return value.
/// #                 assert_eq!(cx.message().msg_code(), MsgCode::ClientErrorMethodNotAllowed);
/// #                 assert_eq!(cx.message().msg_type(), MsgType::Ack);
/// #             })
/// #     )
/// # );
/// # println!("result: {:?}", result);
/// # assert_eq!(result.err(), Some(Error::ClientRequestError));
/// #
/// # let result = pool.run_until(
/// #     local_endpoint.send(
/// #         LoopbackSocketAddr::Unicast,
/// #         CoapRequest::get()       // This is a CoAP GET request
/// #             .emit_successful_response() // Return the first successful response we get
/// #             .uri_host_path(None, rel_ref!("/foobar"))
/// #             .inspect(|cx| {
/// #                 // Inspect here since we currently can't do
/// #                 // a detailed check in the return value.
/// #                 assert_eq!(cx.message().msg_code(), MsgCode::ClientErrorNotFound);
/// #                 assert_eq!(cx.message().msg_type(), MsgType::Ack);
/// #             })
/// #     )
/// # );
/// # println!("result: {:?}", result);
/// # assert_eq!(result.err(), Some(Error::ResourceNotFound));
/// ```
///
pub trait LocalEndpoint: Sized {
    /// The `SocketAddr` type to use with this local endpoint. This is usually
    /// simply `std::net::SocketAddr`, but may be different in some cases (like for CoAP-SMS
    /// endpoints).
    type SocketAddr: SocketAddrExt
        + ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::SocketError>;

    /// The error type associated with errors generated by socket and address-lookup operations.
    /// Typically, this is `std::io::Error`, but it may be different if `Self::SocketAddr` isn't
    /// `std::net::SocketAddr`.
    type SocketError: core::fmt::Debug;

    /// The trait representing the default transmission parameters to use.
    type DefaultTransParams: TransParams;

    /// Type used by closure that is passed into `send()`, representing the context for the
    /// response.
    type InboundContext: InboundContext<SocketAddr = Self::SocketAddr>;

    /// Type used by closure that is passed into `receive()`, representing the context for
    /// inbound requests.
    type RespondableInboundContext: RespondableInboundContext<SocketAddr = Self::SocketAddr>;

    /// Returns a string representing the scheme of the underlying transport.
    /// For example, this could return `"coap"`, `"coaps+sms"`, etc.
    fn scheme(&self) -> &str;

    /// Returns the default port to use when the port is unspecified. This value
    /// is typically defined by the scheme. Returns zero if port numbers are ignored
    /// by the underlying technology.
    fn default_port(&self) -> u16;

    /// The concrete return type of the `lookup()` method.
    type LookupStream: Stream<Item = Self::SocketAddr> + Unpin;

    /// Method for asynchronously looking up the `Self::SocketAddr` instances for the
    /// given hostname and port.
    fn lookup(&self, hostname: &str, port: u16) -> Result<Self::LookupStream, Error>;

    /// The concrete type for a `RemoteEndpoint` associated with this local endpoint.
    type RemoteEndpoint: RemoteEndpoint<
        SocketAddr = Self::SocketAddr,
        InboundContext = Self::InboundContext,
    >;

    /// Constructs a new [`RemoteEndpoint`] instance for the given address, host, and path.
    fn remote_endpoint<S, H, P>(&self, addr: S, host: Option<H>, path: P) -> Self::RemoteEndpoint
    where
        S: ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::SocketError>,
        H: Into<String>,
        P: Into<RelRefBuf>;

    /// Constructs a new [`RemoteEndpoint`] instance for the given Uri.
    fn remote_endpoint_from_uri(&self, uri: &Uri) -> Result<Self::RemoteEndpoint, Error>;

    /// Sends a message to `remote_addr` based on the criteria provided by
    /// [`send_desc`][crate::SendDesc].
    ///
    /// `send_desc`, which implements [`SendDesc`][crate::SendDesc], is the real heavy lifter here.
    /// It defines the message content, retransmit timing, resending logic---even the
    /// return type of this method if defined by `send_desc`.
    /// This flexibility allows this method to uniformly perform complex interactions
    /// like [block transfers][IETF-RFC7959] and [resource observing][IETF-RFC7641].
    ///
    /// [IETF-RFC7959]: https://tools.ietf.org/html/rfc7959
    /// [IETF-RFC7641]: https://tools.ietf.org/html/rfc7641
    ///
    /// A variant of this method, [`LocalEndpointExt::send_as_stream`], is used to
    /// handle cases where multiple responses are expected, such as when sending
    /// multicast requests or doing [resource observing][IETF-RFC7641].
    ///
    /// ## Performance Tips
    ///
    /// If you are going to be calling this method frequently for a destination that you are
    /// referencing by a hostname, it will significantly improve performance on some platforms
    /// if you only pass `SocketAddr` types to `remote_addr` and not rely on having `ToSocketAddrs`
    /// do hostname lookups inside of `send`.
    ///
    /// The easiest way to do this is to use either the [`remote_endpoint`] or
    /// [`remote_endpoint_from_uri`] methods to create a [`RemoteEndpoint`] instance
    /// and call the [`send`][crate::RemoteEndpoint::send] method on that instead. From that
    /// instance you can call `send` multiple times: any hostname that needs to be resolved
    /// is calculated and cached when the `RemoteEndpoint` is first created.
    ///
    /// [`RemoteEndpoint`]: crate::RemoteEndpoint
    /// [`remote_endpoint`]: LocalEndpoint::remote_endpoint
    /// [`remote_endpoint_from_uri`]: LocalEndpoint::remote_endpoint_from_uri
    ///
    /// ## Transaction Tracking
    ///
    /// All state tracking the transmission of the message is stored in the returned future.
    /// To cancel retransmits, drop the returned future.
    ///
    /// The returned future is lazily evaluated, so nothing will be transmitted unless the
    /// returned future is polled: you cannot simply fire and forget. Because of this lazy
    /// evaluation, the futures returned by this method do not need to be used immediately and
    /// may be stored for later use if that happens to be useful.
    ///
    #[must_use = "nothing will be sent unless the returned future is polled"]
    fn send<'a, S, R, SD>(
        &'a self,
        remote_addr: S,
        send_desc: SD,
    ) -> BoxFuture<'a, Result<R, Error>>
    where
        S: ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::SocketError> + 'a,
        SD: SendDesc<Self::InboundContext, R> + 'a,
        R: Send + 'a;

    /// Receives a single request and runs the given `handler` on it once.
    ///
    /// Each call handles (at most) one single inbound request.
    /// To handle multiple requests, call this function from a loop.
    /// The [`LocalEndpointExt`] trait comes with some helpers to make
    /// implementing such a loop easier: [`receive_as_stream`],
    /// [`receive_loop`], and [`receive_loop_arc`].
    ///
    /// [`receive_as_stream`]: LocalEndpointExt::receive_as_stream
    /// [`receive_loop`]: LocalEndpointExt::receive_loop
    /// [`receive_loop_arc`]: LocalEndpointExt::receive_loop_arc
    ///
    /// *Properly calling this method in the background is absolutely critical to
    /// the correct operation of this trait:* **[`send`] will not work without it**.
    ///
    /// [`send`]: LocalEndpoint::send
    ///
    /// Local endpoints which implement [`Sync`] can have this method called from multiple
    /// threads, allowing multiple requests to be handled concurrently.
    ///
    /// ## Handler
    ///
    /// If you are going to be serving resources using this [`LocalEndpoint`], you
    /// will need specify a handler to handle inbound requests.
    /// See the section [Server Usage](#server-usage) above for an example.
    ///
    /// If instead you are only using this [`LocalEndpoint`] as a client, then you may pass
    /// `null_receiver!()` as the handler, as shown in [Client Usage](#client-usage).
    #[must_use = "nothing will be received unless the returned future is polled"]
    fn receive<'a, F>(&'a self, handler: F) -> BoxFuture<'a, Result<(), Error>>
    where
        F: FnMut(&Self::RespondableInboundContext) -> Result<(), Error> + 'a + Send + Unpin;
}

/// Handler for [`LocalEndpoint::receive`] that does nothing and lets the underlying
/// [`LocalEndpoint`] implementation decide how best to respond (if at all).
#[macro_export]
macro_rules! null_receiver {
    ( ) => {
        |_| Ok(())
    };
}

/// Extension trait for [`LocalEndpoint`] which implements additional helper methods.
pub trait LocalEndpointExt: LocalEndpoint {
    /// Sends a message where multiple responses are expected, returned as a [`SendAsStream`].
    ///
    /// In this version of [`LocalEndpoint::send`], the `send_desc` can return
    /// [`ResponseStatus::Done`] from its handler multiple times, with the results being emitted
    /// from the returned [`SendAsStream`].
    ///
    /// The stream can be cleanly ended by the handler eventually returning
    /// [`Error::ResponseTimeout`] or [`Error::Cancelled`], neither of which will be emitted
    /// as an error.
    fn send_as_stream<'a, S, R, SD>(&'a self, dest: S, send_desc: SD) -> SendAsStream<'a, R>
    where
        S: ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::SocketError> + 'a,
        SD: SendDesc<Self::InboundContext, R> + 'a,
        R: Send + 'a,
    {
        let (sender, receiver) = futures::channel::mpsc::channel::<Result<R, Error>>(10);

        SendAsStream {
            receiver,
            send_future: self.send(dest, SendAsStreamDesc::new(send_desc, sender)),
        }
    }

    /// Version of [`LocalEndpoint::receive`] that handles more than one inbound message,
    /// returning a [`crate::ReceiveAsStream`] instead of a future.
    ///
    /// This stream will terminate immediately after any of the following errors are emitted by the
    /// underlying calls to [`LocalEndpoint::receive`]:
    ///
    /// * [`Error::IOError`](enum_Error.html#variant.IOError)
    /// * [`Error::Cancelled`](enum_Error.html#variant.Cancelled)
    ///
    /// All other errors are ignored.
    fn receive_as_stream<'a, F>(&'a self, handler: F) -> ReceiveAsStream<'a, Self, F>
    where
        F: FnMut(&Self::RespondableInboundContext) -> Result<(), Error> + 'a + Clone + Unpin + Send,
    {
        ReceiveAsStream::new(self, handler)
    }

    /// Convenience method for implementing a [`receive`](LocalEndpoint::receive) loop.
    ///
    /// The returned future will terminate when the underlying [`crate::ReceiveAsStream`]
    /// terminates, returning the error that was emitted before the stream terminated,
    /// typically either [`Error::IOError`] or [`Error::Cancelled`].
    fn receive_loop<'a, F>(&'a self, handler: F) -> Collect<ReceiveAsStream<'a, Self, F>, Error>
    where
        F: FnMut(&Self::RespondableInboundContext) -> Result<(), Error> + 'a + Clone + Unpin + Send,
    {
        self.receive_as_stream(handler).collect()
    }

    /// Version of [`LocalEndpointExt::receive_loop`] which consumes and holds an [`Arc<Self>`].
    ///
    /// [`LocalEndpoint`s][LocalEndpoint] are often held inside of an [`Arc<>`], which makes
    /// using methods like [`LocalEndpointExt::receive_loop`] particularly awkward.
    ///
    /// `receive_loop_arc` makes this situation relatively painless by returning the receive loop
    /// future in an (effectively transparent) [`ArcGuard`] wrapper.
    ///
    /// ```
    /// #
    /// # use std::sync::Arc;
    /// # use futures::prelude::*;
    /// # use async_coap::prelude::*;
    /// # use async_coap::null::NullLocalEndpoint;
    /// # use futures::executor::ThreadPool;
    /// # use futures::task::SpawnExt;
    ///
    /// let local_endpoint = Arc::new(NullLocalEndpoint);
    /// let mut pool = ThreadPool::new().expect("Unable to start thread pool");
    ///
    /// pool.spawn(local_endpoint
    ///     .clone()
    ///     .receive_loop_arc(null_receiver!())
    ///     .map(|err| panic!("Receive loop terminated: {}", err))
    /// );
    /// ```
    fn receive_loop_arc<'a, F>(
        self: Arc<Self>,
        handler: F,
    ) -> ArcGuard<Self, Collect<ReceiveAsStream<'a, Self, F>, Error>>
    where
        F: FnMut(&Self::RespondableInboundContext) -> Result<(), Error> + 'a + Clone + Send + Unpin,
        Self: 'a,
    {
        self.guard(|x| x.receive_loop(handler))
    }
}

/// Blanket implementation of `LocalEndpointExt` for all `LocalEndpoint` instances.
impl<T: LocalEndpoint> LocalEndpointExt for T {}
