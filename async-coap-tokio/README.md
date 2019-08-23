[Tokio][]-based [`AsyncDatagramSocket`][] wrapper for [`async-coap`][]
======================================================================

[![Crates.io](https://img.shields.io/crates/v/async-coap-tokio.svg)](https://crates.io/crates/async-coap-tokio)
[![API](https://docs.rs/async-coap-tokio/badge.svg)](https://docs.rs/async-coap-tokio)

This crate provides `TokioAsyncUdpSocket`: an asynchronous, [Tokio][]-based
implementation of [`AsyncDatagramSocket`] for use with [`DatagramLocalEndpoint`].

[`AsyncDatagramSocket`]: https://docs.rs/async-coap/0.1/async_coap/datagram/trait.AsyncDatagramSocket.html
[`DatagramLocalEndpoint`]: https://docs.rs/async-coap/0.1/async_coap/datagram/trait.DatagramLocalEndpoint.html
[Tokio]: https://tokio.rs/

See the [crate documentation](https://docs.rs/async-coap-tokio) for more information.

## Usage ##

Add this to your `Cargo.toml`:

```toml
[dependencies]
async-coap = "0.1"
async-coap-tokio = "0.1"
```

## License ##

async-coap-tokio is released under the [Apache 2.0 license](../LICENSE).

    Copyright (c) 2019 Google LLC

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

        http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.


## Disclaimer ##

This is not an officially supported Google product.
