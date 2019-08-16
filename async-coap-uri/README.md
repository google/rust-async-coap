async-coap-uri: Safe, In-place URI Abstraction
==============================================

[![Build Status](https://travis-ci.com/google/rust-async-coap.svg?branch=master)](https://travis-ci.com/google/rust-async-coap)
[![Crates.io](https://img.shields.io/crates/v/async-coap-uri.svg)](https://crates.io/crates/async-coap-uri)
[![API](https://docs.rs/async-coap-uri/badge.svg)](https://docs.rs/async-coap-uri)

This crate provides safe, efficient, full-featured support for using and
manipulating [Uniform Resource Identifiers][IETF-RFC3986].

It differs significantly from the [rust-url][] and [uri][rust-uri]
in that the API was designed to allow for more control over how
memory is allocated while also being more convenient to use.

[IETF-RFC3986]: https://tools.ietf.org/html/rfc3986
[rust-url]: https://docs.rs/url
[rust-uri]: https://docs.rs/uri

What makes this crate unique is that it provides URI-specific types
that have the same unsized/sized type duality that is used for
`&str`/`String`, except with specific guarantees on the content,
as well as convenient domain-specific methods to access the URI
components. The API was designed to be easy-to-use while making as few
heap allocations as possible. Most common operations require no
allocations at all, and those that do are provided as a convenience
rather than a fundamental requirement. For example, you can parse and
fully percent-decode a URI without doing a single allocation, including
iterating over percent-decoded path segments and query key-value pairs.

See the [crate documentation](https://docs.rs/async-coap-uri) for more information.

## Usage ##

Add this to your `Cargo.toml`:

```toml
[dependencies]
async-coap-uri = "0.1.0"
```

Now, you can use async-coap-uri:

```rust
use async_coap_uri::prelude::*;
```

## License ##

async-coap-uri is released under the [Apache 2.0 license](LICENSE).

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
