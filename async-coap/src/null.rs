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

//! NULL CoAP backend
//!
//! This is a CoAP back end that does nothing. It is used primarily for testing.
use super::*;
use crate::message::NullMessageRead;
use crate::remote_endpoint::RemoteEndpoint;
use futures::future::BoxFuture;
use std::net::{IpAddr, Ipv4Addr};

/// Concrete instance of [`LocalEndpoint::RespondableInboundContext`] for [`NullLocalEndpoint`].
#[derive(Debug)]
pub struct NullRespondableInboundContext
where
    Self: Send + Sync;
impl RespondableInboundContext for NullRespondableInboundContext {
    fn is_multicast(&self) -> bool {
        false
    }

    fn is_fake(&self) -> bool {
        false
    }

    fn respond<F>(&self, _msg_gen: F) -> Result<(), Error>
    where
        F: Fn(&mut dyn MessageWrite) -> Result<(), Error>,
    {
        Ok(())
    }
}
impl InboundContext for NullRespondableInboundContext {
    type SocketAddr = std::net::SocketAddr;

    fn remote_socket_addr(&self) -> Self::SocketAddr {
        Self::SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
    }

    fn is_dupe(&self) -> bool {
        false
    }

    fn message(&self) -> &dyn MessageRead {
        &NullMessageRead
    }
}

/// Concrete instance of [`LocalEndpoint::InboundContext`] for [`NullLocalEndpoint`].
#[derive(Debug)]
pub struct NullInboundContext
where
    Self: Send + Sync;
impl InboundContext for NullInboundContext {
    type SocketAddr = std::net::SocketAddr;

    fn remote_socket_addr(&self) -> Self::SocketAddr {
        Self::SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
    }

    fn is_dupe(&self) -> bool {
        false
    }

    fn message(&self) -> &dyn MessageRead {
        &NullMessageRead
    }
}

/// Concrete instance of [`LocalEndpoint::RemoteEndpoint`] for [`NullLocalEndpoint`].
#[derive(Debug)]
pub struct NullRemoteEndpoint
where
    Self: Send + Sync;

impl RemoteEndpoint for NullRemoteEndpoint {
    type SocketAddr = std::net::SocketAddr;
    type InboundContext = NullInboundContext;

    fn scheme(&self) -> &'static str {
        "null"
    }

    fn uri(&self) -> UriBuf {
        uri!("null:///").to_owned()
    }

    fn send<'a, R, SD>(&'a self, _send_desc: SD) -> BoxFuture<'_, Result<R, Error>>
    where
        SD: SendDesc<Self::InboundContext, R>,
        R: Send + 'a,
    {
        futures::future::ready(Err(Error::ResponseTimeout)).boxed()
    }

    fn send_to<'a, R, SD, UF>(
        &'a self,
        _path: UF,
        _send_desc: SD,
    ) -> BoxFuture<'_, Result<R, Error>>
    where
        SD: SendDesc<Self::InboundContext, R> + 'a,
        R: Send + 'a,
        UF: AsRef<RelRef>,
    {
        futures::future::ready(Err(Error::ResponseTimeout)).boxed()
    }

    fn remove_host_option(&mut self) {}

    fn clone_using_rel_ref(&self, _uri: &RelRef) -> Self {
        NullRemoteEndpoint
    }
}

/// A dummy endpoint implementation that doesn't do anything. Useful for testing.
#[derive(Debug)]
pub struct NullLocalEndpoint
where
    Self: Send + Sync;

impl LocalEndpoint for NullLocalEndpoint {
    type SocketAddr = std::net::SocketAddr;
    type SocketError = std::io::Error;
    type DefaultTransParams = StandardCoapConstants;

    fn scheme(&self) -> &'static str {
        URI_SCHEME_NULL
    }

    fn default_port(&self) -> u16 {
        // Zero means ports are ignored.
        0
    }

    type RemoteEndpoint = NullRemoteEndpoint;

    fn remote_endpoint<S, H, P>(&self, _addr: S, _host: Option<H>, _path: P) -> Self::RemoteEndpoint
    where
        S: ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::SocketError>,
        H: Into<String>,
        P: Into<RelRefBuf>,
    {
        NullRemoteEndpoint
    }

    fn remote_endpoint_from_uri(&self, _uri: &Uri) -> Result<Self::RemoteEndpoint, Error> {
        Ok(NullRemoteEndpoint)
    }

    type LookupStream = futures::stream::Iter<std::vec::IntoIter<Self::SocketAddr>>;

    fn lookup(&self, _hostname: &str, mut _port: u16) -> Result<Self::LookupStream, Error> {
        let dummy_iter = "127.0.0.1:12345".to_socket_addrs().unwrap();
        Ok(futures::stream::iter(dummy_iter))
    }

    type InboundContext = NullInboundContext;

    fn send<'a, S, R, SD>(&'a self, _dest: S, _send_desc: SD) -> BoxFuture<'a, Result<R, Error>>
    where
        S: ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::SocketError> + 'a,
        SD: SendDesc<Self::InboundContext, R> + 'a,
        R: Send + 'a,
    {
        futures::future::ready(Err(Error::ResponseTimeout)).boxed()
    }

    type RespondableInboundContext = NullRespondableInboundContext;

    fn receive<'a, F>(&'a self, _handler: F) -> BoxFuture<'a, Result<(), Error>>
    where
        F: FnMut(&Self::RespondableInboundContext) -> Result<(), Error> + 'a,
    {
        futures::future::pending::<Result<(), Error>>().boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use std::net::SocketAddr;

    #[test]
    fn ping() {
        let local_endpoint = NullLocalEndpoint;

        let future = local_endpoint.send(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1234),
            Ping::new(),
        );

        assert_eq!(Err(Error::ResponseTimeout), block_on(future));
    }
}
