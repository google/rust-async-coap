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

/// Send descriptor created by [`CoapRequest::observe`] used for sending CoAP GET requests that
/// observe changing resources.
///
/// This send descriptor can yield multiple results, so it should be used with
/// [`LocalEndpointExt::send_as_stream`], [`RemoteEndpointExt::send_as_stream`],
/// and/or [`RemoteEndpointExt::send_to_as_stream`].
#[derive(Debug)]
pub struct SendObserve<IC> {
    phantom: PhantomData<IC>,
}

impl<IC> SendDescUnicast for SendObserve<IC> {}

impl<IC> Default for SendObserve<IC> {
    fn default() -> Self {
        Self::new()
    }
}

impl<IC> SendObserve<IC> {
    pub(crate) fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }

    /// Returns a nonconfirmable version of this send descriptor.
    #[inline(always)]
    pub fn nonconfirmable(self) -> Nonconfirmable<SendObserve<IC>> {
        Default::default()
    }

    /// Returns a multicast version of this send descriptor.
    #[inline(always)]
    pub fn multicast(self) -> Multicast<SendObserve<IC>> {
        Default::default()
    }
}

impl<IC: InboundContext> SendDesc<IC, ()> for SendObserve<IC> {
    fn delay_to_restart(&self) -> Option<Duration> {
        // TODO(#7): Derive this value from the `MaxAge` option on the response.
        Some(Duration::from_secs(60))
    }

    fn write_options(
        &self,
        msg: &mut dyn OptionInsert,
        socket_addr: &IC::SocketAddr,
        start: Bound<OptionNumber>,
        end: Bound<OptionNumber>,
    ) -> Result<(), Error> {
        write_options!((msg, socket_addr, start, end) {
            OBSERVE => Some(OBSERVE_REGISTER),
        })
    }

    fn write_payload(
        &self,
        msg: &mut dyn MessageWrite,
        _socket_addr: &IC::SocketAddr,
    ) -> Result<(), Error> {
        msg.set_msg_code(MsgCode::MethodGet);
        Ok(())
    }

    fn handler(&mut self, context: Result<&IC, Error>) -> Result<ResponseStatus<()>, Error> {
        context?;
        Ok(ResponseStatus::Continue)
    }
}
