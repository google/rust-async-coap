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
use crate::send_desc_passthru_handler;
use crate::send_desc_passthru_options;
use crate::send_desc_passthru_timing;

/// Nonconfirmable send descriptor combinator created by the `nonconfirmable()` method on
/// [`SendGet`], [`SendPut`], [`SendPost`], [`SendDelete`], and [`SendObserve`].
#[derive(Debug)]
pub struct Nonconfirmable<SD>(pub(crate) SD);

impl<SD: SendDescUnicast> SendDescUnicast for Nonconfirmable<SD> {}
impl<SD: Default> Default for Nonconfirmable<SD> {
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<SD, IC> SendDesc<IC, ()> for Nonconfirmable<SD>
where
    SD: SendDesc<IC, ()> + Send,
    IC: InboundContext,
{
    send_desc_passthru_timing!(0);
    send_desc_passthru_options!(0);
    send_desc_passthru_handler!(0);

    fn write_payload(
        &self,
        msg: &mut dyn MessageWrite,
        socket_addr: &IC::SocketAddr,
    ) -> Result<(), Error> {
        self.0.write_payload(msg, socket_addr)?;
        msg.set_msg_type(MsgType::Non);
        Ok(())
    }
}
