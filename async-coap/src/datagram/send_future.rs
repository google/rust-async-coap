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
use crate::message::BufferMessageEncoder;
use futures::prelude::*;
use futures::task::Waker;
use futures::Poll;
use futures_timer::Delay;
use std::cell::Cell;
use std::fmt::{Display, Formatter};
use std::ops::Bound;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(super) enum UdpSendFutureState<R> {
    /// This send future hasn't been polled yet.
    Uninit,

    /// We are waiting for either an Ack or a response, and we will retransmit
    /// occasionally.
    ActivelyWaiting,

    /// We are still waiting for a response. We are not retransmitting.
    PassivelyWaiting,

    /// We are finished and waiting for our final result to be polled.
    Finished(Result<R, Error>),

    /// We are completely finished.
    Expired,
}

impl<R> Display for UdpSendFutureState<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            UdpSendFutureState::Uninit => f.write_str("Uninit"),
            UdpSendFutureState::ActivelyWaiting => f.write_str("ActivelyWaiting"),
            UdpSendFutureState::PassivelyWaiting => f.write_str("PassivelyWaiting"),
            UdpSendFutureState::Finished(Ok(_)) => f.write_str("Finished"),
            UdpSendFutureState::Finished(Err(e)) => write!(f, "Errored({:?})", e),
            UdpSendFutureState::Expired => f.write_str("Expired"),
        }
    }
}

impl<R> UdpSendFutureState<R> {
    pub fn is_waiting(&self) -> bool {
        match self {
            UdpSendFutureState::ActivelyWaiting | UdpSendFutureState::PassivelyWaiting => true,
            _ => false,
        }
    }

    pub fn is_finished(&self) -> bool {
        match self {
            UdpSendFutureState::Finished(_) | UdpSendFutureState::Expired => true,
            _ => false,
        }
    }

    pub fn finished(self) -> Option<Result<R, Error>> {
        match self {
            UdpSendFutureState::Finished(x) => Some(x),
            _ => None,
        }
    }
}

pub(super) struct UdpSendFutureInner<R, SD, US, TP>
where
    R: Send,
    SD: SendDesc<DatagramInboundContext<US::SocketAddr>, R>,
    US: AsyncDatagramSocket,
    TP: TransParams,
{
    send_desc: SD,
    state: UdpSendFutureState<R>,
    waker: Option<futures::task::Waker>,
    local_endpoint: Weak<DatagramLocalEndpointInner<US>>,
    dest: US::SocketAddr,

    msg_id: MsgId,
    msg_token: Cell<MsgToken>,
    retransmit_count: Cell<u32>,
    delay: Option<Delay>,
    timeout: Cell<Option<Instant>>,
    _trans_params: TP, // <datagram::DatagramLocalEndpoint<US> as LocalEndpoint>::DefaultTransParams
}

