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
use crate::message::BufferMessageEncoder;
use crate::message::CoapByteDisplayFormatter;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

/// Generic, datagram-based CoAP local endpoint implementation.
#[derive(Debug)]
pub struct DatagramLocalEndpoint<US: AsyncDatagramSocket>
where
    Self: Send + Sync,
{
    inner: Arc<DatagramLocalEndpointInner<US>>,
}

#[derive(Debug)]
pub(crate) struct DatagramLocalEndpointInner<US: AsyncDatagramSocket> {
    socket: US,
    next_msg_id: std::sync::atomic::AtomicU16,
    response_tracker: Mutex<UdpResponseTracker<DatagramInboundContext<US::SocketAddr>>>,
    scheme: &'static str,
    default_port: u16,
}

impl<US: AsyncDatagramSocket> DatagramLocalEndpointInner<US> {
    pub(crate) fn socket(&self) -> &US {
        &self.socket
    }

    pub(crate) fn next_msg_id(&self) -> MsgId {
        self.next_msg_id.fetch_add(1, Ordering::Relaxed)
    }

    pub(crate) fn scheme(&self) -> &'static str {
        self.scheme
    }

    pub(crate) fn default_port(&self) -> u16 {
        self.default_port
    }

    pub(crate) fn add_response_handler<'a>(
        &self,
        msg_id: MsgId,
        msg_token: MsgToken,
        socket_addr: US::SocketAddr,
        handler: Arc<Mutex<dyn HandleResponse<DatagramInboundContext<US::SocketAddr>> + 'a>>,
    ) {
        let mut tracker = self.response_tracker.lock().expect("Lock failed");

        tracker.add_response_handler(msg_id, msg_token, socket_addr, handler);
    }

    pub(crate) fn remove_response_handler(
        &self,
        msg_id: MsgId,
        msg_token: MsgToken,
        socket_addr: US::SocketAddr,
    ) {
        let mut guard = match self.response_tracker.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                debug!("Recovering from mutex poisoning");
                poisoned.into_inner()
            }
        };

        guard.remove_response_handler(msg_id, msg_token, socket_addr)
    }
}

impl<US: AsyncDatagramSocket> DatagramLocalEndpoint<US> {
    /// Creates a new [`DatagramLocalEndpoint`] instance with the given [`AsyncDatagramSocket`]
    /// and the standard scheme (`coap:`) and default port (5683).
    pub fn new(socket: US) -> DatagramLocalEndpoint<US> {
        Self::with_scheme_and_port(socket, URI_SCHEME_COAP, DEFAULT_PORT_COAP_UDP)
    }

    /// Creates a new [`DatagramLocalEndpoint`] instance with the given [`AsyncDatagramSocket`],
    /// using the specified scheme and default port.
    pub fn with_scheme_and_port(
        socket: US,
        scheme: &'static str,
        default_port: u16,
    ) -> DatagramLocalEndpoint<US> {
        DatagramLocalEndpoint {
            inner: Arc::new(DatagramLocalEndpointInner {
                socket,
                next_msg_id: std::sync::atomic::AtomicU16::new(1),
                response_tracker: Mutex::new(UdpResponseTracker::new()),
                scheme,
                default_port,
            }),
        }
    }

    /// Borrows a reference to the underlying socket.
    pub fn socket(&self) -> &US {
        self.inner.socket()
    }
}

impl<US: AsyncDatagramSocket> LocalEndpoint for DatagramLocalEndpoint<US> {
    type SocketAddr = US::SocketAddr;
    type SocketError = US::Error;
    type DefaultTransParams = StandardCoapConstants;
    type LookupStream = futures::stream::Iter<std::vec::IntoIter<Self::SocketAddr>>;
    type RespondableInboundContext = DatagramRespondableInboundContext<Self::SocketAddr>;
    type InboundContext = DatagramInboundContext<Self::SocketAddr>;

    type RemoteEndpoint = DatagramRemoteEndpoint<US>;

