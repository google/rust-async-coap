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

use async_coap::datagram::{
    AsyncDatagramSocket, AsyncRecvFrom, AsyncSendTo, DatagramSocketTypes, MulticastSocket,
};
use futures::task::Context;
use futures::{ready, Poll};
use mio::net::UdpSocket;
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs};
use std::ops::Deref;
use std::pin::Pin;
use tokio_net::util::PollEvented;

/// An asynchronous [`AsyncDatagramSocket`] wrapper around [`std::net::UdpSocket`] that
/// uses [Tokio][] for the event loop.
///
/// This type differs from [`AllowUdpSocket`] in that it provides a real asynchronous,
/// event-driven interface instead of faking one.
///
/// In order to use this type, you must be using [Tokio][] for your event loop.
///
/// [`AllowUdpSocket`]: async-coap::datagram::AllowUdpSocket
/// [Tokio]: https://tokio.rs/
#[derive(Debug)]
pub struct TokioAsyncUdpSocket(PollEvented<UdpSocket>);

impl TokioAsyncUdpSocket {
    /// Analog of [`std::net::UdpSocket::bind`] for [`TokioAsyncUdpSocket`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use async_coap_tokio::TokioAsyncUdpSocket;
    /// # fn main() -> std::io::Result<()> {
    /// let async_socket = TokioAsyncUdpSocket::bind("[::]:0")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn bind<A>(addr: A) -> std::io::Result<TokioAsyncUdpSocket>
    where
        A: std::net::ToSocketAddrs,
    {
        let udp_socket = std::net::UdpSocket::bind(addr)?;
        Ok(Self::from_std(udp_socket))
    }

    /// Upgrades a [`std::net::UdpSocket`] by wrapping it in a [`TokioAsyncUdpSocket`].
    pub fn from_std(udp_socket: std::net::UdpSocket) -> TokioAsyncUdpSocket {
        udp_socket.set_nonblocking(true).unwrap();
        Self::from_mio(mio::net::UdpSocket::from_socket(udp_socket).expect("Unbound socket"))
    }

    /// Wraps a [`mio::net::UdpSocket`] instance with a [`TokioAsyncUdpSocket`].
    pub(crate) fn from_mio(udp_socket: UdpSocket) -> TokioAsyncUdpSocket {
        TokioAsyncUdpSocket(PollEvented::new(udp_socket))
    }
}

impl Unpin for TokioAsyncUdpSocket {}

impl AsyncDatagramSocket for TokioAsyncUdpSocket {}

impl DatagramSocketTypes for TokioAsyncUdpSocket {
    type SocketAddr = std::net::SocketAddr;
    type Error = std::io::Error;

    fn local_addr(&self) -> Result<Self::SocketAddr, Self::Error> {
        self.0.get_ref().local_addr()
    }

    fn lookup_host(
        host: &str,
        port: u16,
    ) -> Result<std::vec::IntoIter<Self::SocketAddr>, Self::Error>
    where
        Self: Sized,
    {
        use async_coap::{
            ALL_COAP_DEVICES_HOSTNAME, ALL_COAP_DEVICES_V4, ALL_COAP_DEVICES_V6_LL,
            ALL_COAP_DEVICES_V6_RL,
        };

        if host == ALL_COAP_DEVICES_HOSTNAME {
            Ok(vec![
                SocketAddr::V6(SocketAddrV6::new(
                    ALL_COAP_DEVICES_V6_LL.parse().unwrap(),
                    port,
                    0,
                    0,
                )),
                SocketAddr::V4(SocketAddrV4::new(
                    ALL_COAP_DEVICES_V4.parse().unwrap(),
                    port,
                )),
                SocketAddr::V6(SocketAddrV6::new(
                    ALL_COAP_DEVICES_V6_RL.parse().unwrap(),
                    port,
                    0,
                    0,
                )),
            ]
            .into_iter())
        } else {
            (host, port).to_socket_addrs()
        }
    }
}

