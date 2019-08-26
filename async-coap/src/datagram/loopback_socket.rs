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
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::lock::Mutex;
use futures::prelude::*;
use futures::task::Context;
use futures::Poll;
use std::fmt::{Debug, Display, Formatter};
use std::pin::Pin;

/// Simplified "SocketAddr" for [`LoopbackSocket`]. Allows for two different types of addresses:
/// Unicast addresses and Multicast addresses.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LoopbackSocketAddr {
    /// "Unicast" Loopback Socket Address.
    Unicast,

    /// "Multicast" Loopback Socket Address.
    Multicast,
}

impl Display for LoopbackSocketAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        <Self as Debug>::fmt(self, f)
    }
}

impl SocketAddrExt for LoopbackSocketAddr {
    fn is_multicast(&self) -> bool {
        match self {
            LoopbackSocketAddr::Unicast => false,
            LoopbackSocketAddr::Multicast => true,
        }
    }

    fn port(&self) -> u16 {
        0
    }

    fn conforming_to(&self, _local: Self) -> Option<Self> {
        Some(*self)
    }

    fn addr_to_string(&self) -> String {
        match self {
            LoopbackSocketAddr::Unicast => "localhost",
            LoopbackSocketAddr::Multicast => "broadcasthost",
        }
        .to_string()
    }
}

impl ToSocketAddrs for LoopbackSocketAddr {
    type Iter = std::option::IntoIter<Self::SocketAddr>;
    type SocketAddr = Self;
    type Error = super::Error;

    fn to_socket_addrs(&self) -> Result<Self::Iter, Self::Error> {
        Ok(Some(*self).into_iter())
    }
}

/// An instance of [`AsyncDatagramSocket`] that implements a simple loopback interface, where
/// all packets that are sent are looped back to the input.
#[derive(Debug)]
pub struct LoopbackSocket {
    // Message is (packet_bytes, dest_addr)
    sender: Sender<(Vec<u8>, LoopbackSocketAddr)>,
    receiver: futures::lock::Mutex<Receiver<(Vec<u8>, LoopbackSocketAddr)>>,
}

impl LoopbackSocket {
    /// Creates a new instance of [`LoopbackSocket`].
    pub fn new() -> LoopbackSocket {
        let (sender, receiver) = channel(3);
        LoopbackSocket {
            sender,
            receiver: Mutex::new(receiver),
        }
    }
}

impl Unpin for LoopbackSocket {}

impl AsyncDatagramSocket for LoopbackSocket {}

impl DatagramSocketTypes for LoopbackSocket {
    type SocketAddr = LoopbackSocketAddr;
    type Error = super::Error;

    fn local_addr(&self) -> Result<Self::SocketAddr, Self::Error> {
        Ok(LoopbackSocketAddr::Unicast)
    }

    fn lookup_host(
        host: &str,
        _port: u16,
    ) -> Result<std::vec::IntoIter<Self::SocketAddr>, Self::Error>
    where
        Self: Sized,
    {
        if host == ALL_COAP_DEVICES_HOSTNAME {
            Ok(vec![LoopbackSocketAddr::Multicast].into_iter())
        } else {
            Ok(vec![LoopbackSocketAddr::Unicast].into_iter())
        }
    }
}

impl AsyncSendTo for LoopbackSocket {
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
            let mut sender = self.get_ref().sender.clone();
            match sender.poll_ready(cx) {
                Poll::Ready(Ok(())) => match sender.start_send((buf.to_vec(), addr)) {
                    Ok(()) => Poll::Ready(Ok(buf.len())),
                    Err(e) => {
                        if e.is_full() {
                            Poll::Pending
                        } else {
                            Poll::Ready(Err(Error::IOError))
                        }
                    }
                },
                Poll::Ready(Err(_)) => Poll::Ready(Err(Error::IOError)),
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(Err(Error::HostNotFound))
        }
    }
}

impl AsyncRecvFrom for LoopbackSocket {
    fn poll_recv_from(
        self: Pin<&Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<(usize, Self::SocketAddr, Option<Self::SocketAddr>), Self::Error>> {
        let mut receiver_lock_future = self.get_ref().receiver.lock();
        let receiver_lock_future = Pin::new(&mut receiver_lock_future);

        if let Poll::Ready(mut receiver_guard) = receiver_lock_future.poll(cx) {
            let receiver: &mut Receiver<(Vec<u8>, LoopbackSocketAddr)> = &mut receiver_guard;
            match receiver.poll_next_unpin(cx) {
                Poll::Ready(Some((packet, addr))) => {
                    let len = packet.len();
                    if buf.len() >= len {
                        buf[..len].copy_from_slice(&packet);
                        Poll::Ready(Ok((len, self.local_addr().unwrap(), Some(addr))))
                    } else {
                        Poll::Ready(Err(Error::IOError))
                    }
                }
                Poll::Ready(None) => Poll::Ready(Err(Error::IOError)),
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Pending
        }
    }
}

impl MulticastSocket for LoopbackSocket {
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
