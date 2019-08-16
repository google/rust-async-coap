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
use crate::message::{OwnedImmutableMessage, VecMessageEncoder};
use std::cell::Cell;

/// Concrete instance of [`LocalEndpoint::RespondableInboundContext`] for [`DatagramLocalEndpoint`].
pub struct DatagramRespondableInboundContext<SA>
where
    Self: Send,
{
    message: OwnedImmutableMessage,
    message_out: Cell<Option<VecMessageEncoder>>,
    remote: SA,
    is_multicast: bool,
}

impl<SA> core::fmt::Debug for DatagramRespondableInboundContext<SA>
where
    SA: core::fmt::Debug,
    Self: Send,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("DatagramRespondableInboundContext")
            .field("message", &self.message)
            .field("message_out", &"")
            .field("remote", &self.remote)
            .field("is_multicast", &self.is_multicast)
            .finish()
    }
}

/// Concrete instance of [`LocalEndpoint::InboundContext`] for [`DatagramLocalEndpoint`].
pub type DatagramInboundContext<SA> = DatagramRespondableInboundContext<SA>;

impl<SA: SocketAddrExt> DatagramRespondableInboundContext<SA> {
    pub(super) fn new(
        buffer: Vec<u8>,
        remote: SA,
        is_multicast: bool,
    ) -> Result<DatagramRespondableInboundContext<SA>, Error> {
        Ok(DatagramRespondableInboundContext {
            message: OwnedImmutableMessage::new(buffer)?,
            message_out: Cell::new(Default::default()),
            remote,
            is_multicast,
        })
    }

    pub(super) fn into_message_out(self) -> Option<VecMessageEncoder> {
        self.message_out.take()
    }
}

impl<UA: SocketAddrExt> RespondableInboundContext for DatagramRespondableInboundContext<UA> {
    fn is_multicast(&self) -> bool {
        self.is_multicast
    }

    fn is_fake(&self) -> bool {
        // TODO: Determine how best to handle `is_fake()` on the Datagram local endpoint.
        false
    }

    fn respond<F>(&self, msg_gen: F) -> Result<(), Error>
    where
        F: Fn(&mut dyn MessageWrite) -> Result<(), Error>,
    {
        let mut builder = VecMessageEncoder::new();

        builder.set_msg_type(MsgType::Ack);
        builder.set_msg_token(self.message().msg_token());

        msg_gen(&mut builder)?;

        builder.set_msg_id(self.message().msg_id());

        self.message_out.replace(Some(builder));

        return Ok(());
    }
}

impl<SA: SocketAddrExt> InboundContext for DatagramRespondableInboundContext<SA> {
    type SocketAddr = SA;

    fn remote_socket_addr(&self) -> Self::SocketAddr {
        self.remote
    }

    fn is_dupe(&self) -> bool {
        // TODO: Determine how best to handle `is_dupe()` on the Datagram local endpoint.
        false
    }

    fn message(&self) -> &dyn MessageRead {
        &self.message
    }
}