impl AsyncSendTo for TokioAsyncUdpSocket {
    fn poll_send_to<B>(
        self: Pin<&Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
        addr: B,
    ) -> Poll<Result<usize, Self::Error>>
    where
        B: async_coap::ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::Error>,
    {
        // We are ignoring the return value of `poll_write_ready` here because
        // it will pretty much always lie to us the first time it is called.
        // Instead, since we know that the underlying socket is configured to
        // be non-blocking, we trust it instead.
        let _ = self.0.poll_write_ready(cx);

        if let Some(addr) = addr.to_socket_addrs()?.next() {
            match self.0.get_ref().send_to(buf, &addr) {
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    self.0.clear_write_ready(cx)?;
                    Poll::Pending
                }
                x => Poll::Ready(x),
            }
        } else {
            Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::AddrNotAvailable,
                "Address lookup failed",
            )))
        }
    }
}

impl AsyncRecvFrom for TokioAsyncUdpSocket {
    fn poll_recv_from(
        self: Pin<&Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<(usize, Self::SocketAddr, Option<Self::SocketAddr>), Self::Error>> {
        ready!(self.0.poll_read_ready(cx, mio::Ready::readable()))?;

        match self.0.get_ref().recv_from(buf) {
            Ok((size, from)) => Poll::Ready(Ok((size, from, None))),
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
                    self.0.clear_read_ready(cx, mio::Ready::readable())?;
                    Poll::Pending
                }
                _ => Poll::Ready(Err(e)),
            },
        }
    }
}

impl Deref for TokioAsyncUdpSocket {
    type Target = UdpSocket;

    fn deref(&self) -> &Self::Target {
        self.0.get_ref()
    }
}

impl MulticastSocket for TokioAsyncUdpSocket {
    type IpAddr = std::net::IpAddr;

    fn join_multicast<A>(&self, addr: A) -> Result<(), Self::Error>
    where
        A: std::convert::Into<Self::IpAddr>,
    {
        use std::net::IpAddr;
        let local_sockaddr = self.local_addr()?;
        match addr.into() {
            IpAddr::V4(addr) => {
                let local_addr = local_sockaddr.ip();
                if let IpAddr::V4(local_addr) = local_addr {
                    self.join_multicast_v4(&addr, &local_addr)
                } else if let SocketAddr::V6(local_sockaddr) = local_sockaddr {
                    self.join_multicast_v6(&addr.to_ipv6_mapped(), local_sockaddr.scope_id())
                } else {
                    unreachable!();
                }
            }
            IpAddr::V6(addr) => {
                if let SocketAddr::V6(local_sockaddr) = local_sockaddr {
                    self.join_multicast_v6(&addr, local_sockaddr.scope_id())
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "multicast-addr/local-addr mismatch",
                    ))
                }
            }
        }
    }

    fn leave_multicast<A>(&self, addr: A) -> Result<(), Self::Error>
    where
        A: std::convert::Into<Self::IpAddr>,
    {
        use std::net::IpAddr;
        let local_sockaddr = self.local_addr()?;
        match addr.into() {
            IpAddr::V4(addr) => {
                let local_addr = local_sockaddr.ip();
                if let IpAddr::V4(local_addr) = local_addr {
                    self.leave_multicast_v4(&addr, &local_addr)
                } else if let SocketAddr::V6(local_sockaddr) = local_sockaddr {
                    self.leave_multicast_v6(&addr.to_ipv6_mapped(), local_sockaddr.scope_id())
                } else {
                    unreachable!();
                }
            }
            IpAddr::V6(addr) => {
                if let SocketAddr::V6(local_sockaddr) = local_sockaddr {
                    self.leave_multicast_v6(&addr, local_sockaddr.scope_id())
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "multicast-addr/local-addr mismatch",
                    ))
                }
            }
        }
    }
}
