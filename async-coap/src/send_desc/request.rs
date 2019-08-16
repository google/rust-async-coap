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

/// Seed combinator used for creating send descriptors for CoAP requests.
#[derive(Debug)]
pub enum CoapRequest {}

impl CoapRequest {
    /// Constructs a simple GET send descriptor.
    ///
    /// The generic parameter `IC` can (for the most part) be ignored: the type will be
    /// inferred when the send descriptor is passed to [`LocalEndpoint::send`] (or one of its
    /// [many][RemoteEndpoint::send_to] [variants][RemoteEndpoint::send]).
    #[inline(always)]
    pub fn get<IC>() -> SendGet<IC> {
        Default::default()
    }

    /// Constructs a simple GET send descriptor configured for observing.
    ///
    /// The generic parameter `IC` can (for the most part) be ignored: the type will be
    /// inferred when the send descriptor is passed to [`LocalEndpointExt::send_as_stream`] (or
    /// one of its [many][RemoteEndpointExt::send_to_as_stream]
    /// [variants][RemoteEndpointExt::send_as_stream]).
    #[inline(always)]
    pub fn observe<IC>() -> SendObserve<IC> {
        Default::default()
    }

    /// Constructs a simple POST send descriptor.
    ///
    /// The generic parameter `IC` can (for the most part) be ignored: the type will be
    /// inferred when the send descriptor is passed to [`LocalEndpoint::send`] (or one of its
    /// [many][RemoteEndpoint::send_to] [variants][RemoteEndpoint::send]).
    #[inline(always)]
    pub fn post<IC>() -> SendPost<IC> {
        Default::default()
    }

    /// Constructs a simple PUT send descriptor.
    ///
    /// The generic parameter `IC` can (for the most part) be ignored: the type will be
    /// inferred when the send descriptor is passed to [`LocalEndpoint::send`] (or one of its
    /// [many][RemoteEndpoint::send_to] [variants][RemoteEndpoint::send]).
    #[inline(always)]
    pub fn put<IC>() -> SendPut<IC> {
        Default::default()
    }

    /// Constructs a simple DELETE send descriptor.
    ///
    /// The generic parameter `IC` can (for the most part) be ignored: the type will be
    /// inferred when the send descriptor is passed to [`LocalEndpoint::send`] (or one of its
    /// [many][RemoteEndpoint::send_to] [variants][RemoteEndpoint::send]).
    #[inline(always)]
    pub fn delete<IC>() -> SendDelete<IC> {
        Default::default()
    }

    /// Constructs a simple send descriptor with an arbitrary CoAP method code.
    ///
    /// The value of `msg_code` is checked in debug mode to ensure it is a CoAP method.
    /// The value is not checked in release mode.
    ///
    /// The generic parameter `IC` can (for the most part) be ignored: the type will be
    /// inferred when the send descriptor is passed to [`LocalEndpoint::send`] (or one of its
    /// [many][RemoteEndpoint::send_to] [variants][RemoteEndpoint::send]).
    #[inline(always)]
    pub fn method<IC>(msg_code: MsgCode) -> CoapRequestMethod<IC> {
        debug_assert!(msg_code.is_method(), "{:?} is not a method", msg_code);
        CoapRequestMethod {
            msg_code,
            phantom: PhantomData,
        }
    }
}

