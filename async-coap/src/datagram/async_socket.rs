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
use futures::prelude::*;
use futures::task::{Context, Poll};
use std::pin::Pin;

/// A trait for asynchronous datagram sockets.
///
/// This is an empty convenience trait that requires several additional traits to be implemented:
/// [`DatagramSocketTypes`], [`AsyncSendTo`], [`AsyncRecvFrom`], [`MulticastSocket`],
/// and [`Send`]+[`Sync`].
///
/// Implementations of this trait can be used with [`DatagramLocalEndpoint`].
pub trait AsyncDatagramSocket:
    DatagramSocketTypes + AsyncSendTo + AsyncRecvFrom + MulticastSocket + Send + Sync
{
}

/// Trait implemented by a "socket" that describes the underlying `SocketAddr` and socket error
/// types as associated types.
pub trait DatagramSocketTypes: Unpin {
    /// The "`SocketAddr`" type used by this "socket".  Typically [`std::net::SocketAddr`].
    type SocketAddr: SocketAddrExt
        + core::fmt::Display
        + core::fmt::Debug
        + std::string::ToString
        + ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::Error>
        + Send
        + Unpin
        + Copy;

    /// The error type for errors emitted from this socket. Typically [`std::io::Error`].
    type Error: std::fmt::Display + std::fmt::Debug;

    /// Returns the local `SocketAddr` of this "socket".
    fn local_addr(&self) -> Result<Self::SocketAddr, Self::Error>;

    /// Performs a blocking hostname lookup.
    fn lookup_host(
        host: &str,
        port: u16,
    ) -> Result<std::vec::IntoIter<Self::SocketAddr>, Self::Error>
    where
        Self: Sized;
}

/// Trait for providing `sent_to` functionality for asynchronous, datagram-based sockets.
pub trait AsyncSendTo: DatagramSocketTypes {
    /// A non-blocking[^1], `poll_*` version of `std::net::UdpSocket::send_to`.
    ///
    /// [^1]: Note that while the spirit of this method intends for it to be non-blocking,
    ///       [`AllowStdUdpSocket`] can block execution depending on the implementation details
    ///       of the underlying [`std::net::UdpSocket`].
    fn poll_send_to<B>(
        self: Pin<&Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
        addr: B,
    ) -> Poll<Result<usize, Self::Error>>
    where
        B: super::ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::Error>;

    /// Returns a future that uses [`AsyncSendTo::poll_send_to`].
    fn send_to<'a, 'b, B>(&'a self, buf: &'b [u8], addr: B) -> SendToFuture<'a, 'b, Self>
    where
        B: super::ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::Error>,
    {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        SendToFuture {
            socket: self,
            buffer: buf,
            addr: addr,
        }
    }
}

/// Future returned from [`AsyncSendTo::send_to`].
#[derive(Debug)]
pub struct SendToFuture<'a, 'b, T>
where
    T: DatagramSocketTypes + AsyncSendTo + ?Sized,
{
    socket: &'a T,
    buffer: &'b [u8],
    addr: T::SocketAddr,
}

impl<'a, 'b, T> SendToFuture<'a, 'b, T>
where
    T: DatagramSocketTypes + AsyncSendTo + ?Sized,
{
    fn poll_unpin(
        self: &mut Self,
        cx: &mut futures::task::Context<'_>,
    ) -> futures::task::Poll<Result<usize, T::Error>> {
        Pin::new(self.socket).poll_send_to(cx, self.buffer, self.addr.clone())
    }
}

impl<'a, 'b, T> Future for SendToFuture<'a, 'b, T>
where
    T: DatagramSocketTypes + AsyncSendTo + ?Sized,
{
    type Output = Result<usize, T::Error>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut futures::task::Context<'_>,
    ) -> futures::task::Poll<Self::Output> {
        self.get_mut().poll_unpin(cx)
    }
}

/// Future returned from [`AsyncRecvFrom::recv_from`].
#[derive(Debug)]
pub struct RecvFromFuture<'a, 'b, T: AsyncRecvFrom + ?Sized> {
    socket: &'a T,
    buffer: &'b mut [u8],
}

impl<'a, 'b, T: AsyncRecvFrom + ?Sized + Unpin> RecvFromFuture<'a, 'b, T> {
    fn poll_unpin(
        self: &mut Self,
        cx: &mut futures::task::Context<'_>,
    ) -> futures::task::Poll<Result<(usize, T::SocketAddr, Option<T::SocketAddr>), T::Error>> {
        Pin::new(self.socket).poll_recv_from(cx, self.buffer)
    }
}

impl<'a, 'b, T: AsyncRecvFrom + ?Sized> Future for RecvFromFuture<'a, 'b, T> {
    type Output = Result<(usize, T::SocketAddr, Option<T::SocketAddr>), T::Error>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut futures::task::Context<'_>,
    ) -> futures::task::Poll<Self::Output> {
        self.get_mut().poll_unpin(cx)
    }
}

/// Trait for providing `recv_from` functionality for asynchronous, datagram-based sockets.
///
/// The value returned on success is a tuple of the following:
///
/// ```
/// # use std::net::SocketAddr;
/// # fn ignore_this_line
/// #
/// (bytes_written: usize,
///  remote_socket_addr: SocketAddr,
///  local_socket_addr: Option<SocketAddr>)
/// #
/// # {} // ignore this line
/// ```
///
/// `local_socket_addr` indicates the local address that the packet was sent to, and may not be
/// supported. If this isn't supported, `local_socket_addr` will be set to `None`.
pub trait AsyncRecvFrom: DatagramSocketTypes {
    /// A non-blocking[^1], `poll_*` version of [`std::net::UdpSocket::recv_from`] that can
    /// optionally provide the destination (local) `SocketAddr`.
    ///
    /// If you need to receive a packet from within an async block, see
    /// [`AsyncRecvFrom::recv_from`], which returns a [`Future`][std::future::Future].
    ///
    /// [^1]: Note that while the spirit of this method intends for it to be non-blocking,
    ///       [`AllowStdUdpSocket`] can in fact block execution depending on the state of the
    ///       underlying [`std::net::UdpSocket`].
    fn poll_recv_from(
        self: Pin<&Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<(usize, Self::SocketAddr, Option<Self::SocketAddr>), Self::Error>>;

    /// Returns a future that uses [`poll_recv_from`][AsyncRecvFrom::poll_recv_from].
    fn recv_from<'a, 'b>(&'a self, buf: &'b mut [u8]) -> RecvFromFuture<'a, 'b, Self> {
        RecvFromFuture {
            socket: self,
            buffer: buf,
        }
    }
}

/// Trait that provides methods for joining/leaving multicast groups.
pub trait MulticastSocket: DatagramSocketTypes {
    /// The "address" type for this socket.
    ///
    /// Note that this is different than a `SocketAddr`, which also includes a port number.
    /// This is just the address.
    type IpAddr;

    /// Attempts to join the given multicast group.
    fn join_multicast<A>(&self, addr: A) -> Result<(), Self::Error>
    where
        A: std::convert::Into<Self::IpAddr>;

    /// Attempts to leave the given multicast group.
    fn leave_multicast<A>(&self, addr: A) -> Result<(), Self::Error>
    where
        A: std::convert::Into<Self::IpAddr>;
}
