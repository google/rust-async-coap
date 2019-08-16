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

impl<SD: SendDescUnicast, IC> SendDescUnicast for Handler<SD, IC> {}
impl<SD: SendDescMulticast, IC> SendDescMulticast for Handler<SD, IC> {}

/// Combinator for Send Descriptors created by [`SendDescExt::use_handler`].
#[derive(Debug)]
pub struct Handler<SD, F> {
    pub(super) inner: SD,
    pub(super) handler: F,
}

impl<SD, F, IC, R> SendDesc<IC, R> for Handler<SD, F>
where
    SD: SendDesc<IC, ()> + Send,
    IC: InboundContext,
    R: Send,
    F: FnMut(
            Result<&dyn InboundContext<SocketAddr = IC::SocketAddr>, Error>,
        ) -> Result<ResponseStatus<R>, Error>
        + Send,
{
    send_desc_passthru_timing!(inner);
    send_desc_passthru_options!(inner);
    send_desc_passthru_payload!(inner);
    send_desc_passthru_supports_option!(inner);

    fn handler(&mut self, context: Result<&IC, Error>) -> Result<ResponseStatus<R>, Error> {
        let inner_result = self.inner.handler(context);
        let outer_result = (self.handler)(
            context.map(|ic| ic as &dyn InboundContext<SocketAddr = IC::SocketAddr>),
        );

        if inner_result.is_err() || outer_result.is_err() {
            Err(inner_result.err().or(outer_result.err()).unwrap())
        } else {
            outer_result
        }
    }
}
