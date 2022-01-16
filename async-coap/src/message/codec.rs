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

//! Low-level message codec functions.

use super::option::*;
use super::*;

/// Calculates the encoded size of a CoAP option.
pub fn calc_option_size(prev_key: OptionNumber, key: OptionNumber, mut value_len: usize) -> usize {
    if value_len >= 269 {
        value_len += 2;
    } else if value_len >= 13 {
        value_len += 1;
    }

    let option_delta = (key - prev_key) as u16;

    if option_delta >= 269 {
        value_len += 3;
    } else if option_delta >= 13 {
        value_len += 2;
    } else {
        value_len += 1;
    }

    return value_len;
}

/// Decodes one option from a `core::slice::Iter`, which can be obtained from a byte slice.
/// The iterator is then advanced to the next option.
///
/// Will return `Ok(None)` if it either encounters the end-of-options marker (0xFF) or if the
/// given iterator has been fully consumed.
pub fn decode_option<'a>(
    iter: &mut core::slice::Iter<'a, u8>,
    last_option: OptionNumber,
) -> Result<Option<(OptionNumber, &'a [u8])>, Error> {
    // TODO(#5): Improve performance.
    macro_rules! try_next {
        ($iter:expr, $none:expr) => {
            match ($iter).next() {
                Some(x) => *x,
                None => return $none,
            }
        };
    }

    let header: u8 = try_next!(iter, Ok(None));

    if header == 0xFF {
        // End of options marker.
        return Ok(None);
    }

    let key_delta: u16 = match header >> 4 {
        13 => 13u16 + try_next!(iter, Err(Error::ParseFailure)) as u16,
        14 => {
            let msb = try_next!(iter, Err(Error::ParseFailure)) as u16;
            (269u16 + try_next!(iter, Err(Error::ParseFailure)) as u16 + (msb << 8)) as u16
        }
        15 => return Err(Error::ParseFailure),
        key @ _ => key as u16,
    };

    let len = match header & 0xF {
        13 => (13 + try_next!(iter, Err(Error::ParseFailure))) as usize,
        14 => {
            let msb = try_next!(iter, Err(Error::ParseFailure)) as u16;
            (269u16 + try_next!(iter, Err(Error::ParseFailure)) as u16 + (msb << 8)) as usize
        }
        15 => return Err(Error::ParseFailure),
        len @ _ => len as usize,
    };

    if last_option > core::u16::MAX - key_delta {
        // Don't let the key wrap.
        return Err(Error::ParseFailure);
    }

    if len == 0 {
        return Ok(Some((last_option + key_delta, &[])));
    }

    let value: &'a [u8] = &iter.as_slice()[..len];

    iter.nth(len - 1);

    Ok(Some((last_option + key_delta, value)))
}

/// Encodes all parts of an option into the given buffer *except* the value. All other parts,
/// including the value length, are encoded. This is typically used directly when inserting
/// options, otherwise `encode_option()` (which writes the value) is typically a better fit.
pub fn encode_option_without_value(
    buffer: &mut [u8],
    prev_key: OptionNumber,
    key: OptionNumber,
    value_len: usize,
) -> Result<usize, Error> {
    if prev_key > key {
        return Err(Error::InvalidArgument);
    }

    let calc_len = calc_option_size(prev_key, key, value_len);
    if calc_len > buffer.len() {
        log::warn!("calc_len:{}, blen:{}", calc_len, buffer.len());
        return Err(Error::OutOfSpace);
    }

    if value_len > MAX_OPTION_VALUE_SIZE {
        log::warn!("value_len:{}, max:{}", value_len, MAX_OPTION_VALUE_SIZE);
        return Err(Error::InvalidArgument);
    }

    let mut value_offset = 1;
    let mut option_delta = key - prev_key;

    let buffer_ptr = buffer.as_mut_ptr();

    unsafe {
        // This is safe because we checked the buffer size constraints in a check above.
        // This significantly improves performance.

        if option_delta >= 269 {
            option_delta -= 269;
            *buffer_ptr.offset(0) = 14 << 4;
            *buffer_ptr.offset(1) = (option_delta >> 8) as u8;
            *buffer_ptr.offset(2) = option_delta as u8;
            value_offset += 2;
        } else if option_delta >= 13 {
            *buffer_ptr.offset(0) = 13 << 4;
            *buffer_ptr.offset(1) = (option_delta - 13) as u8;
            value_offset += 1;
        } else {
            *buffer_ptr.offset(0) = (option_delta << 4) as u8;
        }

        if value_len >= 269 {
            *buffer_ptr.offset(0) |= 14;
            *buffer_ptr.offset(value_offset) = ((value_len - 269) >> 8) as u8;
            *buffer_ptr.offset(value_offset + 1) = (value_len - 269) as u8;
            value_offset += 2;
        } else if value_len >= 13 {
            *buffer_ptr.offset(0) |= 13;
            *buffer_ptr.offset(value_offset) = (value_len - 13) as u8;
            value_offset += 1;
        } else {
            *buffer_ptr.offset(0) |= (value_len & 15) as u8;
        }
    }

    return Ok(value_offset as usize + value_len);
}

