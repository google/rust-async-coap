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

/// Encodes an unsigned 32-bit number into the given buffer, returning
/// the resized buffer. The returned buffer may be smaller than the
/// `dst`, and may even be empty. The returned buffer is only as large
/// as it needs to be to represent the given value.
pub fn encode_u32(value: u32, dst: &mut [u8]) -> &mut [u8] {
    if value == 0 {
        &mut []
    } else if value <= 0xFF {
        dst[0] = value as u8;
        &mut dst[..1]
    } else if value <= 0xFFFF {
        dst[0] = (value >> 8) as u8;
        dst[1] = value as u8;
        &mut dst[..2]
    } else if value <= 0xFFFFFF {
        dst[0] = (value >> 16) as u8;
        dst[1] = (value >> 8) as u8;
        dst[2] = value as u8;
        &mut dst[..3]
    } else {
        dst[0] = (value >> 24) as u8;
        dst[1] = (value >> 16) as u8;
        dst[2] = (value >> 8) as u8;
        dst[3] = value as u8;
        &mut dst[..4]
    }
}

/// Attempts to decode the given little-endian-encoded integer to a `u32`.
/// Input may be up to four bytes long. If the input is larger than four
/// bytes long, returns `None`.
pub fn try_decode_u32(src: &[u8]) -> Option<u32> {
    match src.len() {
        0 => Some(0u32),
        1 => Some(src[0] as u32),
        2 => Some(((src[0] as u32) << 8) + src[1] as u32),
        3 => Some(((src[0] as u32) << 16) + ((src[1] as u32) << 8) + src[2] as u32),
        4 => Some(
            ((src[0] as u32) << 24)
                + ((src[1] as u32) << 16)
                + ((src[2] as u32) << 8)
                + src[3] as u32,
        ),
        _ => None,
    }
}

/// Attempts to decode the given little-endian-encoded integer to a `u16`.
/// Input may be up to two bytes long. If the input is larger than two
/// bytes long, returns `None`.
pub fn try_decode_u16(src: &[u8]) -> Option<u16> {
    match src.len() {
        0 => Some(0u16),
        1 => Some(src[0] as u16),
        2 => Some(((src[0] as u16) << 8) + src[1] as u16),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::util::*;

    #[test]
    fn encode_decode_u32() {
        for i in vec![
            0x00, 0x01, 0x0FF, 0x100, 0x0FFFF, 0x10000, 0x0FFFFFF, 0x1000000, 0xFFFFFFFF,
        ] {
            assert_eq!(try_decode_u32(encode_u32(i, &mut [0; 4])).unwrap(), i);
        }

        assert_eq!(try_decode_u32(&mut [0; 5]), None);
    }

    #[test]
    fn encode_decode_u16() {
        for i in 0u32..=core::u16::MAX as u32 {
            let buf = &mut [0; 4];
            let enc = encode_u32(i as u32, buf);
            assert_eq!(try_decode_u16(enc).unwrap(), i as u16, "enc:{:02x?}", enc);
        }

        assert_eq!(try_decode_u16(&mut [0; 3]), None);
    }
}
