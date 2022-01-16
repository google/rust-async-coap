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
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Mutex, Weak};

pub(crate) trait HandleResponse<IC: InboundContext>: Send {
    fn handle_response(&mut self, context: Result<&IC, Error>) -> bool;
}

pub(super) trait ResponseTracker<IC: InboundContext> {
    fn add_response_handler<'a>(
        &mut self,
        msg_id: MsgId,
        msg_token: MsgToken,
        socket_addr: IC::SocketAddr,
        handler: Arc<Mutex<dyn HandleResponse<IC> + 'a>>,
    );

    fn remove_response_handler(
        &mut self,
        msg_id: MsgId,
        msg_token: MsgToken,
        socket_addr: IC::SocketAddr,
    );
}

pub(crate) struct UdpResponseTracker<IC: InboundContext> {
    msg_id_map: HashMap<(MsgId, Option<IC::SocketAddr>), Weak<Mutex<dyn HandleResponse<IC>>>>,
    msg_token_map: HashMap<(MsgToken, Option<IC::SocketAddr>), Weak<Mutex<dyn HandleResponse<IC>>>>,
}

impl<IC: InboundContext> Debug for UdpResponseTracker<IC> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("UdpResponseTracker")
            .field("msg_id_map", &self.msg_id_map.keys())
            .field("msg_token_map", &self.msg_token_map.keys())
            .finish()
    }
}

impl<IC: InboundContext> UdpResponseTracker<IC> {
    pub(super) fn new() -> Self {
        UdpResponseTracker {
            msg_id_map: HashMap::new(),
            msg_token_map: HashMap::new(),
        }
    }

    pub(super) fn handle_response(&mut self, context: &IC) -> bool {
        let message = context.message();
        let socket_addr = context.remote_socket_addr();

        if let Some(weak) = self
            .msg_id_map
            .remove(&(message.msg_id(), Some(socket_addr)))
            .or(self.msg_id_map.remove(&(message.msg_id(), None)))
        {
            debug!("Matched response on msgid");
            if let Some(mutex) = weak.upgrade() {
                let mut handler = mutex.lock().expect("lock failure");
                let finished = handler.handle_response(Ok(context));
                if finished {
                    self.remove_by_token(message.msg_token(), socket_addr);
                }

                return true;
            }
        } else if let Some(weak) = self
            .msg_token_map
            .get(&(message.msg_token(), Some(socket_addr)))
            .or(self.msg_token_map.get(&(message.msg_token(), None)))
        {
            debug!("Matched response on token");
            if let Some(mutex) = weak.upgrade() {
                let mut handler = mutex.lock().expect("lock failure");
                let finished = handler.handle_response(Ok(context));
                if finished {
                    self.remove_by_token(message.msg_token(), socket_addr);
                }

                return true;
            }
        }
        debug!("Response did not match.");
        false
    }

    fn remove_by_token(&mut self, token: MsgToken, socket_addr: IC::SocketAddr) {
        self.msg_token_map
            .remove(&(token, Some(socket_addr)))
            .or(self.msg_token_map.remove(&(token, None)));
    }
}

impl<IC: InboundContext> ResponseTracker<IC> for UdpResponseTracker<IC> {
    fn add_response_handler<'a>(
        &mut self,
        msg_id: MsgId,
        msg_token: MsgToken,
        socket_addr: IC::SocketAddr,
        handler: Arc<Mutex<dyn HandleResponse<IC> + 'a>>,
    ) {
        // TODO(#3): Eliminate the need for this transmute.
        //       This transmute action here is a hack to coerce the lifetime 'a into 'static.
        //       It feels like there must be a different way, but after 8+ hours of lifetime hell
        //       I couldn't figure it out.
        let handler: Arc<Mutex<dyn HandleResponse<IC>>> = unsafe { std::mem::transmute(handler) };
        log::info!(
            "Adding response handler: msg_id:{:04X}, msg_token:{}",
            msg_id, msg_token
        );
        let socket_addr = if socket_addr.is_multicast() {
            None
        } else {
            Some(socket_addr)
        };

        self.msg_id_map
            .insert((msg_id, socket_addr), Arc::downgrade(&handler));
        self.msg_token_map
            .insert((msg_token, socket_addr), Arc::downgrade(&handler));
    }

    fn remove_response_handler(
        &mut self,
        msg_id: MsgId,
        msg_token: MsgToken,
        socket_addr: IC::SocketAddr,
    ) {
        let socket_addr = if socket_addr.is_multicast() {
            None
        } else {
            Some(socket_addr)
        };
        self.msg_id_map.remove(&(msg_id, socket_addr));
        self.msg_token_map.remove(&(msg_token, socket_addr));
    }
}