/// Encodes an option into the given buffer, including the value.
pub fn encode_option(
    buffer: &mut [u8],
    prev_key: OptionNumber,
    key: OptionNumber,
    value: &[u8],
) -> Result<usize, Error> {
    let option_len = encode_option_without_value(buffer, prev_key, key, value.len())?;

    // The value bytes are always at the end.
    buffer[option_len - value.len()..option_len].copy_from_slice(value);

    return Ok(option_len);
}

/// Helper function for implementing option insertion.
/// Return value is a tuple of several fields:
///
/// * `split_index` (`usize`) The index where the new option should be inserted.
/// * `prev_option_key` (`OptionNumber`) The option number of the option immediately before the split.
/// * `next_key` (`OptionNumber`) The option number of the option immediately after the split.
/// * `next_value_len` (`usize`) The length of the value of the option immediately after the split.
/// * `next_option_size` (`usize`) The length of the entire option immediately after the split.
///
fn insert_split_helper(
    buffer: &[u8],
    key: OptionNumber,
) -> (usize, OptionNumber, OptionNumber, usize, usize) {
    // This is the key for the option immediately prior to
    // the option we are adding.
    let mut prev_option_key = OptionNumber(0);

    // This marks at what index we will split the two halves.
    let mut split_index;

    let mut iter = OptionIterator::new(buffer);

    loop {
        split_index = iter.as_slice().as_ptr() as usize - buffer.as_ptr() as usize;

        let (next_key, next_value) = iter
            .next()
            .expect(&format!(
                "Unexpected end of options (prev: {}, iter: {:?})",
                prev_option_key, iter
            ))
            .expect("Wrote corrupt options");

        if next_key > key {
            let next_option_size =
                iter.as_slice().as_ptr() as usize - buffer.as_ptr() as usize - split_index;
            return (
                split_index,
                prev_option_key,
                next_key,
                next_value.len(),
                next_option_size,
            );
        }

        prev_option_key = next_key;
    }
}

/// Inserts an option into an option list. Very slow unless called sequentially.
pub fn insert_option(
    buffer: &mut [u8],
    mut len: usize,
    last_option: OptionNumber,
    key: OptionNumber,
    value: &[u8],
) -> Result<(usize, OptionNumber), Error> {
    if value.len() > MAX_OPTION_VALUE_SIZE {
        return Err(Error::InvalidArgument);
    }

    if key >= last_option {
        // This is the easy case: A simple append is adequate.
        len += encode_option(&mut buffer[len..], last_option, key, value)?;
        return Ok((len, key));
    }

    // What follows will only happen if this method is called with a property key
    // out-of-order. Hopefully this should only happen rarely, as there is a
    // significant performance penalty for doing so. This approach does have a
    // bright side though: It doesn't require a heap.

    let (split_index, prev_option_key, next_option_key, next_option_value_len, next_option_size) =
        insert_split_helper(&buffer[..len], key);

    // This variable is keeping track of the small possible change
    // in size due to the change of the key delta encoding.
    let key_delta_size_adj =
        next_option_size - calc_option_size(key, next_option_key, next_option_value_len);

    // The size of the option we are going to insert.
    let new_option_size = calc_option_size(prev_option_key, key, value.len());

    // Calculate the total change in size.
    let adj_size = new_option_size - key_delta_size_adj;

    // Do a space check before we start trying to move buffers around.
    if len + adj_size > buffer.len() {
        log::warn!(
            "len:{} + adj_size:{} > blen:{}",
            len,
            adj_size,
            buffer.len()
        );
        return Err(Error::OutOfSpace);
    }

    let src = split_index..len;
    let dest = split_index + adj_size;

    // Move the options above the split.
    buffer.copy_within(src, dest);
    len += adj_size;

    // Encode our new option.
    // This should not fail---if it does then something
    // has gone terribly wrong and we should panic.
    encode_option(
        &mut buffer[split_index..split_index + new_option_size],
        prev_option_key,
        key,
        value,
    )
    .expect("Internal inconsistency inserting option");

    if key != prev_option_key {
        // Partially Re-encode the next option, since the previous option
        // key value has changed. Since the value part hasn't changed and
        // remains at the end of the option, we don't need it here.
        // This should not fail---if it does then something
        // has gone terribly wrong and we should panic.
        encode_option_without_value(
            &mut buffer[split_index + new_option_size..],
            key,
            next_option_key,
            next_option_value_len,
        )
        .expect("Internal inconsistency inserting option");
    }

    return Ok((len, last_option));
}