    fn remote_endpoint<S, H, P>(&self, addr: S, host: Option<H>, path: P) -> Self::RemoteEndpoint
    where
        S: ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::SocketError>,
        H: Into<String>,
        P: Into<RelRefBuf>,
    {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        DatagramRemoteEndpoint::new(&self.inner, addr, host.map(|h| h.into()), path.into())
    }

    fn remote_endpoint_from_uri(&self, uri: &Uri) -> Result<Self::RemoteEndpoint, Error> {
        if let Some(scheme) = uri.scheme() {
            if scheme != self.scheme() {
                return Err(Error::UnsupportedUriScheme);
            }
        }

        if let Some((_userinfo, host, port)) = uri.raw_userinfo_host_port() {
            let host = host
                .unescape_uri()
                .try_to_cow()
                .expect("Host in URI is corrupted");

            let mut lookup_stream = self.lookup(&host, port.unwrap_or(0))?;

            // TODO: Eventually remove the call to "now_or_never()"
            if let Some(socket_addr) = lookup_stream
                .next()
                .now_or_never()
                .expect("Lookup stream not ready")
            {
                Ok(self.remote_endpoint(socket_addr, Some(host), uri.trim_fragment().rel()))
            } else {
                Err(Error::HostNotFound)
            }
        } else {
            Err(Error::HostNotFound)
        }
    }

