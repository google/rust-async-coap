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

//! # Send Descriptors
//!
//! *Send Descriptors* are types that implement [`SendDesc`] that can be passed to the `send*`
//! methods of [`LocalEndpoint`] and [`RemoteEndpoint`]. They define almost every aspect of how
//! a message transaction is handled.
//!
//! Typical usage of this crate does not require writing implementing [`SendDesc`] by hand,
//! although you could certainly do so if needed.
//! Instead, `SyncDesc` instances are easily constructed using *combinators*.
//!
//! ## Example
//!
//! Here we create a `SendDesc` instance that just sends a GET request and waits for a response:
//!
//! ```
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
//! #
//! let mut remote_endpoint = local_endpoint
//!     .remote_endpoint_from_uri(uri!("coap://coap.me:5683/test"))
//!     .expect("Remote endpoint lookup failed");
//!
//! let future = remote_endpoint.send(CoapRequest::get());
//!
//! assert_eq!(future.await, Ok(()));
//! #
//! #
//! # };
//! # pool.run_until(future);
//! ```
//!
//! That `SendDesc` was perhaps a little *too* simple: it doesn't even interpret the results,
//! returning `Ok(())` for any message responding with a `2.05 Content` message!
//!
//! By using the combinator `.emit_successful_response()`, we can have our `SendDesc` return
//! an owned copy of the message it received ([`OwnedImmutableMessage`](crate::message::OwnedImmutableMessage)):
//!
//! ```
//! # use std::sync::Arc;
//! # use futures::{prelude::*,executor::LocalPool,task::LocalSpawnExt};
//! # use async_coap::prelude::*;
//! # use async_coap::datagram::{DatagramLocalEndpoint, AllowStdUdpSocket, LoopbackSocket};
//! # let socket = AllowStdUdpSocket::bind("[::]:0").expect("UDP bind failed");
//! # let local_endpoint = Arc::new(DatagramLocalEndpoint::new(socket));
//! # let mut pool = LocalPool::new();
//! # pool.spawner().spawn_local(local_endpoint.clone().receive_loop_arc(null_receiver!()).map(|_|unreachable!()));
//! # let future = async move {
//! #    use async_coap::message::OwnedImmutableMessage;
//! #    let mut remote_endpoint = local_endpoint
//! #        .remote_endpoint_from_uri(uri!("coap://coap.me:5683/test"))
//! #        .expect("Remote endpoint lookup failed");
//! #
//! #
//! let send_desc = CoapRequest::get().emit_successful_response();
//!
//! let future = remote_endpoint.send(send_desc);
//!
//! let message = future.await.expect("Request failed");
//!
//! println!("Got reply: {:?}", message);
//! #
//! #
//! # };
//! # pool.run_until(future);
//! ```
//!
//! What if we wanted the response in JSON? What if it was really large and we
//! knew we would need to do a block2 transfer? We can do that easily:
//!
//! ```ignore
//! let send_desc = CoapRequest::get()
//!     .accept(ContentFormat::APPLICATION_JSON)
//!     .block2(None)
//!     .emit_successful_collected_response();
//!
//! // Here we are specifying that we want to send the request to a specific
//! // path on the remote endpoint, `/large` in this case.
//! let future = remote_endpoint.send_to(rel_ref!("/large"), send_desc);
//!
//! let message = future.await.expect("Request failed");
//!
//! println!("Got reply: {:?}", message);
//! ```
//!
//! But if this is a large amount of data, we won't get any indication about the transfer
//! until it is done. What if we wanted to add some printouts about the status?
//!
//! ```ignore
//! let send_desc = CoapRequest::get()
//!     .accept(ContentFormat::APPLICATION_JSON)
//!     .block2(None)
//!     .emit_successful_collected_response()
//!     .inspect(|context| {
//!         let addr = context.remote_address();
//!         let msg = context.message();
//!
//!         // Print out each individual block message received.
//!         println!("Got {:?} from {}", msg, addr);
//!     });
//!
//! let future = remote_endpoint.send_to(rel_ref!("/large"), send_desc);
//!
//! let message = future.await.expect("Request failed");
//!
//! println!("Got reply: {:?}", message);
//! ```
//!
//! There are [many more combinators][SendDescExt] for doing all sorts of things, such as
//! adding additional options and [block2 message aggregation](SendDescUnicast::block2).

