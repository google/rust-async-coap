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

impl<SD: SendDescUnicast, IC> SendDescUnicast for PayloadWriter<SD, IC> {}
impl<SD: SendDescMulticast, IC> SendDescMulticast for PayloadWriter<SD, IC> {}

/// Combinator for Send Descriptors created by [`SendDescExt::payload_writer`].
#[derive(Debug)]
pub struct PayloadWriter<SD, F> {
    pub(super) inner: SD,
    pub(super) writer: F,
}

impl<SD, F, IC, R> SendDesc<IC, R> for PayloadWriter<SD, F>
where
    SD: SendDesc<IC, R> + Send,
    IC: InboundContext,
    R: Send,
    F: Fn(&mut dyn MessageWrite) -> Result<(), Error> + Send,
{
    send_desc_passthru_timing!(inner);
    send_desc_passthru_options!(inner);
    send_desc_passthru_handler!(inner, R);

    fn write_payload(
        &self,
        msg: &mut dyn MessageWrite,
        _socket_addr: &IC::SocketAddr,
    ) -> Result<(), Error> {
        (self.writer)(msg)
    }
}
