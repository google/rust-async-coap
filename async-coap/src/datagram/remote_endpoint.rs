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
use std::sync::{Arc, Weak};

/// [`RemoteEndpoint`] implementation for [`DatagramLocalEndpoint`].
#[derive(Debug, Clone)]
pub struct DatagramRemoteEndpoint<US: AsyncDatagramSocket> {
    local_endpoint: Weak<DatagramLocalEndpointInner<US>>,
    socket_addr: US::SocketAddr,
    host: Option<String>,
    path: RelRefBuf,
}

impl<US: AsyncDatagramSocket> DatagramRemoteEndpoint<US> {
    pub(crate) fn new(
        local_endpoint: &Arc<DatagramLocalEndpointInner<US>>,
        socket_addr: US::SocketAddr,
        host: Option<String>,
        path: RelRefBuf,
    ) -> DatagramRemoteEndpoint<US> {
        DatagramRemoteEndpoint {
            local_endpoint: Arc::downgrade(local_endpoint),
            socket_addr,
            host,
            path,
        }
    }
}

impl<US: AsyncDatagramSocket> RemoteEndpoint for DatagramRemoteEndpoint<US> {
    type SocketAddr = US::SocketAddr;
    type InboundContext = DatagramInboundContext<Self::SocketAddr>;

    fn uri(&self) -> UriBuf {
        let local_endpoint = match self.local_endpoint.upgrade() {
            Some(local_endpoint) => local_endpoint,
            None => return uri!("null:///").to_owned(),
        };

        let scheme = local_endpoint.scheme();
        let path = &self.path;

        let mut uri_abs = if self.host.is_some() && !self.host.as_ref().unwrap().is_empty() {
            let host = self.host.as_ref().unwrap();
            let port = self.socket_addr.port();
            if port != local_endpoint.default_port() {
                UriBuf::from_scheme_host_port(scheme, host, Some(port))
            } else {
                UriBuf::from_scheme_host_port(scheme, host, None)
            }
        } else {
            uri_format!("{}://{}", scheme, self.socket_addr).unwrap()
        };

        uri_abs.replace_path(path);

        uri_abs
    }

    fn scheme(&self) -> &'static str {
        match self.local_endpoint.upgrade() {
            Some(local_endpoint) => local_endpoint.scheme(),
            None => return "null",
        }
    }

    fn remove_host_option(&mut self) {
        self.host = None;
    }

    fn clone_using_rel_ref(&self, uri: &RelRef) -> Self {
        DatagramRemoteEndpoint {
            local_endpoint: self.local_endpoint.clone(),
            socket_addr: self.socket_addr,
            host: self.host.clone(),
            path: self.path.resolved_rel_ref(uri),
        }
    }

    fn send<'a, R, SD>(&'a self, send_desc: SD) -> BoxFuture<'a, Result<R, Error>>
    where
        SD: SendDesc<Self::InboundContext, R> + 'a,
        R: Send + 'a,
    {
        let local_endpoint = match self.local_endpoint.upgrade() {
            Some(local_endpoint) => local_endpoint,
            None => return futures::future::ready(Err(Error::Cancelled)).boxed(),
        };

        let send_desc = send_desc.uri_host_path(self.host.clone(), &self.path);

        let ret = if let Some(trans_params) = send_desc.trans_params() {
            UdpSendFuture::new(&local_endpoint, self.socket_addr, send_desc, trans_params)
        } else {
            UdpSendFuture::new(
                &local_endpoint,
                self.socket_addr,
                send_desc,
                StandardCoapConstants,
            )
        };

        ret.boxed()
    }

    fn send_to<'a, R, SD, UF>(&'a self, path: UF, send_desc: SD) -> BoxFuture<'a, Result<R, Error>>
    where
        SD: SendDesc<Self::InboundContext, R> + 'a,
        R: Send + 'a,
        UF: AsRef<RelRef>,
    {
        let local_endpoint = match self.local_endpoint.upgrade() {
            Some(local_endpoint) => local_endpoint,
            None => return futures::future::ready(Err(Error::Cancelled)).boxed(),
        };

        let send_desc =
            send_desc.uri_host_path(self.host.clone(), self.path.resolved_rel_ref(path));

        let ret = if let Some(trans_params) = send_desc.trans_params() {
            UdpSendFuture::new(&local_endpoint, self.socket_addr, send_desc, trans_params)
        } else {
            UdpSendFuture::new(
                &local_endpoint,
                self.socket_addr,
                send_desc,
                StandardCoapConstants,
            )
        };

        ret.boxed()
    }
}