use super::*;

mod request;
pub use request::*;

mod observe;
pub use observe::*;

mod unicast_block2;
pub use unicast_block2::*;

mod handler;
pub use handler::*;

mod inspect;
pub use inspect::*;

mod payload;
pub use payload::*;

mod ping;
pub use ping::Ping;

mod add_option;
pub use add_option::*;

mod nonconfirmable;
pub use nonconfirmable::*;

mod multicast;
pub use multicast::*;

mod emit;
pub use emit::*;

mod include_socket_addr;
pub use include_socket_addr::*;

mod uri_host_path;
pub use uri_host_path::UriHostPath;

use std::iter::{once, Once};
use std::marker::PhantomData;
use std::ops::Bound;
use std::time::Duration;

/// # Send Descriptor Trait
///
/// Types implementing this trait can be passed to the `send*` methods of [`LocalEndpoint`]
/// and [`RemoteEndpoint`], and can define almost every aspect of how a message transaction
/// is handled.
///
/// See the [module level documentation](index.html) for more information on typical usage
/// patterns.
///
/// ## Internals
///
/// There are several methods in this trait, but three of them are critical:
///
/// * [`write_options`](SendDesc::write_options)\: Defines which options are going to be
///   included in the outbound message.
/// * [`write_payload`](SendDesc::write_payload)\: Defines the contents of the payload for the
///   outbound message.
/// * [`handler`](SendDesc::handler)\: Handles inbound reply messages, as well as error conditions.
///
pub trait SendDesc<IC, R = (), TP = StandardCoapConstants>: Send
where
    IC: InboundContext,
    R: Send,
    TP: TransParams,
{
    /// **Experimental**: Gets custom transmission parameters.
    fn trans_params(&self) -> Option<TP> {
        None
    }

    /// **Experimental**: Used for determining if the given option seen in the reply message
    /// is supported or not.
    ///
    /// Response messages with any options that cause this
    /// method to return false will be rejected.
    ///
    fn supports_option(&self, option: OptionNumber) -> bool {
        !option.is_critical()
    }

    /// Calculates the duration of the delay to wait before sending the next retransmission.
    ///
    /// If `None` is returned, then no further retransmissions will be attempted.
    fn delay_to_retransmit(&self, retransmits_sent: u32) -> Option<Duration> {
        if retransmits_sent > TP::COAP_MAX_RETRANSMIT {
            return None;
        }

        let ret = (TP::COAP_ACK_TIMEOUT.as_millis() as u64) << retransmits_sent as u64;

        const JDIV: u64 = 512u64;
        let rmod: u64 = (JDIV as f32 * (TP::COAP_ACK_RANDOM_FACTOR - 1.0)) as u64;
        let jmul = JDIV + rand::random::<u64>() % rmod;

        Some(Duration::from_millis(ret * jmul / JDIV))
    }

    /// The delay to wait between when we have received a successful response and when
    /// we should send out another request.
    ///
    /// The new request will have a new msg_id, but
    /// the same token. The retransmission counter will be reset to zero.
    ///
    /// This mechanism is currently used exclusively for CoAP observing.
    ///
    /// The default return value is `None`, indicating that there are to be no message
    /// restarts.
    fn delay_to_restart(&self) -> Option<Duration> {
        None
    }

    /// The maximum time to wait for an asynchronous response after having received an ACK.
    fn max_rtt(&self) -> Duration {
        TP::COAP_MAX_RTT
    }

    /// the maximum time from the first transmission of a Confirmable message to the time when
    /// the sender gives up on receiving an acknowledgement or reset.
    fn transmit_wait_duration(&self) -> Duration {
        TP::COAP_MAX_TRANSMIT_WAIT
    }

    /// Defines which options are going to be included in the outbound message.
    ///
    /// Writes all options in the given range to `msg`.
    fn write_options(
        &self,
        msg: &mut dyn OptionInsert,
        socket_addr: &IC::SocketAddr,
        start: Bound<OptionNumber>,
        end: Bound<OptionNumber>,
    ) -> Result<(), Error>;

    /// Generates the outbound message by making calls into `msg`.
    fn write_payload(
        &self,
        msg: &mut dyn MessageWrite,
        socket_addr: &IC::SocketAddr,
    ) -> Result<(), Error>;

    /// Handles the response to the outbound message.
    fn handler(&mut self, context: Result<&IC, Error>) -> Result<ResponseStatus<R>, Error>;
}

