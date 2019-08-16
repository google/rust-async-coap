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

use super::send_desc_passthru_options;
use super::*;

/// Multicast send descriptor combinator created by the `multicast()` method on
/// [`SendGet`], [`SendPut`], [`SendPost`], [`SendDelete`], and [`SendObserve`].
///
/// This send descriptor can yield multiple results, so it should be used with
/// [`LocalEndpointExt::send_as_stream`], [`RemoteEndpointExt::send_as_stream`],
/// and/or [`RemoteEndpointExt::send_to_as_stream`].
#[derive(Debug)]
pub struct Multicast<SD>(pub(crate) SD);

impl<SD> SendDescMulticast for Multicast<SD> {}
impl<SD: Default> Default for Multicast<SD> {
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<SD, IC> SendDesc<IC, ()> for Multicast<SD>
where
    SD: SendDesc<IC, ()> + Send,
    IC: InboundContext,
{
    send_desc_passthru_options!(0);
    send_desc_passthru_supports_option!(0);

    fn delay_to_retransmit(&self, retransmits_sent: u32) -> Option<Duration> {
        self.0.delay_to_retransmit(retransmits_sent)
    }
    fn delay_to_restart(&self) -> Option<Duration> {
        self.0.delay_to_restart()
    }
    fn max_rtt(&self) -> Duration {
        Duration::from_secs(8)
    }
    fn transmit_wait_duration(&self) -> Duration {
        Duration::from_secs(8)
    }

    fn write_payload(
        &self,
        msg: &mut dyn MessageWrite,
        socket_addr: &IC::SocketAddr,
    ) -> Result<(), Error> {
        self.0.write_payload(msg, socket_addr)?;
        msg.set_msg_type(MsgType::Non);
        Ok(())
    }

    fn handler(&mut self, context: Result<&IC, Error>) -> Result<ResponseStatus<()>, Error> {
        context?;
        Ok(ResponseStatus::Continue)
    }
}
