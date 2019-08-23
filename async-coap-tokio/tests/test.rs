#![feature(async_await)]

use async_coap::datagram::DatagramLocalEndpoint;
use async_coap::prelude::*;
use async_coap_tokio::TokioAsyncUdpSocket;
use futures::prelude::*;
use std::sync::Arc;
use tokio::executor::spawn;

#[tokio::test]
async fn test_tokio() {
    let socket = TokioAsyncUdpSocket::bind("[::]:0").expect("UDP bind failed");

    // Create a new local endpoint from the socket we just created,
    // wrapping it in a `Arc<>` to ensure it can live long enough.
    let local_endpoint = Arc::new(DatagramLocalEndpoint::new(socket));

    // Add our local endpoint to the pool, so that it
    // can receive packets.
    spawn(
        local_endpoint
            .clone()
            .receive_loop_arc(null_receiver!())
            .map(|err| panic!("Receive loop terminated: {}", err)),
    );

    // Create a remote endpoint instance to represent the
    // device we wish to interact with.
    let remote_endpoint = local_endpoint
        .remote_endpoint_from_uri(uri!("coap://coap.me"))
        .unwrap(); // Will only fail if the URI scheme or authority is unrecognizable

    // Create a future that sends a request to a specific path
    // on the remote endpoint, collecting any blocks in the response
    // and returning `Ok(OwnedImmutableMessage)` upon success.
    let future = remote_endpoint.send_to(
        rel_ref!("large"),
        CoapRequest::get() // This is a CoAP GET request
            .accept(ContentFormat::TEXT_PLAIN_UTF8) // We only want plaintext
            .block2(Some(Default::default())) // Enable block2 processing
            .emit_successful_collected_response(), // Collect all blocks into a single message
    );

    // Wait until we get the result of our request.
    let result = future.await;

    assert!(result.is_ok(), "Error: {:?}", result.err().unwrap());
}
