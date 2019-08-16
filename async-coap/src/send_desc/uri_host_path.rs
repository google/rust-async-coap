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
use std::marker::PhantomData;

impl<SD: SendDescUnicast, IC> SendDescUnicast for UriHostPath<SD, IC> {}
impl<SD: SendDescMulticast, IC> SendDescMulticast for UriHostPath<SD, IC> {}

/// Combinator for Send Descriptors created by [`SendDescExt::uri_host_path`].
#[derive(Debug)]
pub struct UriHostPath<SD, IC> {
    pub(super) inner: SD,
    pub(super) host: Option<String>,
    pub(super) path_and_query: RelRefBuf,
    pub(super) phantom: PhantomData<IC>,
}

impl<SD, IC, R> SendDesc<IC, R> for UriHostPath<SD, IC>
where
    SD: SendDesc<IC, R>,
    IC: InboundContext,
    R: Send,
{
    send_desc_passthru_timing!(inner);
    send_desc_passthru_handler!(inner, R);
    send_desc_passthru_payload!(inner);

    fn write_options(
        &self,
        msg: &mut dyn OptionInsert,
        socket_addr: &IC::SocketAddr,
        start: Bound<OptionNumber>,
        end: Bound<OptionNumber>,
    ) -> Result<(), Error> {
        // We define this up here at the top of the method so that it doesn't
        // go out of scope.
        let mut unescape_buf;

        write_options!((msg, socket_addr, start, end, self.inner) {
            URI_HOST => self.host.iter(),
            URI_PATH => {
                unescape_buf = self.path_and_query.clone().into_unescape_buf();
                unescape_buf.path_segments()
            },
            URI_QUERY => {
                unescape_buf = self.path_and_query.clone().into_unescape_buf();
                unescape_buf.query_items()
            },
        })
    }
}