/// Marker trait for identifying that this `SendDesc` is for *unicast* requests.
/// Also contains unicast-specific combinators, such as [`block2()`][SendDescUnicast::block2].
pub trait SendDescUnicast {
    /// Returns a send descriptor that will perform Block2 processing.
    ///
    /// Note that just adding this to your send descriptor chain alone is unlikely to do what
    /// you want. You've got three options:
    ///
    /// * Add a call to [`emit_successful_collected_response`][UnicastBlock2::emit_successful_collected_response]
    ///   immediately after the call to this method. This will cause the message to be reconstructed from the blocks
    ///   and returned as a value from the future from `send`. You can optionally add an
    ///   [`inspect`][SendDescExt::inspect] combinator to get some feedback as the message is being
    ///   reconstructed from all of the individual block messages.
    /// * Add a call to [`emit_successful_response`][SendDescExt::emit_successful_response] along
    ///   with using `send_to_stream` instead of `send`. This will give you a `Stream` that will
    ///   contain all of the individual block messages in the stream.
    /// * [Add your own handler][SendDescExt::use_handler] to do whatever you need to do, returning
    ///   `ResponseStatus::SendNext` until all of the blocks have been received. This is
    ///   useful if you want to avoid memory allocation.
    ///
    /// There may be other valid combinations of combinators, depending on what you are trying
    /// to do.
    fn block2<IC, R, TP>(self, block2: Option<BlockInfo>) -> UnicastBlock2<Self, IC>
    where
        IC: InboundContext,
        R: Send,
        TP: TransParams,
        Self: SendDesc<IC, R, TP> + Sized,
    {
        UnicastBlock2::new(self, block2)
    }
}

/// Marker trait for identifying that this `SendDesc` is for *multicast* requests.
/// Also contains multicast-specific extensions.
pub trait SendDescMulticast {}

