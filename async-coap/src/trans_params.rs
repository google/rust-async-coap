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

use std::time::Duration;

/// Trait defining [CoAP transmission parameters][tp]. Experimental.
///
/// [tp]: https://tools.ietf.org/html/rfc7252#section-4.8
#[doc(hidden)]
pub trait TransParams: Default + Copy + Sync + Send + Unpin {
    fn max_outbound_packet_length(&self) -> usize {
        Self::MAX_OUTBOUND_PACKET_LENGTH
    }

    fn coap_max_retransmit(&self) -> u32 {
        Self::COAP_MAX_RETRANSMIT
    }

    fn coap_ack_timeout(&self) -> Duration {
        Self::COAP_ACK_TIMEOUT
    }

    fn coap_ack_random_factor(&self) -> f32 {
        Self::COAP_ACK_RANDOM_FACTOR
    }

    fn coap_nstart(&self) -> u32 {
        Self::COAP_NSTART
    }

    fn coap_default_leisure(&self) -> Duration {
        Self::COAP_DEFAULT_LEISURE
    }

    fn coap_probing_rate(&self) -> u32 {
        Self::COAP_PROBING_RATE
    }

    fn coap_max_latency(&self) -> Duration {
        Self::COAP_MAX_LATENCY
    }

    fn coap_processing_delay(&self) -> Duration {
        self.coap_ack_timeout()
    }

    fn coap_max_transmit_span(&self) -> Duration {
        Self::COAP_MAX_TRANSMIT_SPAN
    }

    fn coap_max_transmit_wait(&self) -> Duration {
        Self::COAP_MAX_TRANSMIT_WAIT
    }

    fn coap_max_rtt(&self) -> Duration {
        Self::COAP_MAX_RTT
    }

    fn coap_exchange_lifetime(&self) -> Duration {
        Self::COAP_EXCHANGE_LIFETIME
    }

    fn coap_non_lifetime(&self) -> Duration {
        Self::COAP_NON_LIFETIME
    }

    const MAX_OUTBOUND_PACKET_LENGTH: usize = 1152;

    const COAP_MAX_RETRANSMIT: u32 = 4;

    const COAP_ACK_TIMEOUT: Duration = Duration::from_secs(2);

    const COAP_ACK_RANDOM_FACTOR: f32 = 1.5;

    const COAP_NSTART: u32 = 1;

    const COAP_DEFAULT_LEISURE: Duration = Duration::from_secs(5);

    /// CoAP probing rate, measured in bytes per second.
    const COAP_PROBING_RATE: u32 = 1;

    /// From RFC7252:
    ///
    /// > `MAX_LATENCY` is the maximum time a datagram is expected to take
    /// > from the start of its transmission to the completion of its
    /// > reception.  This constant is related to the MSL (Maximum Segment
    /// > Lifetime) of [RFC0793][IETF-RFC0793], which is "arbitrarily defined to be 2
    /// > minutes" ([RFC0793][IETF-RFC0793] glossary, page 81).  Note that this is not
    /// > necessarily smaller than `MAX_TRANSMIT_WAIT`, as `MAX_LATENCY` is not
    /// > intended to describe a situation when the protocol works well, but
    /// > the worst-case situation against which the protocol has to guard.
    /// > We, also arbitrarily, define `MAX_LATENCY` to be 100 seconds.  Apart
    /// > from being reasonably realistic for the bulk of configurations as
    /// > well as close to the historic choice for TCP, this value also allows
    /// > Message ID lifetime timers to be represented in 8 bits (when
    /// > measured in seconds).  In these calculations, there is no assumption
    /// > that the direction of the transmission is irrelevant (i.e., that the
    /// > network is symmetric); there is just the assumption that the same
    /// > value can reasonably be used as a maximum value for both directions.
    /// > If that is not the case, the following calculations become only
    /// > slightly more complex.
    ///
    /// [IETF-RFC0793]: https://tools.ietf.org/html/rfc793
    const COAP_MAX_LATENCY: Duration = Duration::from_secs(100);

    /// From RFC7252:
    ///
    /// > `PROCESSING_DELAY` is the time a node takes to turn around a
    /// > Confirmable message into an acknowledgement.  We assume the node
    /// > will attempt to send an ACK before having the sender time out, so as
    /// > a conservative assumption we set it equal to `ACK_TIMEOUT`.
    const COAP_PROCESSING_DELAY: Duration = Self::COAP_ACK_TIMEOUT;

    /// From RFC7252:
    ///
    /// > `MAX_TRANSMIT_SPAN` is the maximum time from the first transmission
    /// > of a Confirmable message to its last retransmission.  For the
    /// > default transmission parameters, the value is (2+4+8+16)*1.5 = 45
    /// > seconds, or more generally:
    /// >
    /// >> `ACK_TIMEOUT * ((2 ** MAX_RETRANSMIT) - 1) * ACK_RANDOM_FACTOR`
    const COAP_MAX_TRANSMIT_SPAN: Duration = Duration::from_millis(
        (Self::COAP_ACK_TIMEOUT.as_millis() as f32
            * (Self::COAP_MAX_RETRANSMIT * 2 - 1) as f32
            * Self::COAP_ACK_RANDOM_FACTOR) as u64,
    );

