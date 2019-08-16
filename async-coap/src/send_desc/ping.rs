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

/// Send descriptor for sending a CoAP ping.
#[derive(Debug)]
pub struct Ping;

impl Ping {
    /// Creates a new instance of `Ping`.
    #[inline]
    pub fn new() -> Ping {
        Ping
    }
}

impl Default for Ping {
    #[inline]
    fn default() -> Self {
        Ping
    }
}

impl<IC: InboundContext> SendDesc<IC> for Ping {
    fn write_options(
        &self,
        _msg: &mut dyn OptionInsert,
        _socket_addr: &IC::SocketAddr,
        _start: Bound<OptionNumber>,
        _end: Bound<OptionNumber>,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn write_payload(
        &self,
        msg: &mut dyn MessageWrite,
        _socket_addr: &IC::SocketAddr,
    ) -> Result<(), Error> {
        msg.set_msg_code(MsgCode::Empty);
        msg.set_msg_type(MsgType::Con);
        msg.set_msg_token(MsgToken::EMPTY);
        Ok(())
    }

    fn handler(&mut self, context: Result<&IC, Error>) -> Result<ResponseStatus<()>, Error> {
        let context = context?;
        if context.message().msg_type() == MsgType::Res {
            Ok(ResponseStatus::Done(()))
        } else {
            Err(Error::BadResponse)
        }
    }
}