macro_rules! send_desc_def_method {
    ($(#[$tags:meta])* $name:ident, $code:expr, $handler:expr) => {
        $(#[$tags])*
        #[derive(Debug)]
        pub struct $ name < IC > (PhantomData < IC > );
        send_desc_def_method!(@rest ($name,$code,$handler));
    };
    ($name:ident, $code:expr, $handler:expr) => {
        pub struct $ name < IC > (PhantomData < IC > );
        send_desc_def_method!(@rest ($name,$code,$handler));
    };
    (@rest ($name:ident, $code:expr, $handler:expr)) => {
        impl<IC> SendDescUnicast for $name<IC> {}

        impl<IC> Default for $name<IC> {
            #[inline(always)]
            fn default() -> Self {
                Self(PhantomData)
            }
        }

        impl<IC> $name<IC> {
            /// Returns a nonconfirmable version of this send descriptor.
            #[inline(always)]
            pub fn nonconfirmable(self) -> Nonconfirmable<$name<IC>> {
                Nonconfirmable(self)
            }

            /// Returns a multicast version of this send descriptor.
            #[inline(always)]
            pub fn multicast(self) -> Multicast<$name<IC>> {
                Multicast(self)
            }
        }

        impl<IC: InboundContext> SendDesc<IC, ()> for $name<IC> {
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
                msg.set_msg_code($code);
                Ok(())
            }

            fn handler(
                &mut self,
                context: Result<&IC, Error>,
            ) -> Result<ResponseStatus<()>, Error> {
                let context = context?;

                if context.is_dupe() {
                    // Ignore dupes.
                    return Ok(ResponseStatus::Continue);
                }

                let code = context.message().msg_code();
                ($handler)(code)
            }
        }
    };
}

send_desc_def_method!(
    /// Send descriptor created by [`CoapRequest::get`] used for sending CoAP GET requests.
    SendGet,
    MsgCode::MethodGet,
    |code| {
        match code {
            MsgCode::SuccessContent | MsgCode::SuccessValid => Ok(ResponseStatus::Done(())),
            MsgCode::ClientErrorNotFound => Err(Error::ResourceNotFound),
            MsgCode::ClientErrorForbidden => Err(Error::Forbidden),
            MsgCode::ClientErrorUnauthorized => Err(Error::Unauthorized),
            code if code.is_client_error() => Err(Error::ClientRequestError),
            _ => Err(Error::ServerError),
        }
    }
);

send_desc_def_method!(
    /// Send descriptor created by [`CoapRequest::put`] used for sending CoAP PUT requests.
    SendPut,
    MsgCode::MethodPut,
    |code| {
        match code {
            MsgCode::SuccessCreated | MsgCode::SuccessChanged | MsgCode::SuccessValid => {
                Ok(ResponseStatus::Done(()))
            }
            MsgCode::ClientErrorNotFound => Err(Error::ResourceNotFound),
            MsgCode::ClientErrorForbidden => Err(Error::Forbidden),
            MsgCode::ClientErrorUnauthorized => Err(Error::Unauthorized),
            code if code.is_client_error() => Err(Error::ClientRequestError),
            _ => Err(Error::ServerError),
        }
    }
);

send_desc_def_method!(
    /// Send descriptor created by [`CoapRequest::post`] used for sending CoAP POST requests.
    SendPost,
    MsgCode::MethodPost,
    |code| {
        match code {
            code if code.is_success() => Ok(ResponseStatus::Done(())),
            MsgCode::ClientErrorNotFound => Err(Error::ResourceNotFound),
            MsgCode::ClientErrorForbidden => Err(Error::Forbidden),
            MsgCode::ClientErrorUnauthorized => Err(Error::Unauthorized),
            code if code.is_client_error() => Err(Error::ClientRequestError),
            _ => Err(Error::ServerError),
        }
    }
);

send_desc_def_method!(
    /// Send descriptor created by [`CoapRequest::delete`] used for sending CoAP DELETE requests.
    SendDelete,
    MsgCode::MethodDelete,
    |code| {
        match code {
            MsgCode::SuccessDeleted => Ok(ResponseStatus::Done(())),
            MsgCode::ClientErrorNotFound => Err(Error::ResourceNotFound),
            MsgCode::ClientErrorForbidden => Err(Error::Forbidden),
            MsgCode::ClientErrorUnauthorized => Err(Error::Unauthorized),
            code if code.is_client_error() => Err(Error::ClientRequestError),
            _ => Err(Error::ServerError),
        }
    }
);

/// Send descriptor created by [`CoapRequest::method`] used for sending CoAP requests with a
/// programmatically defined method.
#[derive(Debug)]
pub struct CoapRequestMethod<IC> {
    msg_code: MsgCode,
    phantom: PhantomData<IC>,
}

impl<IC> SendDescUnicast for CoapRequestMethod<IC> {}

impl<IC> CoapRequestMethod<IC> {
    /// Returns a nonconfirmable version of this send descriptor.
    #[inline(always)]
    pub fn nonconfirmable(self) -> Nonconfirmable<CoapRequestMethod<IC>> {
        Nonconfirmable(self)
    }

    /// Returns a multicast version of this send descriptor.
    #[inline(always)]
    pub fn multicast(self) -> Multicast<CoapRequestMethod<IC>> {
        Multicast(self)
    }
}

impl<IC> SendDesc<IC, ()> for CoapRequestMethod<IC>
where
    IC: InboundContext,
{
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
        msg.set_msg_code(self.msg_code);
        Ok(())
    }

    fn handler(&mut self, context: Result<&IC, Error>) -> Result<ResponseStatus<()>, Error> {
        let context = context?;

        if context.is_dupe() {
            // Ignore dupes.
            return Ok(ResponseStatus::Continue);
        }

        let code = context.message().msg_code();

        match code {
            code if code.is_success() => Ok(ResponseStatus::Done(())),
            MsgCode::ClientErrorNotFound => Err(Error::ResourceNotFound),
            MsgCode::ClientErrorForbidden => Err(Error::Forbidden),
            MsgCode::ClientErrorUnauthorized => Err(Error::Unauthorized),
            code if code.is_client_error() => Err(Error::ClientRequestError),
            _ => Err(Error::ServerError),
        }
    }
}
