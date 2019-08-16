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

impl<SD: SendDescUnicast> SendDescUnicast for IncludeSocketAddr<SD> {}
impl<SD: SendDescMulticast> SendDescMulticast for IncludeSocketAddr<SD> {}

/// Combinator for Send Descriptors created by [`SendDescExt::include_socket_addr`].
#[derive(Debug)]
pub struct IncludeSocketAddr<SD> {
    pub(super) inner: SD,
}

impl<SD> IncludeSocketAddr<SD> {
    pub(super) fn new(inner: SD) -> IncludeSocketAddr<SD> {
        IncludeSocketAddr { inner }
    }
}

impl<SD, IC, R> SendDesc<IC, (R, IC::SocketAddr)> for IncludeSocketAddr<SD>
where
    SD: SendDesc<IC, R> + Send,
    IC: InboundContext,
    R: Send,
{
    send_desc_passthru_timing!(inner);
    send_desc_passthru_options!(inner);
    send_desc_passthru_payload!(inner);
    send_desc_passthru_supports_option!(inner);

    fn handler(
        &mut self,
        context: Result<&IC, Error>,
    ) -> Result<ResponseStatus<(R, IC::SocketAddr)>, Error> {
        let socket_addr = context.ok().map(|x| x.remote_socket_addr());

        self.inner.handler(context).map(|x| match (x, socket_addr) {
            (ResponseStatus::Done(x), Some(socket_addr)) => ResponseStatus::Done((x, socket_addr)),
            (ResponseStatus::Done(_), None) => unreachable!(),
            (ResponseStatus::SendNext, _) => ResponseStatus::SendNext,
            (ResponseStatus::Continue, _) => ResponseStatus::Continue,
        })
    }
}