/// Combinator extension trait for Send Descriptors.
pub trait SendDescExt<IC, R, TP>: SendDesc<IC, R, TP> + Sized
where
    IC: InboundContext,
    R: Send,
    TP: TransParams,
{
    /// Adds zero or more instances of the option `key`, using values coming from `viter`.
    ///
    /// This method allows you to conditionally add options to a send descriptor. For example,
    /// you could convert an `Option` to an iterator (using `into_iterator()`) and pass it to
    /// this method: if the `Option` is `None` then no coap option will be added.
    fn add_option_iter<K, I>(self, key: OptionKey<K>, viter: I) -> AddOption<Self, K, I, IC>
    where
        I: IntoIterator<Item = K> + Send + Clone,
        K: Send + Clone,
    {
        AddOption {
            inner: self,
            key,
            viter,
            phantom: PhantomData,
        }
    }

    /// Adds one instance of the option `key` with a value of `value`.
    fn add_option<K>(self, key: OptionKey<K>, value: K) -> AddOption<Self, K, Once<K>, IC>
    where
        K: Send + Clone,
    {
        self.add_option_iter(key, once(value))
    }

    /// Adds an Accept option with the given `ContentFormat`.
    fn accept(
        self,
        accept: ContentFormat,
    ) -> AddOption<Self, ContentFormat, Once<ContentFormat>, IC> {
        self.add_option(option::ACCEPT, accept)
    }

    /// Adds an Content-Format option with the given `ContentFormat`.
    fn content_format(
        self,
        content_format: ContentFormat,
    ) -> AddOption<Self, ContentFormat, Once<ContentFormat>, IC> {
        self.add_option(option::CONTENT_FORMAT, content_format)
    }

    /// Adds a handler function to be called when a response message has been received (or when
    /// an error has occurred).
    fn use_handler<F, FR>(self, handler: F) -> Handler<Self, F>
    where
        F: FnMut(
                Result<&dyn InboundContext<SocketAddr = IC::SocketAddr>, Error>,
            ) -> Result<ResponseStatus<FR>, Error>
            + Send,
        FR: Send,
    {
        Handler {
            inner: self,
            handler,
        }
    }

    /// Updates the send descriptor chain to emit any received message as a result, even
    /// if that message has a message code that indicates an error.
    fn emit_any_response(self) -> EmitAnyResponse<Self> {
        EmitAnyResponse::new(self)
    }

    /// Updates the send descriptor chain to emit received message as a result, but only
    /// if that message has a message code that indicates success.
    fn emit_successful_response(self) -> EmitSuccessfulResponse<Self> {
        EmitSuccessfulResponse::new(self)
    }

    /// Updates the send descriptor chain to emit only the message code of the received
    /// response.
    fn emit_msg_code(self) -> EmitMsgCode<Self> {
        EmitMsgCode::new(self)
    }

    /// Updates the send descriptor chain to also emit the SocketAddr of the sender
    /// of the response, resulting in tuple return type.
    ///
    /// This is useful for handling responses to a multicast request.
    fn include_socket_addr(self) -> IncludeSocketAddr<Self> {
        IncludeSocketAddr::new(self)
    }

    /// Adds an inspection closure that will be called for each received response message.
    ///
    /// The inspector closure will not be called if no responses are received, and it cannot
    /// change the behavior of the send descriptor chain. If you need either of those
    /// behaviors, see [`SendDescExt::use_handler`].
    fn inspect<F>(self, inspect: F) -> Inspect<Self, F>
    where
        F: FnMut(&dyn InboundContext<SocketAddr = IC::SocketAddr>) + Send,
    {
        Inspect {
            inner: self,
            inspect,
        }
    }

    /// Adds a closure that writes to the payload of the outbound message.
    fn payload_writer<F>(self, writer: F) -> PayloadWriter<Self, F>
    where
        F: Fn(&mut dyn MessageWrite) -> Result<(), Error> + Send,
    {
        PayloadWriter {
            inner: self,
            writer,
        }
    }

    /// Allows you to specify the URI_HOST, URI_PATH, and URI_QUERY option values
    /// in a more convenient way than using `add_option_iter` manually.
    fn uri_host_path<T: Into<RelRefBuf>>(
        self,
        host: Option<String>,
        uri_path: T,
    ) -> UriHostPath<Self, IC> {
        UriHostPath {
            inner: self,
            host,
            path_and_query: uri_path.into(),
            phantom: PhantomData,
        }
    }
}

/// Blanket implementation of `SendDescExt` for all types implementing `SendDesc`.
impl<T, IC, R, TP> SendDescExt<IC, R, TP> for T
where
    T: SendDesc<IC, R, TP>,
    IC: InboundContext,
    R: Send,
    TP: TransParams,
{
}

