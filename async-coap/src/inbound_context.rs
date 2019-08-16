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

/// Represents the context for processing an inbound message.
pub trait InboundContext: Send {
    /// The `SocketAddr` type from the associated `LocalEndpoint`.
    type SocketAddr: SocketAddrExt;

    /// Returns a copy of the remote address of the inbound message.
    fn remote_socket_addr(&self) -> Self::SocketAddr;

    /// Indicates if the endpoint thinks this message is a duplicate. This is used
    /// for non-idempotent methods (like POST) to determine if the operation should
    /// have real effects or if it should just go through the motions without changing
    /// state. Duplicates are generally only passed through when the underlying transport
    /// doesn't support support storing sent replies for this purpose.
    fn is_dupe(&self) -> bool;

    /// Returns a reference to a MessageRead trait to inspect the content
    /// of the inbound message.
    fn message(&self) -> &dyn MessageRead;
}

/// Represents the context for processing an inbound request that can be responded to.
pub trait RespondableInboundContext: InboundContext {
    /// Indicates if the inbound request was a multicast request or not. Multicast
    /// requests have additional response timing requirements in order to avoid
    /// congestion.
    fn is_multicast(&self) -> bool;

    /// Indicates if this inbound request is from a real inbound request or if it
    /// is a fake request that is being generated internally to solicit a response.
    /// Fake requests are only generated for the `GET` method.
    fn is_fake(&self) -> bool;

    /// Responds to this inbound request using a message generated from `msg_gen`.
    /// The `msg_id` and `msg_token` fields will be automatically populated.
    /// This method will return the value returned by `msg_gen`.
    fn respond<F>(&self, msg_gen: F) -> Result<(), Error>
    where
        F: Fn(&mut dyn MessageWrite) -> Result<(), Error>;
}