impl<R, SD, US, TP> UdpSendFutureInner<R, SD, US, TP>
where
    R: Send,
    SD: SendDesc<DatagramInboundContext<US::SocketAddr>, R>,
    US: AsyncDatagramSocket + Sized,
    TP: TransParams,
{
    fn state(&self) -> &UdpSendFutureState<R> {
        &self.state
    }

    fn change_state(&mut self, mut state: UdpSendFutureState<R>) -> UdpSendFutureState<R> {
        if state.is_finished() {
            self.update_timeout(None);
        }
        std::mem::swap(&mut self.state, &mut state);
        state
    }

    fn update_waker(&mut self, waker_ref: &Waker) {
        if let Some(waker) = self.waker.take() {
            self.waker = Some(if waker_ref.will_wake(&waker) {
                waker
            } else {
                waker_ref.clone()
            });
        } else {
            self.waker = Some(waker_ref.clone());
        }
    }

    fn update_timeout(&mut self, d: Option<Duration>) {
        if let Some(d) = d {
            if let Some(delay) = self.delay.as_mut() {
                delay.reset(d);
            } else {
                self.delay = Some(Delay::new(d));
            }
        } else {
            self.delay = None;
        }
    }

    fn poll_timeout(
        &mut self,
        cx: &mut futures::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        if let Some(delay) = self.delay.as_mut() {
            Pin::new(delay).poll(cx)
        } else {
            Poll::Pending
        }
    }

    pub fn transmit(&self) -> Result<(), Error> {
        let mut buffer = [0u8; StandardCoapConstants::MAX_OUTBOUND_PACKET_LENGTH];
        let mut builder = BufferMessageEncoder::new(&mut buffer);

        let mut token = self.msg_token.get();

        if token.is_empty() {
            token = MsgToken::from(self.msg_id);
        }

        builder.set_msg_token(token);

        self.send_desc.write_options(
            &mut builder,
            &self.dest,
            Bound::Unbounded,
            Bound::Unbounded,
        )?;
        self.send_desc.write_payload(&mut builder, &self.dest)?;

        let builder_token = builder.msg_token();

        self.msg_token.replace(builder_token);

        // We always control the msg_id.
        builder.set_msg_id(self.msg_id);

        println!("OUTBOUND: {} {}", self.dest, builder);

        let buffer: &[u8] = &builder;

        if let Some(e) = self
            .local_endpoint
            .upgrade()
            .ok_or(Error::Cancelled)?
            .socket()
            .send_to(&buffer, self.dest)
            .err()
        {
            println!("send_to: io error: {:?} (dest={:?})", e, self.dest);
            return Err(Error::IOError);
        }

        println!("Did transmit.");

        self.retransmit_count.set(0);

        Ok(())
    }

    pub fn retransmit(&self) -> Result<(), Error> {
        let mut buffer = [0u8; StandardCoapConstants::MAX_OUTBOUND_PACKET_LENGTH];
        let mut builder = BufferMessageEncoder::new(&mut buffer);

        if let Some(timeout) = self.timeout.get() {
            if Instant::now() > timeout {
                return Err(Error::ResponseTimeout);
            }
        }

        builder.set_msg_token(self.msg_token.get());

        self.send_desc.write_options(
            &mut builder,
            &self.dest,
            Bound::Unbounded,
            Bound::Unbounded,
        )?;
        self.send_desc.write_payload(&mut builder, &self.dest)?;

        builder.set_msg_id(self.msg_id);

        println!(
            "OUTBOUND[{}]: {} {}",
            self.retransmit_count.get() + 1,
            self.dest,
            builder
        );

        let buffer: &[u8] = &builder;

        if let Some(e) = self
            .local_endpoint
            .upgrade()
            .ok_or(Error::Cancelled)?
            .socket()
            .send_to(buffer, self.dest)
            .err()
        {
            println!("send_to: io error: {:?} (dest={:?})", e, self.dest);
            return Err(Error::IOError);
        }

        self.retransmit_count.set(self.retransmit_count.get() + 1);

        println!("Did retransmit, count {}", self.retransmit_count.get());

        Ok(())
    }

    fn wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}

impl<R, SD, US, TP> HandleResponse<DatagramInboundContext<US::SocketAddr>>
    for UdpSendFutureInner<R, SD, US, TP>
where
    R: Send,
    SD: SendDesc<DatagramInboundContext<US::SocketAddr>, R>,
    US: AsyncDatagramSocket,
    TP: TransParams,
{
    fn handle_response(&mut self, context: Result<&DatagramInboundContext<US::SocketAddr>, Error>) {
        // This should only be called if we are waiting for a response.
        assert!(self.state().is_waiting(), "Invalid state: {}", self.state());

        // If this is an ack, we don't pass this along to the send_desc.
        if let Some(context) = context.ok() {
            let message = context.message();

            if !self.dest.is_multicast()
                && message.msg_code().is_empty()
                && message.msg_type().is_ack()
            {
                println!("Got ack!");

                self.change_state(UdpSendFutureState::PassivelyWaiting);
                let d = self.send_desc.max_rtt();
                self.update_timeout(Some(d));
                self.wake();
                return;
            }
        }

        // Pass the full context along to our `send_desc.handler()`
        match self.send_desc.handler(context) {
            Ok(ResponseStatus::Done(x)) => {
                // Stick a fork in us, we are done.
                self.change_state(UdpSendFutureState::Finished(Ok(x)));
            }
            Ok(ResponseStatus::Continue) => {
                if !self.dest.is_multicast() {
                    self.change_state(UdpSendFutureState::PassivelyWaiting);
                    let d = self.send_desc.max_rtt();
                    self.update_timeout(Some(d));
                }
            }
            Ok(ResponseStatus::SendNext) => {
                // Allocate a new msg-id, Reset retransmit count, and resend.
                self.change_state(UdpSendFutureState::Uninit);
            }
            Err(e) => {
                self.change_state(UdpSendFutureState::Finished(Err(e)));
            }
        }

        self.wake();
    }
}

pub(super) struct UdpSendFuture<R, SD, US, TP>
where
    R: Send,
    SD: SendDesc<DatagramInboundContext<US::SocketAddr>, R>,
    US: AsyncDatagramSocket,
    TP: TransParams,
{
    inner: Arc<Mutex<UdpSendFutureInner<R, SD, US, TP>>>,
}

