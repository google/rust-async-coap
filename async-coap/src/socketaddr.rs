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
use std::hash::Hash;

/// A flavor of `std::net::ToSocketAddrs` that allows the implementation of
/// `SocketAddr` to be replaced.
///
/// This is necessary to enable support for things like
/// CoAP-over-SMS, where socket addresses are telephone numbers.
pub trait ToSocketAddrs {
    /// Analogous to [`std::net::ToSocketAddrs::Iter`]
    type Iter: Iterator<Item = Self::SocketAddr>;

    /// The `SocketAddr` type returned by the above iterator.
    type SocketAddr: SocketAddrExt + Copy;

    /// The error type to use for errors while resolving.
    type Error: core::fmt::Debug;

    /// Analogous to [`std::net::ToSocketAddrs::to_socket_addrs`]
    fn to_socket_addrs(&self) -> Result<Self::Iter, Self::Error>;
}

/// Blanket implementation of `ToSocketAddrs` for all implementations of `std::net::ToSocketAddrs`.
#[cfg(feature = "std")]
impl<T, I> ToSocketAddrs for T
where
    T: std::net::ToSocketAddrs<Iter = I>,
    I: Iterator<Item = std::net::SocketAddr>,
{
    type Iter = I;
    type SocketAddr = std::net::SocketAddr;
    type Error = std::io::Error;

    fn to_socket_addrs(&self) -> Result<Self::Iter, Self::Error> {
        std::net::ToSocketAddrs::to_socket_addrs(self)
    }
}

/// Extension trait for `SocketAddr` types that allows the local endpoint get the information
/// it needs.
pub trait SocketAddrExt:
    Sized + ToSocketAddrs + Copy + core::fmt::Display + core::fmt::Debug + Send + Eq + Hash
{
    /// Determines if the address in this `SocketAddr` is a multicast/broadcast address or not.
    fn is_multicast(&self) -> bool;

    /// Returns the port number for this socket.
    ///
    /// A value of zero indicates no specific value.
    fn port(&self) -> u16;

    /// Returns a version of this socket address that conforms to the address type of `local`,
    /// or `None` if such a conversion is not possible.
    ///
    /// This method is useful in mixed ipv6/ipv4 environments.
    #[allow(unused_variables)]
    fn conforming_to(&self, local: Self) -> Option<Self> {
        Some(*self)
    }

    /// Renders the address portion to a string.
    fn addr_to_string(&self) -> String;

    /// Creates a URI from this `SocketAddr` using the given scheme.
    fn as_uri_buf(&self, scheme: &str) -> UriBuf {
        UriBuf::from_scheme_host_port(scheme, self.addr_to_string(), Some(self.port()))
    }
}

#[cfg(feature = "std")]
impl SocketAddrExt for std::net::SocketAddr {
    fn is_multicast(&self) -> bool {
        self.ip().is_multicast()
            || if let std::net::IpAddr::V4(addr) = self.ip() {
                addr.is_broadcast()
            } else {
                false
            }
    }

    fn port(&self) -> u16 {
        self.port()
    }

    fn conforming_to(&self, local: Self) -> Option<Self> {
        if self.is_ipv6() == local.is_ipv6() {
            Some(*self)
        } else if let std::net::SocketAddr::V4(v4) = self {
            Some((v4.ip().to_ipv6_mapped(), v4.port()).into())
        } else {
            None
        }
    }

    fn addr_to_string(&self) -> String {
        self.ip().to_string()
    }

    fn as_uri_buf(&self, scheme: &str) -> UriBuf {
        UriBuf::from_scheme_host_port(scheme, self.ip().to_string(), Some(self.port()))
    }
}