    /// From RFC7252:
    ///
    /// > `MAX_TRANSMIT_WAIT` is the maximum time from the first transmission
    /// > of a Confirmable message to the time when the sender gives up on
    /// > receiving an acknowledgement or reset.  For the default
    /// > transmission parameters, the value is (2+4+8+16+32)*1.5 = 93
    /// > seconds, or more generally:
    /// >
    /// >> `ACK_TIMEOUT * ((2 ** (MAX_RETRANSMIT + 1)) - 1) * ACK_RANDOM_FACTOR`
    const COAP_MAX_TRANSMIT_WAIT: Duration = Duration::from_millis(
        (Self::COAP_ACK_TIMEOUT.as_millis() as f32
            * ((Self::COAP_MAX_RETRANSMIT + 1) * 2 - 1) as f32
            * Self::COAP_ACK_RANDOM_FACTOR) as u64,
    );

    /// From RFC7252:
    ///
    /// > `MAX_RTT` is the maximum round-trip time, or:
    /// >
    /// >> `(2 * MAX_LATENCY) + PROCESSING_DELAY`
    ///
    /// Default value is 202 seconds.
    const COAP_MAX_RTT: Duration = Duration::from_millis(
        2 * Self::COAP_MAX_LATENCY.as_millis() as u64
            + Self::COAP_PROCESSING_DELAY.as_millis() as u64,
    );

    /// From RFC7252:
    ///
    /// > `EXCHANGE_LIFETIME` is the time from starting to send a Confirmable
    /// > message to the time when an acknowledgement is no longer expected,
    /// > i.e., message-layer information about the message exchange can be
    /// > purged.  `EXCHANGE_LIFETIME` includes a `MAX_TRANSMIT_SPAN`, a
    /// > `MAX_LATENCY` forward, `PROCESSING_DELAY`, and a `MAX_LATENCY` for
    /// > the way back.  Note that there is no need to consider
    /// > `MAX_TRANSMIT_WAIT` if the configuration is chosen such that the
    /// > last waiting period (`ACK_TIMEOUT` * (2 \*\* `MAX_RETRANSMIT`) or
    /// > the difference between `MAX_TRANSMIT_SPAN` and `MAX_TRANSMIT_WAIT`)
    /// > is less than `MAX_LATENCY` -- which is a likely choice, as
    /// > `MAX_LATENCY` is a worst-case value unlikely to be met in the real
    /// > world.  In this case, `EXCHANGE_LIFETIME` simplifies to:
    /// >
    /// >> `MAX_TRANSMIT_SPAN + (2 * MAX_LATENCY) + PROCESSING_DELAY`
    /// >
    /// > or 247 seconds with the default transmission parameters.
    const COAP_EXCHANGE_LIFETIME: Duration = Duration::from_millis(
        Self::COAP_MAX_TRANSMIT_SPAN.as_millis() as u64
            + 2 * Self::COAP_MAX_LATENCY.as_millis() as u64
            + Self::COAP_PROCESSING_DELAY.as_millis() as u64,
    );

    /// From RFC7252:
    ///
    /// > `NON_LIFETIME` is the time from sending a Non-confirmable message to
    /// > the time its Message ID can be safely reused.  If multiple
    /// > transmission of a NON message is not used, its value is
    /// > `MAX_LATENCY`, or 100 seconds.  However, a CoAP sender might send a
    /// > NON message multiple times, in particular for multicast
    /// > applications.  While the period of reuse is not bounded by the
    /// > specification, an expectation of reliable detection of duplication
    /// > at the receiver is on the timescales of `MAX_TRANSMIT_SPAN`.
    /// > Therefore, for this purpose, it is safer to use the value:
    /// >
    /// >> `MAX_TRANSMIT_SPAN + MAX_LATENCY`
    /// >
    /// > or 145 seconds with the default transmission parameters; however, an
    /// > implementation that just wants to use a single timeout value for
    /// > retiring Message IDs can safely use the larger value for
    /// > `EXCHANGE_LIFETIME`.
    const COAP_NON_LIFETIME: Duration = Duration::from_millis(
        Self::COAP_MAX_TRANSMIT_SPAN.as_millis() as u64 + Self::COAP_MAX_LATENCY.as_millis() as u64,
    );

    /// Calculates the delay between retransmissions. `attempt` is zero-based, so a value of
    /// 1 represents the duration to wait between the transmission of the first packet and the
    /// second packet.
    fn calc_retransmit_duration(&self, mut attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_secs(0);
        }

        if attempt > self.coap_max_retransmit() {
            attempt = self.coap_max_retransmit();
        }

        attempt -= 1;

        let ret = (self.coap_ack_timeout().as_millis() as u64) << attempt;

        const JDIV: u64 = 512u64;
        let rmod: u64 = (JDIV as f32 * (Self::COAP_ACK_RANDOM_FACTOR - 1.0)) as u64;
        let jmul = JDIV + rand::random::<u64>() % rmod;

        Duration::from_millis(ret * jmul / JDIV)
    }
}

/// Set of the standard transmission parameters as recommended by [IETF-RFC7252 Section 4.8].
///
/// [IETF-RFC7252 Section 4.8]: https://tools.ietf.org/html/rfc7252#section-4.8
#[doc(hidden)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct StandardCoapConstants;

impl TransParams for StandardCoapConstants {}

impl Default for StandardCoapConstants {
    fn default() -> Self {
        StandardCoapConstants
    }
}