impl<'lep, R, SD, US, TP> UdpSendFuture<R, SD, US, TP>
where
    R: Send,
    SD: SendDesc<DatagramInboundContext<US::SocketAddr>, R>,
    US: AsyncDatagramSocket,
    TP: TransParams,
{
    pub(super) fn new(
        local_endpoint: &Arc<DatagramLocalEndpointInner<US>>,
        dest: US::SocketAddr,
        send_desc: SD,
        trans_params: TP,
    ) -> UdpSendFuture<R, SD, US, TP> {
        UdpSendFuture {
            inner: Arc::new(Mutex::new(UdpSendFutureInner {
                send_desc,
                state: UdpSendFutureState::Uninit,
                waker: None,
                msg_id: local_endpoint.next_msg_id(),
                msg_token: Cell::new(MsgToken::EMPTY),
                local_endpoint: Arc::downgrade(&local_endpoint),
                dest,
                retransmit_count: Cell::new(0),
                delay: None,
                timeout: Cell::new(None),
                _trans_params: trans_params,
            })),
        }
    }

    fn poll(
        &mut self,
        cx: &mut futures::task::Context<'_>,
    ) -> futures::task::Poll<Result<R, Error>> {
        let mut inner = self
            .inner
            .lock()
            .expect("UdpSendFuture inner mutex poisoned");

        match inner.state() {
            UdpSendFutureState::Uninit => {
                // TODO(#4): Figure out how this can be set programmatically.
                inner.timeout.set(Some(
                    Instant::now() + inner.send_desc.transmit_wait_duration(),
                ));

                if let Some(error) = inner.transmit().err() {
                    inner.change_state(UdpSendFutureState::Finished(Err(error)));
                } else {
                    inner
                        .local_endpoint
                        .upgrade()
                        .ok_or(Error::Cancelled)?
                        .add_response_handler(
                            inner.msg_id,
                            inner.msg_token.get(),
                            inner.dest.clone(),
                            self.inner.clone(),
                        );

                    if let Some(d) = inner
                        .send_desc
                        .delay_to_retransmit(inner.retransmit_count.get())
                    {
                        inner.change_state(UdpSendFutureState::ActivelyWaiting);
                        inner.update_timeout(Some(d));
                        let _ = inner.poll_timeout(cx);
                    } else {
                        inner.change_state(UdpSendFutureState::PassivelyWaiting);
                        let d = inner.send_desc.max_rtt();
                        inner.update_timeout(Some(d));
                        let _ = inner.poll_timeout(cx);
                    }
                }
            }

            UdpSendFutureState::ActivelyWaiting => {
                // We are waiting to retransmit.
                if inner.poll_timeout(cx).is_ready() {
                    if let Some(error) = inner.retransmit().err() {
                        inner.change_state(UdpSendFutureState::Finished(Err(error)));
                    } else if let Some(d) = inner
                        .send_desc
                        .delay_to_retransmit(inner.retransmit_count.get())
                    {
                        inner.update_timeout(Some(d));
                        let _ = inner.poll_timeout(cx);
                    } else {
                        inner.change_state(UdpSendFutureState::PassivelyWaiting);
                        let d = inner.send_desc.max_rtt();
                        inner.update_timeout(Some(d));
                        let _ = inner.poll_timeout(cx);
                    }
                }
            }

            UdpSendFutureState::PassivelyWaiting => {
                // We are waiting for the end of the RTT
                if inner.poll_timeout(cx).is_ready() {
                    inner.handle_response(Err(Error::ResponseTimeout));
                }
            }

            UdpSendFutureState::Finished(_) | UdpSendFutureState::Expired => {
                // We are done, nothing to do here.
            }
        }

        if inner.state().is_finished() {
            let ret = inner
                .change_state(UdpSendFutureState::Expired)
                .finished()
                .unwrap();
            futures::task::Poll::Ready(ret)
        } else {
            inner.update_waker(cx.waker());

            futures::task::Poll::Pending
        }
    }
}

impl<R, SD, US, TP> Drop for UdpSendFuture<R, SD, US, TP>
where
    R: Send,
    SD: SendDesc<DatagramInboundContext<US::SocketAddr>, R>,
    US: AsyncDatagramSocket,
    TP: TransParams,
{
    fn drop(&mut self) {
        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("UdpSendFuture mutex inner was poisoned, locking anyway to drop");
                poisoned.into_inner()
            }
        };

        if let Some(le) = inner.local_endpoint.upgrade() {
            le.remove_response_handler(inner.msg_id, inner.msg_token.get(), inner.dest.clone());
        }
    }
}

impl<R, SD, US, TP> Future for UdpSendFuture<R, SD, US, TP>
where
    R: Send,
    SD: SendDesc<DatagramInboundContext<US::SocketAddr>, R>,
    US: AsyncDatagramSocket,
    TP: TransParams,
{
    type Output = Result<R, Error>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut futures::task::Context<'_>,
    ) -> futures::task::Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}