/// Helper macro that assists with writing correct implementations of [`SendDesc::write_options`].
///
/// ## Example
///
/// ```
/// # use async_coap::uri::RelRefBuf;
/// # use std::marker::PhantomData;
/// # use async_coap::send_desc::SendDesc;
/// # use async_coap::prelude::*;
/// # use async_coap::write_options;
/// # use async_coap::{InboundContext, Error, message::MessageWrite};
/// # use std::ops::Bound;
/// # pub struct WriteOptionsExample<IC>(PhantomData<IC>);
/// # impl<IC: InboundContext> SendDesc<IC, ()> for WriteOptionsExample<IC> {
/// #
/// fn write_options(
///     &self,
///     msg: &mut dyn OptionInsert,
///     socket_addr: &IC::SocketAddr,
///     start: Bound<OptionNumber>,
///     end: Bound<OptionNumber>,
/// ) -> Result<(), Error> {
///     write_options!((msg, socket_addr, start, end) {
///         // Note that the options **MUST** be listed **in numerical order**,
///         // otherwise the behavior will be undefined!
///         URI_HOST => Some("example.com").into_iter(),
///         URI_PORT => Some(1234).into_iter(),
///         URI_PATH => vec!["a","b","c"].into_iter(),
///     })
/// }
/// #
/// #    fn write_payload(&self,msg: &mut dyn MessageWrite, socket_addr: &IC::SocketAddr) -> Result<(), Error> {
/// #        Ok(())
/// #    }
/// #    fn handler(&mut self,context: Result<&IC, Error>) -> Result<ResponseStatus<()>, Error> {
/// #        context.map(|_| ResponseStatus::Done(()))
/// #    }
/// # }
/// ```
#[macro_export]
macro_rules! write_options {
    (($msg:expr, $socket_addr:expr, $start:expr, $end:expr, $inner:expr) { $($key:expr => $viter:expr),* }) => {{
        let mut start = $start;
        let end = $end;
        let inner = &$inner;
        let msg = $msg;
        let socket_addr = $socket_addr;
        #[allow(unused)]
        use $crate::option::*;
        #[allow(unused)]
        use std::iter::once;

        $( write_options!(_internal $key, $viter, start, end, msg, socket_addr, inner); )*

        inner.write_options(msg, socket_addr, start, end)
    }};

    (($msg:expr, $socket_addr:expr, $start:expr, $end:expr) { $($key:expr => $viter:expr),* }) => {{
        let mut start = $start;
        let end = $end;
        let msg = $msg;
        let _socket_addr = $socket_addr;
        #[allow(unused)]
        use $crate::option::*;
        #[allow(unused)]
        use std::iter::once;

        $( write_options!(_internal $key, $viter, start, end, msg, socket_addr); )*

        let _ = start;

        Ok(())
    }};

    (($msg:ident, $socket_addr:ident, $start:ident, $end:ident, $inner:expr) { $($key:expr => $viter:expr),* ,}) => {
        write_options!(($msg,$socket_addr,$start,$end,$inner){$($key=>$viter),*})
    };

    (($msg:ident, $socket_addr:ident, $start:ident, $end:ident) { $($key:expr => $viter:expr),* ,}) => {
        write_options!(($msg,$socket_addr,$start,$end){$($key=>$viter),*})
    };

    ( _internal $key:expr, $viter:expr, $start:ident, $end:ident, $msg:ident, $socket_addr:ident, $inner:expr) => {{
        let key = $key;
        let mut value_iter = $viter.into_iter().peekable();

        if value_iter.peek().is_some()
            && match $start {
                Bound::Included(b) => b <= key.0,
                Bound::Excluded(b) => b < key.0,
                Bound::Unbounded => true,
            }
        {
            if match $end {
                Bound::Included(b) => key.0 <= b,
                Bound::Excluded(b) => key.0 < b,
                Bound::Unbounded => true,
            } {
                $inner.write_options($msg, $socket_addr, $start, Bound::Included(key.0))?;
                for value in value_iter {
                    $msg.insert_option(key, value)?;
                }
                $start = Bound::Excluded(key.0)
            }
        }
    }};

    ( _internal $key:expr, $viter:expr, $start:ident, $end:ident, $msg:ident, $socket_addr:ident) => {{
        let key = $key;
        let mut value_iter = $viter.into_iter().peekable();

        if value_iter.peek().is_some()
            && match $start {
                Bound::Included(b) => b <= key.0,
                Bound::Excluded(b) => b < key.0,
                Bound::Unbounded => true,
            }
        {
            if match $end {
                Bound::Included(b) => key.0 <= b,
                Bound::Excluded(b) => key.0 < b,
                Bound::Unbounded => true,
            } {
                for value in value_iter {
                    $msg.insert_option(key, value)?;
                }
                $start = Bound::Excluded(key.0)
            }
        }
    }};
}

