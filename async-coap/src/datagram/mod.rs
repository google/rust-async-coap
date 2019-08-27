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

//! Generic, datagram-based CoAP backend, with associated socket abstractions.
//!
//! The actual backend is [`DatagramLocalEndpoint`]. It uses an asynchronous datagram
//! socket that implements the trait [`AsyncDatagramSocket`] and all of its required dependency
//! traits ([`DatagramSocketTypes`], [`AsyncSendTo`], [`AsyncRecvFrom`], [`MulticastSocket`],
//! [`Send`], and [`Sync`]).
//!
//! [`DatagramLocalEndpoint`]: datagram::DatagramLocalEndpoint
//! [`DatagramSocketTypes`]: datagram::DatagramSocketTypes
//! [`AsyncDatagramSocket`]: datagram::AsyncDatagramSocket
//! [`AsyncSendTo`]: datagram::AsyncSendTo
//! [`AsyncRecvFrom`]: datagram::AsyncRecvFrom
//! [`AsyncRecvFrom`]: datagram::AsyncRecvFrom
//! [`MulticastSocket`]: datagram::MulticastSocket
//!
use super::*;

mod async_socket;
pub use async_socket::{
    AsyncDatagramSocket, AsyncRecvFrom, AsyncSendTo, DatagramSocketTypes, MulticastSocket,
    RecvFromFuture, SendToFuture,
};

mod allow_udp_socket;
pub use allow_udp_socket::AllowStdUdpSocket;

mod loopback_socket;
pub use loopback_socket::LoopbackSocket;
pub use loopback_socket::LoopbackSocketAddr;

mod null_socket;
pub use null_socket::NullSocket;
pub use null_socket::NullSocketAddr;

mod response_tracker;
use response_tracker::*;

mod send_future;
use send_future::*;

mod remote_endpoint;
pub use remote_endpoint::*;

mod local_endpoint;
pub use local_endpoint::*;

mod inbound_context;
pub use inbound_context::*;
