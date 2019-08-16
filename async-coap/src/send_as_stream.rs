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

use crate::send_desc::SendDesc;
use futures::channel::mpsc::{Receiver, Sender};
use futures::task::Context;
use futures::task::Poll;
use pin_utils::unsafe_pinned;
use std::marker::PhantomData;
use std::ops::Bound;
use std::pin::Pin;

/// A [`Stream`] that is created by [`LocalEndpointExt::send_as_stream`],
/// [`RemoteEndpointExt::send_as_stream`], and [`RemoteEndpointExt::send_to_as_stream`].
///
/// [`Stream`]: futures::stream::Stream
/// [`LocalEndpointExt::send_as_stream`]: crate::LocalEndpointExt::send_as_stream
/// [`RemoteEndpointExt::send_as_stream`]: crate::RemoteEndpointExt::send_as_stream
/// [`RemoteEndpointExt::send_to_as_stream`]: crate::RemoteEndpointExt::send_to_as_stream
pub struct SendAsStream<'a, R: Send> {
    pub(crate) receiver: Receiver<Result<R, Error>>,
    pub(crate) send_future: BoxFuture<'a, Result<R, Error>>,
}

impl<'a, R: Send + core::fmt::Debug> core::fmt::Debug for SendAsStream<'a, R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("SendAsStream")
            .field("receiver", &self.receiver)
            .field("send_future", &"")
            .finish()
    }
}

impl<'a, R: Send> SendAsStream<'a, R> {
    unsafe_pinned!(send_future: BoxFuture<'a, Result<R, Error>>);
    unsafe_pinned!(receiver: futures::channel::mpsc::Receiver<Result<R, Error>>);
}

impl<'a, R: Send> Stream for SendAsStream<'a, R> {
    type Item = Result<R, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.as_mut().receiver().poll_next(cx) {
            Poll::Ready(None) => Poll::Ready(None),
            from_receiver => match self.as_mut().send_future().poll(cx) {
                Poll::Ready(Ok(_)) => Poll::Ready(None),
                Poll::Ready(Err(Error::ResponseTimeout)) | Poll::Ready(Err(Error::Cancelled)) => {
                    Poll::Ready(None)
                }
                Poll::Ready(Err(x)) => Poll::Ready(Some(Err(x))),
                Poll::Pending => from_receiver,
            },
        }
    }
}

#[derive(Debug)]
pub(crate) struct SendAsStreamDesc<SD, IC, R>
where
    SD: SendDesc<IC, R>,
    IC: InboundContext,
    R: Send,
{
    inner: SD,
    sender: Sender<Result<R, Error>>,
    phantom: PhantomData<IC>,
}

impl<SD, IC, R> SendAsStreamDesc<SD, IC, R>
where
    SD: SendDesc<IC, R>,
    IC: InboundContext,
    R: Send,
{
    pub(crate) fn new(inner: SD, sender: Sender<Result<R, Error>>) -> SendAsStreamDesc<SD, IC, R> {
        SendAsStreamDesc {
            inner,
            sender,
            phantom: PhantomData,
        }
    }
}

impl<SD, IC, R> SendDesc<IC, R> for SendAsStreamDesc<SD, IC, R>
where
    SD: SendDesc<IC, R>,
    IC: InboundContext,
    R: Send,
{
    send_desc_passthru_timing!(inner);
    send_desc_passthru_options!(inner);
    send_desc_passthru_payload!(inner);
    send_desc_passthru_supports_option!(inner);

    fn handler(&mut self, context: Result<&IC, Error>) -> Result<ResponseStatus<R>, Error> {
        match self.inner.handler(context)? {
            ResponseStatus::Done(x) => {
                if let Some(err) = self.sender.start_send(Ok(x)).err() {
                    if err.is_full() {
                        Err(Error::OutOfSpace)
                    } else {
                        Err(Error::Cancelled)
                    }
                } else {
                    Ok(ResponseStatus::Continue)
                }
            }
            response_status => Ok(response_status),
        }
    }
}
