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
use std::marker::PhantomData;

impl<SD: SendDescUnicast, IC> SendDescUnicast for UnicastBlock2<SD, IC> {}
impl<SD: SendDescUnicast, IC> SendDescUnicast for UnicastBlock2Collect<SD, IC> {}

/// Unicast Block2 Tracking combinator, created by [`SendDescUnicast::block2`].
///
#[derive(Debug)]
pub struct UnicastBlock2<SD, IC> {
    pub(super) inner: SD,
    pub(super) block2_default: Option<BlockInfo>,
    pub(super) reconstructor: Option<BlockReconstructor<VecMessageEncoder>>,
    pub(super) etag: Option<ETag>,
    pub(super) phantom: PhantomData<IC>,
}

impl<SD, IC> UnicastBlock2<SD, IC> {
    pub(super) fn new(inner: SD, block2: Option<BlockInfo>) -> UnicastBlock2<SD, IC> {
        UnicastBlock2 {
            inner,
            block2_default: block2,
            reconstructor: None,
            etag: None,
            phantom: PhantomData,
        }
    }

    /// Adds Block2 collection support to this [`SendDesc`] chain.
    ///
    /// This may only follow a [`UnicastBlock2`], and the prior return type
    /// must be `()` (the default).
    pub fn emit_successful_collected_response(self) -> UnicastBlock2Collect<SD, IC> {
        UnicastBlock2Collect { inner: self }
    }
}

impl<SD, IC, R> SendDesc<IC, R> for UnicastBlock2<SD, IC>
where
    SD: SendDesc<IC, R> + Send + SendDescUnicast,
    IC: InboundContext,
    R: Send,
{
    send_desc_passthru_timing!(inner);
    send_desc_passthru_payload!(inner);

    fn supports_option(&self, option: OptionNumber) -> bool {
        self.inner.supports_option(option) || option == OptionNumber::BLOCK2
    }

    fn write_options(
        &self,
        msg: &mut dyn OptionInsert,
        socket_addr: &IC::SocketAddr,
        start: Bound<OptionNumber>,
        end: Bound<OptionNumber>,
    ) -> Result<(), Error> {
        let block2 = self
            .reconstructor
            .as_ref()
            .map(|r| r.next_block())
            .or(self.block2_default);

        write_options!((msg, socket_addr, start, end, self.inner) {
        // Commenting this out for now because coap.me seems to be broken?
        //            ETAG => self.etag.into_iter(),
                    BLOCK2 => block2.into_iter(),
                })
    }

    fn handler(&mut self, context: Result<&IC, Error>) -> Result<ResponseStatus<R>, Error> {
        if let Some(context) = context.ok() {
            if context.is_dupe() {
                // Ignore dupes.
                return Ok(ResponseStatus::Continue);
            }
            let msg = context.message();
            let block2 = msg.block2();

            if let Some(block2) = block2 {
                let etag = msg.options().find_next_of(option::ETAG).transpose()?;

                if etag != self.etag {
                    if self.etag.is_none() && self.reconstructor.is_none() {
                        self.etag = etag;
                    } else {
                        // Etag mismatch
                        self.reconstructor = None;
                        self.etag = None;
                        return self.inner.handler(Err(Error::Reset));
                    }
                }

                if self.reconstructor.is_none() {
                    let mut encoder = VecMessageEncoder::default();
                    msg.write_msg_to(&mut encoder)?;

                    if !block2.more_flag() || block2.offset() != 0 {
                        // Bad initial block2?
                        return self.inner.handler(Ok(context));
                    }

                    let next_block = block2.next().unwrap();
                    self.reconstructor = Some(BlockReconstructor::new(encoder, next_block));
                }

                match self
                    .reconstructor
                    .as_mut()
                    .unwrap()
                    .feed(block2, msg.payload())
                {
                    Ok(false) => {
                        return self
                            .inner
                            .handler(Ok(context))
                            .map(|_| ResponseStatus::SendNext)
                    }
                    Ok(true) => return self.inner.handler(Ok(context)),
                    Err(_) => {
                        self.reconstructor = None;
                        self.etag = None;
                        return self.inner.handler(Err(Error::Reset));
                    }
                };
            } else {
                self.reconstructor = None;
                self.etag = None;
            }
        }

        self.inner.handler(context)
    }
}

/// Unicast Block2 Collecting combinator, created by [`UnicastBlock2::emit_successful_collected_response`].
///
/// This `SendDesc` will collect all of the various pieces and emit a single allocated
/// [`MessageRead`] instance that contains the entire payload.
#[derive(Debug)]
pub struct UnicastBlock2Collect<SD, SA> {
    inner: UnicastBlock2<SD, SA>,
}

impl<SD, IC> SendDesc<IC, OwnedImmutableMessage> for UnicastBlock2Collect<SD, IC>
where
    SD: SendDesc<IC, ()> + Send + SendDescUnicast,
    IC: InboundContext,
{
    send_desc_passthru_timing!(inner);
    send_desc_passthru_payload!(inner);
    send_desc_passthru_options!(inner);
    send_desc_passthru_supports_option!(inner);

    fn handler(
        &mut self,
        context: Result<&IC, Error>,
    ) -> Result<ResponseStatus<OwnedImmutableMessage>, Error> {
        let ret = match self.inner.handler(context) {
            Ok(rs) => {
                if let Some(recons) = self.inner.reconstructor.as_ref() {
                    if recons.is_finished() {
                        self.inner.reconstructor.take().unwrap().into_inner().into()
                    } else {
                        return Ok(match rs {
                            ResponseStatus::SendNext => ResponseStatus::SendNext,
                            _ => ResponseStatus::Continue,
                        });
                    }
                } else if let Some(context) = context.ok() {
                    context.message().to_owned()
                } else {
                    return Ok(match rs {
                        ResponseStatus::SendNext => ResponseStatus::SendNext,
                        _ => ResponseStatus::Continue,
                    });
                }
            }
            Err(Error::ClientRequestError) if context.is_ok() => {
                context.unwrap().message().to_owned()
            }
            Err(e) => return Err(e),
        };

        return Ok(ResponseStatus::Done(ret));
    }
}
