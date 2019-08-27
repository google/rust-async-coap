async-coap: An experimental, asynchronous CoAP library
======================================================

[![Build Status](https://travis-ci.org/google/rust-async-coap.svg?branch=master)](https://travis-ci.org/google/rust-async-coap)
[![Crates.io](https://img.shields.io/crates/v/async-coap.svg)](https://crates.io/crates/async-coap)
[![API](https://docs.rs/async-coap/badge.svg)](https://docs.rs/async-coap)

## Introduction ##

`rust-async-coap` is an experimental, asynchronous Rust library for
using and serving Constrained Application Protocol (CoAP) resources.

This library provides a flexible, [asynchronous](https://rust-lang-nursery.github.io/futures-rs/)
interface for using and serving CoAP resources. A back-end that wraps around Rust's standard
`UdpSocket` is included, but can be replaced with one supporting DTLS, SMS, or whatever else
you might think of.

See the [crate documentation](https://docs.rs/async-coap) for more information.

## What is CoAP? ##

[CoAP][RFC7252] is a RESTful application protocol (like HTTP) designed for highly embedded
devices and small packet sizes. It is similar to HTTP in the sense that it is request/response
based, has methods like `GET`/`PUT`/`POST`, and uses hierarchical URLs to identify resources;
but unlike HTTP it is optimized for machine-to-machine interactions and includes mechanisms for
asynchronously [observing resources][RFC7641] for changes.

CoAP is the basis for several [upcoming][iot-1] [IoT][iot-2] [protocols][iot-3].

[RFC7252]: https://tools.ietf.org/html/rfc7252
[RFC7641]: https://tools.ietf.org/html/rfc7641
[iot-1]: https://github.com/google/splot-java#splot-for-java
[iot-2]: https://iot.mozilla.org/
[iot-3]: https://www.omaspecworks.org/what-is-oma-specworks/iot/lightweight-m2m-lwm2m/

## Usage ##

Add this to your `Cargo.toml`:

```toml
[dependencies]
async-coap = "0.1.0"
```

Now, you can use rust-async-coap:

```rust
use async_coap::prelude::*;
```

## License ##

rust-async-coap is released under the [Apache 2.0 license](LICENSE).

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
