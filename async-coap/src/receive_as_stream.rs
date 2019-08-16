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
use futures::task::Context;
use futures::task::Poll;
use std::pin::Pin;

/// A [`Stream`] that is created by [`LocalEndpointExt::receive_as_stream`].
///
/// [`Stream`]: futures::stream::Stream
/// [`LocalEndpointExt::receive_as_stream`]: crate::LocalEndpointExt::receive_as_stream
pub struct ReceiveAsStream<'a, LE, F> {
    local_endpoint: &'a LE,
    handler: F,
    recv_future: Option<BoxFuture<'a, Result<(), Error>>>,
}

impl<'a, LE: core::fmt::Debug, F: core::fmt::Debug> core::fmt::Debug
    for ReceiveAsStream<'a, LE, F>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("ReceiveAsStream")
            .field("local_endpoint", self.local_endpoint)
            .field("handler", &self.handler)
            .field("recv_future", &self.recv_future.as_ref().map(|_| ""))
            .finish()
    }
}

impl<'a, LE, F> ReceiveAsStream<'a, LE, F>
where
    LE: LocalEndpoint,
    F: FnMut(&LE::RespondableInboundContext) -> Result<(), Error> + 'a + Clone + Unpin + Send,
{
    pub(crate) fn new(local_endpoint: &'a LE, handler: F) -> ReceiveAsStream<'a, LE, F> {
        let mut ret = ReceiveAsStream {
            local_endpoint,
            recv_future: None,
            handler,
        };
        ret.update_recv_future();
        return ret;
    }

    fn update_recv_future(&mut self) {
        self.recv_future = Some(self.local_endpoint.receive(self.handler.clone()));
    }

    fn _poll_next_unpin(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<(), Error>>> {
        if let Some(recv_future) = self.recv_future.as_mut() {
            match recv_future.poll_unpin(cx) {
                Poll::Ready(Err(Error::IOError)) => {
                    self.recv_future = None;
                    Poll::Ready(Some(Err(Error::IOError)))
                }
                Poll::Ready(Err(Error::Cancelled)) => {
                    self.recv_future = None;
                    Poll::Ready(Some(Err(Error::Cancelled)))
                }
                Poll::Ready(_) => {
                    self.update_recv_future();
                    Poll::Ready(Some(Ok(())))
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(None)
        }
    }
}

impl<'a, LE, F> Stream for ReceiveAsStream<'a, LE, F>
where
    LE: LocalEndpoint,
    F: FnMut(&LE::RespondableInboundContext) -> Result<(), Error> + 'a + Clone + Unpin + Send,
{
    type Item = Result<(), Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut()._poll_next_unpin(cx)
    }
}
