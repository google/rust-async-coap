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
use crate::option::OptionValue;

impl<SD: SendDescUnicast, K, I: Send, IC> SendDescUnicast for AddOption<SD, K, I, IC> {}
impl<SD: SendDescMulticast, K, I: Send, IC> SendDescMulticast for AddOption<SD, K, I, IC> {}

/// Combinator for Send Descriptors created by [`SendDescExt::add_option`].
#[derive(Debug)]
pub struct AddOption<SD, K, I: Send, IC> {
    pub(super) inner: SD,
    pub(super) key: OptionKey<K>,
    pub(super) viter: I,
    pub(super) phantom: PhantomData<IC>,
}

impl<'a, SD, IC, R, K, I> SendDesc<IC, R> for AddOption<SD, K, I, IC>
where
    SD: SendDesc<IC, R> + Send,
    IC: InboundContext,
    R: Send,
    I: IntoIterator<Item = K> + Clone + Send,
    K: Into<OptionValue<'a>>,
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
        write_options!((msg, socket_addr, start, end, self.inner) {
            self.key => self.viter.clone(),
        })
    }
}
