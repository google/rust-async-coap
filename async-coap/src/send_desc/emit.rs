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
use crate::message::OwnedImmutableMessage;

impl<SD: SendDescUnicast> SendDescUnicast for EmitAnyResponse<SD> {}
impl<SD: SendDescMulticast> SendDescMulticast for EmitAnyResponse<SD> {}

/// Combinator for Send Descriptors created by [`SendDescExt::emit_any_response`].
#[derive(Debug)]
pub struct EmitAnyResponse<SD> {
    pub(super) inner: SD,
}

impl<SD> EmitAnyResponse<SD> {
    pub(super) fn new(inner: SD) -> EmitAnyResponse<SD> {
        EmitAnyResponse { inner }
    }
}

impl<SD, IC> SendDesc<IC, OwnedImmutableMessage> for EmitAnyResponse<SD>
where
    SD: SendDesc<IC, ()> + Send,
    IC: InboundContext,
{
    send_desc_passthru_timing!(inner);
    send_desc_passthru_options!(inner);
    send_desc_passthru_payload!(inner);
    send_desc_passthru_supports_option!(inner);

    fn handler(
        &mut self,
        context: Result<&IC, Error>,
    ) -> Result<ResponseStatus<OwnedImmutableMessage>, Error> {
        let msg = context.ok().map(|x| x.message());

        match (self.inner.handler(context), msg) {
            (_, Some(msg)) => Ok(ResponseStatus::Done(msg.to_owned())),
            (Ok(ResponseStatus::SendNext), None) => Ok(ResponseStatus::SendNext),
            (Ok(ResponseStatus::Continue), None) => Ok(ResponseStatus::Continue),
            (Ok(ResponseStatus::Done(())), None) => unreachable!(),
            (Err(e), None) => Err(e),
        }
    }
}

/// Combinator for Send Descriptors created by [`SendDescExt::emit_successful_response`].
#[derive(Debug)]
pub struct EmitSuccessfulResponse<SD> {
    pub(super) inner: SD,
}

impl<SD> EmitSuccessfulResponse<SD> {
    pub(super) fn new(inner: SD) -> EmitSuccessfulResponse<SD> {
        EmitSuccessfulResponse { inner }
    }
}

impl<SD, IC> SendDesc<IC, OwnedImmutableMessage> for EmitSuccessfulResponse<SD>
where
    SD: SendDesc<IC, ()> + Send,
    IC: InboundContext,
{
    send_desc_passthru_timing!(inner);
    send_desc_passthru_options!(inner);
    send_desc_passthru_payload!(inner);
    send_desc_passthru_supports_option!(inner);

    fn handler(
        &mut self,
        context: Result<&IC, Error>,
    ) -> Result<ResponseStatus<OwnedImmutableMessage>, Error> {
        let msg = context.ok().map(|x| x.message());

        match (self.inner.handler(context), msg) {
            (Err(e), _) => Err(e),
            (_, Some(msg)) => Ok(ResponseStatus::Done(msg.to_owned())),
            (Ok(ResponseStatus::SendNext), None) => Ok(ResponseStatus::SendNext),
            (Ok(ResponseStatus::Continue), None) => Ok(ResponseStatus::Continue),
            (Ok(ResponseStatus::Done(())), None) => unreachable!(),
        }
    }
}

impl<SD: SendDescUnicast> SendDescUnicast for EmitMsgCode<SD> {}
impl<SD: SendDescMulticast> SendDescMulticast for EmitMsgCode<SD> {}

/// Combinator for Send Descriptors created by [`SendDescExt::emit_msg_code`].
#[derive(Debug)]
pub struct EmitMsgCode<SD> {
    pub(super) inner: SD,
}

impl<SD> EmitMsgCode<SD> {
    pub(super) fn new(inner: SD) -> EmitMsgCode<SD> {
        EmitMsgCode { inner }
    }
}

impl<SD, IC> SendDesc<IC, MsgCode> for EmitMsgCode<SD>
where
    SD: SendDesc<IC, ()> + Send,
    IC: InboundContext,
{
    send_desc_passthru_timing!(inner);
    send_desc_passthru_options!(inner);
    send_desc_passthru_payload!(inner);
    send_desc_passthru_supports_option!(inner);

    fn handler(&mut self, context: Result<&IC, Error>) -> Result<ResponseStatus<MsgCode>, Error> {
        let msg_code = context.ok().map(|x| x.message().msg_code());

        self.inner.handler(context).map(|x| match (x, msg_code) {
            (ResponseStatus::SendNext, _) => ResponseStatus::SendNext,
            (_, Some(msg)) => ResponseStatus::Done(msg.to_owned()),
            (_, _) => unreachable!(),
        })
    }
}