    fn send<'a, S, R, SD>(&'a self, dest: S, send_desc: SD) -> BoxFuture<'a, Result<R, Error>>
    where
        S: ToSocketAddrs<SocketAddr = Self::SocketAddr, Error = Self::SocketError> + 'a,
        SD: SendDesc<Self::InboundContext, R> + 'a,
        R: Send + 'a,
    {
        match dest.to_socket_addrs() {
            Ok(mut iter) => match iter.next() {
                Some(socket_addr) => {
                    if let Some(trans_params) = send_desc.trans_params() {
                        UdpSendFuture::new(&self.inner, socket_addr, send_desc, trans_params)
                            .boxed()
                    } else {
                        UdpSendFuture::new(
                            &self.inner,
                            socket_addr,
                            send_desc,
                            StandardCoapConstants,
                        )
                        .boxed()
                    }
                }
                None => futures::future::ready(Err(Error::HostNotFound)).boxed(),
            },
            Err(_) => futures::future::ready(Err(Error::HostLookupFailure)).boxed(),
        }
    }

    fn receive<'a, F>(&'a self, mut handler: F) -> BoxFuture<'a, Result<(), Error>>
    where
        F: FnMut(&Self::RespondableInboundContext) -> Result<(), Error> + 'a + Send,
    {
        async move {
            let mut buffer = [0u8; StandardCoapConstants::MAX_OUTBOUND_PACKET_LENGTH];
            let (len, source, dest) = match self.socket().next_recv_from(&mut buffer).await {
                Ok(x) => x,
                Err(_) => return Err(Error::IOError),
            };
            let buffer = &buffer[..len];
            debug!("INBOUND: {} {}", source, CoapByteDisplayFormatter(buffer));

            let is_multicast = match dest {
                Some(local_addr) => local_addr.is_multicast(),
                None => false,
            };

            let inbound_context: Self::RespondableInboundContext =
                DatagramRespondableInboundContext::new(buffer.to_vec(), source, is_multicast)?;

            let msg_code = inbound_context.message().msg_code();
            let msg_type = inbound_context.message().msg_type();
            let msg_id = inbound_context.message().msg_id();

            let ret = if msg_code.is_method() {
                // This is a request
                debug!("Message is a request.");
                handler(&inbound_context)?;

                if let Some(message) = inbound_context.into_message_out() {
                    if let Some(e) = self.socket().next_send_to(&message, source).await.err() {
                        error!("send_to: io error: {:?} (dest={:?})", e, source);
                    }
                } else {
                    let mut buffer = [0u8; 12];
                    let mut builder = BufferMessageEncoder::new(&mut buffer);

                    builder.set_msg_id(msg_id);

                    let _ = message::ResetMessage.write_msg_to(&mut builder);

                    if let Some(e) = self.socket().next_send_to(&builder, source).await.err() {
                        error!("send_to: io error: {:?} (dest={:?})", e, source);
                    }
                }
                Ok(())
            } else if !msg_code.is_empty() || msg_type.is_ack() || msg_type.is_res() {
                // This is a response
                debug!("Message is a response.");
                let was_handled = {
                    let mut tracker = self.inner.response_tracker.lock().expect("Lock failed");
                    tracker.handle_response(&inbound_context)
                };
                debug!("was_handled: {}", was_handled);

                // Drop the inbound context so that we don't cross a `.await` holding it.
                core::mem::drop(inbound_context);

                if msg_type.is_con() {
                    let mut buffer = [0u8; 12];
                    let mut builder = BufferMessageEncoder::new(&mut buffer);
                    builder.set_msg_id(msg_id);

                    if was_handled {
                        let _ = message::AckMessage.write_msg_to(&mut builder);
                    } else {
                        let _ = message::ResetMessage.write_msg_to(&mut builder);
                    }

                    if let Some(e) = self.socket().next_send_to(&builder, source).await.err() {
                        error!("send_to: io error: {:?} (dest={:?})", e, source);
                        Err(Error::IOError)
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                }
            } else if msg_code.is_empty() || msg_type.is_con() {
                // Send reset

                let mut buffer = [0u8; 12];
                let mut builder = BufferMessageEncoder::new(&mut buffer);

                // Drop the inbound context so that we don't cross a `.await` holding it.
                core::mem::drop(inbound_context);

                builder.set_msg_id(msg_id);

                let _ = message::ResetMessage.write_msg_to(&mut builder);

                if let Some(e) = self.socket().next_send_to(&builder, source).await.err() {
                    error!("send_to: io error: {:?} (dest={:?})", e, source);
                }

                Ok(())
            } else {
                Err(Error::ParseFailure)
            };

            ret
        }
            .boxed()
    }

    fn scheme(&self) -> &'static str {
        self.inner.scheme
    }

    fn default_port(&self) -> u16 {
        self.inner.default_port
    }

    fn lookup(&self, hostname: &str, mut port: u16) -> Result<Self::LookupStream, Error> {
        if port == 0 {
            port = self.default_port();
        }

        match US::lookup_host(hostname, port) {
            Ok(iter) => {
                if let Some(local) = self.socket().local_addr().ok() {
                    let filtered_iter = iter.filter_map(|sockaddr| {
                        debug!("sockaddr: {:?}", sockaddr);
                        debug!("local: {:?}", local);
                        debug!(
                            "sockaddr.conforming_to(local): {:?}",
                            sockaddr.conforming_to(local)
                        );
                        sockaddr.conforming_to(local)
                    });
                    let filtered_vec: Vec<Self::SocketAddr> = filtered_iter.collect();
                    Ok(futures::stream::iter(filtered_vec.into_iter()))
                } else {
                    Ok(futures::stream::iter(iter))
                }
            }
            Err(_) => Err(Error::HostLookupFailure),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageDisplay;
    use crate::ContentFormat;
    use futures::executor::block_on;
    use futures::future::select;
    use futures::future::Either;
    use futures_timer::TryFutureExt;
    use std::time::Duration;

    fn test_process_request<LE, F, R>(local_endpoint: &LE, future: F) -> R
    where
        LE: LocalEndpoint,
        F: Future<Output = R> + Unpin,
        R: Send,
    {
        let future_receive = local_endpoint.receive_loop(null_receiver!());

        let combined_future = select(future, future_receive);

        let combined_result = block_on(combined_future);

        match combined_result {
            Either::Right(_) => panic!("Receive future finished unexpectedly"),
            Either::Left((ret, _)) => ret,
        }
    }

    #[test]
    fn ping_loopback() {
        let socket = LoopbackSocket::new();
        let local_endpoint = DatagramLocalEndpoint::new(socket);

        let dest = LoopbackSocketAddr::Unicast;
        let send_desc = Ping::new();

        let future = local_endpoint.send(dest, send_desc);
        assert_eq!(Ok(()), test_process_request(&local_endpoint, future));
    }

    /// Test that verifies that timeouts are working properly.
    /// This can currently take a while to execute, so it is currently disabled.
    #[test]
    #[ignore]
    fn ping_null() {
        let socket = NullSocket::new();
        let local_endpoint = DatagramLocalEndpoint::new(socket);
        let dest = NullSocketAddr;
        let send_desc = Ping::new();
        let future = local_endpoint.send(dest, send_desc);

        assert_eq!(
            Err(Error::ResponseTimeout),
            test_process_request(&local_endpoint, future)
        );
    }

    #[test]
    fn ping_localhost() {
        let socket = AllowStdUdpSocket::bind("[::]:12345").expect("UDP bind failed");
        let local_endpoint = DatagramLocalEndpoint::new(socket);
        let dest = "[::1]:12345";
        let send_desc = Ping::new();
        let future = local_endpoint.send(dest, send_desc);

        assert_eq!(Ok(()), test_process_request(&local_endpoint, future));
    }

    #[test]
    fn ping_coap_me() {
        let socket = AllowStdUdpSocket::bind("0.0.0.0:0").expect("UDP bind failed");
        let local_endpoint = DatagramLocalEndpoint::new(socket);

        let mut lookup_results = local_endpoint
            .lookup("coap.me", 5683)
            .expect("DNS lookup failure");
        let dest = block_on(lookup_results.next()).expect("DNS lookup failure");
        let send_desc = Ping::new();
        let future = local_endpoint.send(dest, send_desc);

        assert_eq!(Ok(()), test_process_request(&local_endpoint, future));
    }

    #[test]
    fn host_lookup_failure() {
        let socket = AllowStdUdpSocket::bind("[::]:0").expect("UDP bind failed");
        let local_endpoint = DatagramLocalEndpoint::new(socket);

        let dest = "[127.0.0.1]:5683";
        let send_desc = Ping::new();
        let future = local_endpoint.send(dest, send_desc);

        assert_eq!(
            Err(Error::HostLookupFailure),
            test_process_request(&local_endpoint, future)
        );
    }

    /// Test that performs a `GET` on <coap://coap.me/test>
    #[test]
    fn get_coap_me_test() {
        let socket = AllowStdUdpSocket::bind("0.0.0.0:0").expect("UDP bind failed");
        let local_endpoint = DatagramLocalEndpoint::new(socket);

        let remote_endpoint = local_endpoint
            .remote_endpoint_from_uri(uri!("coap://coap.me/"))
            .expect("client construct failed");

        let send_desc = CoapRequest::get()
            .add_option(option::ACCEPT, ContentFormat::APPLICATION_JSON)
            .inspect(|ctx| {
                debug!("Got Response: {}", MessageDisplay(ctx.message()));
                assert_eq!(ctx.message().msg_code(), MsgCode::SuccessContent);
            });

        let future = remote_endpoint.send_to(rel_ref!("test"), send_desc);

        assert_eq!(Ok(()), test_process_request(&local_endpoint, future));
    }

    /// Test that performs a `GET` on <coap://coap.me/bl%C3%A5b%C3%A6rsyltet%C3%B8y>
    #[test]
    fn get_coap_me_blabaersyltetoy() {
        let socket = AllowStdUdpSocket::bind("0.0.0.0:0").expect("UDP bind failed");
        let local_endpoint = DatagramLocalEndpoint::new(socket);

        let remote_endpoint = local_endpoint
            .remote_endpoint_from_uri(uri!("coap://coap.me/bl%C3%A5b%C3%A6rsyltet%C3%B8y"))
            .expect("client construct failed");

        let send_desc = CoapRequest::get().emit_msg_code();

        let future = remote_endpoint.send(send_desc);

        assert_eq!(
            Ok(MsgCode::SuccessContent),
            test_process_request(&local_endpoint, future)
        );
    }

    /// Test that performs a `GET` on <coap://coap.me/separate>
    #[test]
    fn get_coap_me_separate() {
        let socket = AllowStdUdpSocket::bind("0.0.0.0:0").expect("UDP bind failed");
        let local_endpoint = DatagramLocalEndpoint::new(socket);

        let remote_endpoint = local_endpoint
            .remote_endpoint_from_uri(uri!("coap://coap.me/"))
            .expect("client construct failed");

        let send_desc = CoapRequest::get().inspect(|ctx| {
            debug!("Got Response: {}", MessageDisplay(ctx.message()));
            assert_eq!(ctx.message().msg_code(), MsgCode::SuccessContent);
        });

        let future = remote_endpoint.send_to(rel_ref!("separate"), send_desc);

        assert_eq!(Ok(()), test_process_request(&local_endpoint, future));
    }

    /// Test that performs a `GET` on <coap://coap.me/large>, which exercises the block2 stuff.
    #[test]
    fn get_coap_me_large() {
        let socket = AllowStdUdpSocket::bind("0.0.0.0:0").expect("UDP bind failed");
        let local_endpoint = DatagramLocalEndpoint::new(socket);

        let remote_endpoint = local_endpoint
            .remote_endpoint_from_uri(uri!("coap://coap.me/large"))
            .expect("client construct failed");

        debug!("Requesting <{}>", remote_endpoint.uri());

        let send_desc = CoapRequest::get()
            .accept(ContentFormat::TEXT_PLAIN_UTF8)
            .block2(None)
            .emit_successful_collected_response()
            .inspect(|ctx| {
                assert!(ctx.message().msg_code().is_success());
            });

        let future = remote_endpoint
            .send(send_desc)
            .timeout(Duration::from_secs(5));

        let result = test_process_request(&local_endpoint, future);
        assert!(result.is_ok(), "{:?}", result);

        let collected_message = result.unwrap();
        let content_string = collected_message.payload_as_str().unwrap();

        debug!("Full response: {}", content_string);
        assert_eq!(1700, content_string.len());
    }

    /// Test that performs a `GET` on <coap://coap.me/path/sub1>
    #[test]
    fn get_coap_me_path_sub1() {
        let socket = AllowStdUdpSocket::bind("0.0.0.0:0").expect("UDP bind failed");
        let local_endpoint = DatagramLocalEndpoint::new(socket);

        let remote_endpoint = local_endpoint
            .remote_endpoint_from_uri(uri!("coap://coap.me/"))
            .expect("client construct failed");

        let send_desc = CoapRequest::get()
            .accept(ContentFormat::APPLICATION_JSON)
            .add_option_iter(option::URI_PATH, vec!["path", "sub1"])
            .inspect(|ctx| {
                assert_eq!(ctx.message().msg_code(), MsgCode::SuccessContent);
                assert_eq!(
                    ctx.message().content_format(),
                    Some(ContentFormat::APPLICATION_JSON)
                );
            });

        let future = remote_endpoint.send(send_desc);

        assert_eq!(Ok(()), test_process_request(&local_endpoint, future));
    }

    #[test]
    fn client_get_coap_me_path_sub2() {
        let socket = AllowStdUdpSocket::bind("0.0.0.0:0").expect("UDP bind failed");

        let local_endpoint = DatagramLocalEndpoint::new(socket);

        let remote_endpoint = local_endpoint
            .remote_endpoint_from_uri(uri!("coap://coap.me/path/"))
            .expect("client construct failed");

        let send_desc = CoapRequest::get()
            .accept(ContentFormat::APPLICATION_JSON)
            .add_option_iter(option::URI_PATH, vec![])
            .inspect(|ctx| {
                assert_eq!(ctx.message().msg_code(), MsgCode::SuccessContent);
            });

        let future = remote_endpoint.send_to(rel_ref!("sub2"), send_desc);

        assert_eq!(Ok(()), test_process_request(&local_endpoint, future));
    }
}
