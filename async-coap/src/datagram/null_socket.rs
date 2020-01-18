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
use futures::task::{Context, Poll};
use std::fmt::{Debug, Display, Formatter};
use std::pin::Pin;

/// Simplified "SocketAddr" for [`NullSocket`]. Does nothing.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NullSocketAddr;

impl Display for NullSocketAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        <Self as Debug>::fmt(self, f)
    }
}

impl SocketAddrExt for NullSocketAddr {
    fn is_multicast(&self) -> bool {
        return false;
    }

    fn port(&self) -> u16 {
        0
    }

    fn conforming_to(&self, _local: Self) -> Option<Self> {
        Some(*self)
    }

    fn addr_to_string(&self) -> String {
        "null".to_string()
    }
}

impl ToSocketAddrs for NullSocketAddr {
    type Iter = std::option::IntoIter<Self::SocketAddr>;
    type SocketAddr = Self;
    type Error = super::Error;

    fn to_socket_addrs(&self) -> Result<Self::Iter, Self::Error> {
        Ok(Some(*self).into_iter())
    }
}

impl MulticastSocket for NullSocket {
    type IpAddr = String;

    fn join_multicast<A>(&self, _addr: A) -> Result<(), Self::Error>
    where
        A: std::convert::Into<Self::IpAddr>,
    {
        Ok(())
    }

    fn leave_multicast<A>(&self, _addr: A) -> Result<(), Self::Error>
    where
        A: std::convert::Into<Self::IpAddr>,
    {
        Ok(())
    }
}

/// An instance of [`AsyncDatagramSocket`] that implements a simple null interface, where
/// all packets that are sent are discarded.
#[derive(Debug)]
pub struct NullSocket;

impl NullSocket {
    /// Creates a new instance of [`NullSocket`].
    pub fn new() -> NullSocket {
        NullSocket
    }
}

impl Unpin for NullSocket {}

impl AsyncDatagramSocket for NullSocket {}

impl DatagramSocketTypes for NullSocket {
    type SocketAddr = NullSocketAddr;
    type Error = super::Error;

    fn local_addr(&self) -> Result<Self::SocketAddr, Self::Error> {
        Ok(NullSocketAddr)
    }

    fn lookup_host(
        _host: &str,
        _port: u16,
    ) -> Result<std::vec::IntoIter<Self::SocketAddr>, Self::Error>
    where
        Self: Sized,
    {
        Ok(vec![NullSocketAddr].into_iter())
    }
}

impl AsyncSendTo for NullSocket {
    fn poll_send_to<B>(
        self: Pin<&Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
        _addr: B,
    ) -> Poll<Result<usize, Self::Error>>
    where
        B: super::ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::Error>,
    {
        Poll::Ready(Ok(buf.len()))
    }
}

impl AsyncRecvFrom for NullSocket {
    fn poll_recv_from(
        self: Pin<&Self>,
        _cx: &mut Context<'_>,
        _buf: &mut [u8],
    ) -> Poll<Result<(usize, Self::SocketAddr, Option<Self::SocketAddr>), Self::Error>> {
        Poll::Pending
    }
}
