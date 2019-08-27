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
use futures::task::Context;
use futures::Poll;
use futures_timer::Delay;
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket};
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::Duration;

/// A wrapper around [`std::net::UdpSocket`] that implements [`AsyncDatagramSocket`].
///
/// This can be used to allow the standard rust [`UdpSocket`] (which doesn't provide an
/// asynchronous API) to be used in an asynchronous fashion, similar to [`futures-preview::io::AllowStdio`].
///
/// Note that by default this wrapper will block execution whenever one of the `poll` methods is
/// called (An exception is any instance created with [`AllowStdUdpSocket::bind`]).
/// You can fake true asynchronous behavior by calling `set_nonblocking(true)` or
/// `set_read_timeout()`. In the case where a UDP message is not received when polled (or it
/// times out waiting for a response), a `futures_timer::Delay` is used to schedule an appropriate
/// duration (set via `set_async_poll_interval()`) after which it can try again. This is obviously
/// sub-optimal, but that's the best that can be offered without becoming intrusive.
///
/// As such, it is really intended to be a convenience stop-gap to get things up and running
/// quickly. For production code, you would want to use a [`AsyncDatagramSocket`] wrapper around
/// something truly asynchronous, like [`tokio-udp::UdpSocket`].
#[derive(Debug)]
pub struct AllowStdUdpSocket(UdpSocket, Mutex<Option<Delay>>, Option<Duration>);

impl AllowStdUdpSocket {
    /// The default interval between polling attempts.
    ///
    /// This value can be overridden by [`AllowStdUdpSocket::set_async_poll_interval`].
    const DEFAULT_ASYNC_POLL_INTERVAL: Duration = Duration::from_millis(30);

    /// Upgrades the given [`std::net::UdpSocket`] to an instance of [`AllowStdUdpSocket`].
    ///
    /// Note that no operations are performed on `udp_socket` by this method. It is recommended
    /// that you call [`std::net::UdpSocket::set_nonblocking`] on the socket before using this
    /// method. See the documentation for [`AllowStdUdpSocket`] for more information.
    pub fn from_std(udp_socket: UdpSocket) -> AllowStdUdpSocket {
        AllowStdUdpSocket(
            udp_socket,
            Mutex::new(None),
            Some(Self::DEFAULT_ASYNC_POLL_INTERVAL),
        )
    }

    /// Analog of [`std::net::UdpSocket::bind`] for [`AllowStdUdpSocket`].
    ///
    /// If `addr` is successfully resolved, the underlying `UdpSocket` will already be
    /// configured in a non-blocking operation mode.
    pub fn bind<A>(addr: A) -> std::io::Result<AllowStdUdpSocket>
    where
        A: std::net::ToSocketAddrs,
    {
        let udp_socket = UdpSocket::bind(addr)?;
        udp_socket.set_nonblocking(true).unwrap();
        Ok(AllowStdUdpSocket::from_std(udp_socket))
    }

    /// Changes the async poll interval for this socket, returning the previous value.
    ///
    /// A value of `None` indicates that no timed polling should be performed.
    ///
    /// The default value is
    /// [`Some(DEFAULT_ASYNC_POLL_INTERVAL)`][AllowStdUdpSocket::DEFAULT_ASYNC_POLL_INTERVAL],
    /// or 30ms.
    pub fn set_async_poll_interval(&mut self, mut dur: Option<Duration>) -> Option<Duration> {
        std::mem::swap(&mut self.2, &mut dur);
        dur
    }

    fn wait_for_data(self: &Self, cx: &mut futures::task::Context<'_>) {
        let delay;
        if let Some(d) = self.2 {
            let mut lock = self.1.lock().expect("Lock failed");
            let opt_mut: &mut Option<Delay> = &mut lock;
            if opt_mut.is_none() {
                *opt_mut = Some(Delay::new(d));
                delay = opt_mut.as_mut().unwrap();
            } else {
                delay = opt_mut.as_mut().unwrap();
                delay.reset(d);
            }

            let _ = Pin::new(delay).poll(cx);
        }
    }
}

impl Unpin for AllowStdUdpSocket {}

impl AsyncDatagramSocket for AllowStdUdpSocket {}

impl DatagramSocketTypes for AllowStdUdpSocket {
    type SocketAddr = std::net::SocketAddr;
    type Error = std::io::Error;

    fn local_addr(&self) -> Result<Self::SocketAddr, Self::Error> {
        self.0.local_addr()
    }

    fn lookup_host(
        host: &str,
        port: u16,
    ) -> Result<std::vec::IntoIter<Self::SocketAddr>, Self::Error>
    where
        Self: Sized,
    {
        if host == ALL_COAP_DEVICES_HOSTNAME {
            Ok(vec![
                SocketAddr::V6(SocketAddrV6::new(
                    "FF02:0:0:0:0:0:0:FD".parse().unwrap(),
                    port,
                    0,
                    0,
                )),
                SocketAddr::V4(SocketAddrV4::new("224.0.1.187".parse().unwrap(), port)),
                SocketAddr::V6(SocketAddrV6::new(
                    "FF03:0:0:0:0:0:0:FD".parse().unwrap(),
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

impl AsyncSendTo for AllowStdUdpSocket {
    fn poll_send_to<B>(
        self: Pin<&Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
        addr: B,
    ) -> Poll<Result<usize, Self::Error>>
    where
        B: super::ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::Error>,
    {
        if let Some(addr) = addr.to_socket_addrs()?.next() {
            match self.get_ref().0.send_to(buf, addr) {
                Ok(written) => Poll::Ready(Ok(written)),
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        self.get_ref().wait_for_data(cx);
                        Poll::Pending
                    } else {
                        Poll::Ready(Err(e))
                    }
                }
            }
        } else {
            Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::AddrNotAvailable,
                "Address lookup failed",
            )))
        }
    }
}

impl AsyncRecvFrom for AllowStdUdpSocket {
    fn poll_recv_from(
        self: Pin<&Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<(usize, Self::SocketAddr, Option<Self::SocketAddr>), Self::Error>> {
        match self.0.recv_from(buf) {
            Ok((size, from)) => Poll::Ready(Ok((size, from, None))),
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
                    self.wait_for_data(cx);
                    Poll::Pending
                }
                _ => Poll::Ready(Err(e)),
            },
        }
    }
}

impl Deref for AllowStdUdpSocket {
    type Target = UdpSocket;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MulticastSocket for AllowStdUdpSocket {
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
