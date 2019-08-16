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

impl<SD: SendDescUnicast, IC> SendDescUnicast for Inspect<SD, IC> {}
impl<SD: SendDescMulticast, IC> SendDescMulticast for Inspect<SD, IC> {}

/// Combinator for Send Descriptors created by [`SendDescExt::inspect`].
#[derive(Debug)]
pub struct Inspect<SD, F> {
    pub(super) inner: SD,
    pub(super) inspect: F,
}

impl<SD, F, IC, R> SendDesc<IC, R> for Inspect<SD, F>
where
    SD: SendDesc<IC, R> + Send,
    IC: InboundContext,
    R: Send,
    F: FnMut(&dyn InboundContext<SocketAddr = IC::SocketAddr>) + Send,
{
    send_desc_passthru_timing!(inner);
    send_desc_passthru_options!(inner);
    send_desc_passthru_payload!(inner);
    send_desc_passthru_supports_option!(inner);

    fn handler(&mut self, context: Result<&IC, Error>) -> Result<ResponseStatus<R>, Error> {
        if let Some(context) = context.ok() {
            (self.inspect)(context);
        }
        self.inner.handler(context)
    }
}