/// Helper macro that provides pass-thru implementations of the timing-related methods
/// of a [`SendDesc`].
///
/// This macro takes a single argument: the name of the member variable to pass along
/// the call to.
#[doc(hidden)]
#[macro_export]
macro_rules! send_desc_passthru_timing {
    ($inner:tt) => {
        fn delay_to_retransmit(&self, retransmits_sent: u32) -> Option<::core::time::Duration> {
            self.$inner.delay_to_retransmit(retransmits_sent)
        }
        fn delay_to_restart(&self) -> Option<::core::time::Duration> {
            self.$inner.delay_to_restart()
        }
        fn max_rtt(&self) -> ::core::time::Duration {
            self.$inner.max_rtt()
        }
        fn transmit_wait_duration(&self) -> ::core::time::Duration {
            self.$inner.transmit_wait_duration()
        }
    }
}

/// Helper macro that provides pass-thru implementation of [`SendDesc::write_options`].
///
/// This macro takes a single argument: the name of the member variable to pass along
/// the call to.
#[doc(hidden)]
#[macro_export]
macro_rules! send_desc_passthru_options {
    ($inner:tt) => {
        fn write_options(
            &self,
            msg: &mut dyn OptionInsert,
            socket_addr: &IC::SocketAddr,
            start: Bound<OptionNumber>,
            end: Bound<OptionNumber>,
        ) -> Result<(), Error> {
            self.$inner.write_options(msg, socket_addr, start, end)
        }
    }
}

/// Helper macro that provides pass-thru implementations of [`SendDesc::handler`] and
/// [`SendDesc::supports_option`].
///
/// This macro takes a single argument: the name of the member variable to pass along
/// the call to.
#[doc(hidden)]
#[macro_export]
macro_rules! send_desc_passthru_handler {
    ($inner:tt, $rt:ty) => {
        fn supports_option(&self, option: OptionNumber) -> bool {
            self.$inner.supports_option(option)
        }
        fn handler(&mut self, context: Result<&IC, Error>) -> Result<ResponseStatus<$rt>, Error> {
            self.$inner.handler(context)
        }
    };

    ($inner:tt) => {
        send_desc_passthru_handler!($inner, ());
    }
}

/// Helper macro that provides pass-thru implementation of [`SendDesc::supports_option`].
///
/// This macro takes a single argument: the name of the member variable to pass along
/// the call to.
#[doc(hidden)]
#[macro_export]
macro_rules! send_desc_passthru_supports_option {
    ($inner:tt) => {
        fn supports_option(&self, option: OptionNumber) -> bool {
            self.$inner.supports_option(option)
        }
    }
}

/// Helper macro that provides pass-thru implementation of [`SendDesc::write_payload`].
///
/// This macro takes a single argument: the name of the member variable to pass along
/// the call to.
#[doc(hidden)]
#[macro_export]
macro_rules! send_desc_passthru_payload {
    ($inner:tt) => {
        fn write_payload(
            &self,
            msg: &mut dyn MessageWrite,
            socket_addr: &IC::SocketAddr,
        ) -> Result<(), Error> {
            self.$inner.write_payload(msg, socket_addr)
        }
    }
}
