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

#[derive(Debug)]
pub struct OptionEncoder<'a> {
    buffer: &'a mut [u8],
    len: usize,
    last_option: OptionNumber,
}

impl<'a> OptionEncoder<'a> {
    pub fn new(buffer: &'a mut [u8]) -> OptionEncoder<'a> {
        OptionEncoder {
            buffer,
            len: 0,
            last_option: Default::default(),
        }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns tuple of (option_data, unused_data).
    pub fn finish(self) -> (&'a mut [u8], &'a mut [u8]) {
        self.buffer.split_at_mut(self.len)
    }
}

impl<'a> OptionInsert for OptionEncoder<'a> {
    fn insert_option_with_bytes(&mut self, key: OptionNumber, value: &[u8]) -> Result<(), Error> {
        let (len, last_option) =
            insert_option(self.buffer, self.len, self.last_option, key, value)?;

        self.last_option = last_option;
        self.len = len;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_builder_seq() {
        let buffer = &mut [0u8; 200];

        let mut builder = OptionEncoder::new(buffer);

        assert_eq!(Ok(()), builder.insert_option_empty(OptionNumber(1)));
        assert_eq!(Ok(()), builder.insert_option_with_u32(OptionNumber(2), 20));
        assert_eq!(
            Ok(()),
            builder.insert_option_with_str(OptionNumber(3), "hello")
        );

        let (option_data, buffer_unused) = builder.finish();

        println!("buffer_unused: {:?}", buffer_unused);
        println!("option_data: {:?}", option_data);

        let mut iter = OptionIterator::new(option_data);

        assert_eq!(
            Ok(Some((OptionNumber(1), "".as_bytes()))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(2), &[20u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(3), "hello".as_bytes()))),
            iter.next().transpose()
        );
        assert_eq!(None, iter.next());
    }

    #[test]
    fn option_builder_seq_ordered() {
        let buffer = &mut [0u8; 200];

        let mut builder = OptionEncoder::new(buffer);

        assert_eq!(Ok(()), builder.insert_option_with_u32(OptionNumber(2), 1));
        assert_eq!(Ok(()), builder.insert_option_with_u32(OptionNumber(2), 2));
        assert_eq!(Ok(()), builder.insert_option_with_u32(OptionNumber(2), 3));
        assert_eq!(Ok(()), builder.insert_option_with_u32(OptionNumber(2), 4));

        let (option_data, buffer_unused) = builder.finish();

        println!("buffer_unused: {:?}", buffer_unused);
        println!("option_data: {:?}", option_data);

        let mut iter = OptionIterator::new(option_data);

        assert_eq!(
            Ok(Some((OptionNumber(2), &[1u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(2), &[2u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(2), &[3u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(2), &[4u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(None, iter.next());
    }

    #[test]
    fn option_builder_nonseq_ordered() {
        let buffer = &mut [0u8; 20];

        let mut builder = OptionEncoder::new(buffer);

        assert_eq!(Ok(()), builder.insert_option_with_u32(OptionNumber(3), 0));
        println!("builder: {:?}", builder);
        assert_eq!(Ok(()), builder.insert_option_with_u32(OptionNumber(2), 1));
        println!("builder: {:?}", builder);
        assert_eq!(Ok(()), builder.insert_option_with_u32(OptionNumber(2), 2));
        println!("builder: {:?}", builder);
        assert_eq!(Ok(()), builder.insert_option_with_u32(OptionNumber(2), 3));
        println!("builder: {:?}", builder);
        assert_eq!(Ok(()), builder.insert_option_with_u32(OptionNumber(2), 4));
        println!("builder: {:?}", builder);

        let (option_data, buffer_unused) = builder.finish();

        println!("buffer_unused: {:?}", buffer_unused);
        println!("option_data: {:?}", option_data);

        let mut iter = OptionIterator::new(option_data);

        assert_eq!(
            Ok(Some((OptionNumber(2), &[1u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(2), &[2u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(2), &[3u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(2), &[4u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(3), &[][..]))),
            iter.next().transpose()
        );
        assert_eq!(None, iter.next());
    }

    #[test]
    fn option_builder_stress_insert() {
        use rand::prelude::*;
        use rand::rngs::SmallRng;

        let buffer = &mut [0u8; 2000];
        let mut rng = SmallRng::from_seed(Default::default());
        let range = 0..100;

        let mut builder = OptionEncoder::new(buffer);

        for i in range.clone() {
            let key = OptionNumber(rng.gen());
            let bytes = &[1, 2, 3];
            println!("{}: Inserting key {} with value {:?}", i, key, bytes);

            assert_eq!(Ok(()), builder.insert_option_with_bytes(key, bytes));
        }

        let (option_data, buffer_unused) = builder.finish();

        println!("buffer_unused: {:?}", buffer_unused);
        println!("option_data: {:?}", option_data);

        let mut iter = OptionIterator::new(option_data);

        for _ in range {
            let result = iter.next();
            assert!(result.is_some());

            let result = result.unwrap();
            assert!(result.is_ok());

            let result = result.unwrap();

            assert_eq!(&[1, 2, 3], result.1);
        }
        assert_eq!(None, iter.next());
    }

    #[test]
    fn option_builder_space_overflow() {
        let buffer = &mut [0u8; 201];
        let range = 0..50;

        let mut builder = OptionEncoder::new(buffer);

        for i in range {
            let key = OptionNumber(2);
            let bytes = &[1, 2, 3];
            println!("{}: Inserting key {} with value {:?}", i, key, bytes);

            assert_eq!(Ok(()), builder.insert_option_with_bytes(key, bytes));
        }
        assert_eq!(
            Err(Error::OutOfSpace),
            builder.insert_option_with_bytes(OptionNumber(2), &[1, 2, 3])
        );
        assert_eq!(
            Err(Error::OutOfSpace),
            builder.insert_option_with_bytes(OptionNumber(1), &[1, 2, 3])
        );
        assert_eq!(Ok(()), builder.insert_option_empty(OptionNumber(1)));
    }

    #[test]
    fn option_builder_nonseq1() {
        let buffer = &mut [0u8; 200];

        let mut builder = OptionEncoder::new(buffer);

        assert_eq!(Ok(()), builder.insert_option_empty(OptionNumber(1)));
        assert_eq!(Ok(()), builder.insert_option_with_u32(OptionNumber(3), 20));
        assert_eq!(Ok(()), builder.insert_option_with_u32(OptionNumber(3), 20));
        assert_eq!(
            Ok(()),
            builder.insert_option_with_str(OptionNumber(4), "hello")
        );
        assert_eq!(
            Ok(()),
            builder.insert_option_with_str(OptionNumber(2), "ERMAGAHD")
        );

        let (option_data, buffer_unused) = builder.finish();

        println!("buffer_unused: {:?}", buffer_unused);
        println!("option_data: {:?}", option_data);

        let mut iter = OptionIterator::new(option_data);

        assert_eq!(
            Ok(Some((OptionNumber(1), "".as_bytes()))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(2), "ERMAGAHD".as_bytes()))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(3), &[20u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(3), &[20u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(4), "hello".as_bytes()))),
            iter.next().transpose()
        );
        assert_eq!(None, iter.next());
    }

    #[test]
    fn option_builder_nonseq2() {
        let buffer = &mut [0u8; 200];

        let mut builder = OptionEncoder::new(buffer);

        assert_eq!(Ok(()), builder.insert_option_empty(OptionNumber(1)));
        assert_eq!(
            Ok(()),
            builder.insert_option_with_u32(OptionNumber(300), 20)
        );
        assert_eq!(
            Ok(()),
            builder.insert_option_with_u32(OptionNumber(300), 20)
        );
        assert_eq!(
            Ok(()),
            builder.insert_option_with_str(OptionNumber(400), "hello")
        );
        assert_eq!(
            Ok(()),
            builder.insert_option_with_str(OptionNumber(200), "ERMAGAHD")
        );

        let (option_data, buffer_unused) = builder.finish();

        println!("buffer_unused: {:?}", buffer_unused);
        println!("option_data: {:?}", option_data);

        let mut iter = OptionIterator::new(option_data);

        assert_eq!(
            Ok(Some((OptionNumber(1), "".as_bytes()))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(200), "ERMAGAHD".as_bytes()))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(300), &[20u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(300), &[20u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(400), "hello".as_bytes()))),
            iter.next().transpose()
        );
        assert_eq!(None, iter.next());
    }

    #[test]
    fn option_builder_nonseq3() {
        let buffer = &mut [0u8; 200];

        let mut builder = OptionEncoder::new(buffer);

        assert_eq!(Ok(()), builder.insert_option_empty(OptionNumber(1)));
        assert_eq!(
            Ok(()),
            builder.insert_option_with_u32(OptionNumber(3000), 20)
        );
        assert_eq!(
            Ok(()),
            builder.insert_option_with_u32(OptionNumber(3000), 20)
        );
        assert_eq!(
            Ok(()),
            builder.insert_option_with_str(OptionNumber(4000), "hello")
        );
        assert_eq!(
            Ok(()),
            builder.insert_option_with_str(OptionNumber(2000), "ERMAGAHD")
        );

        let (option_data, buffer_unused) = builder.finish();

        println!("buffer_unused: {:?}", buffer_unused);
        println!("option_data: {:?}", option_data);

        let mut iter = OptionIterator::new(option_data);

        assert_eq!(
            Ok(Some((OptionNumber(1), "".as_bytes()))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(2000), "ERMAGAHD".as_bytes()))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(3000), &[20u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(3000), &[20u8][..]))),
            iter.next().transpose()
        );
        assert_eq!(
            Ok(Some((OptionNumber(4000), "hello".as_bytes()))),
            iter.next().transpose()
        );
        assert_eq!(None, iter.next());
    }

    #[test]
    fn option_builder_option_key() {
        let buffer = &mut [0u8; 200];

        let mut builder = OptionEncoder::new(buffer);
        let example_com = format!("{}", "example.com");

        assert_eq!(
            Ok(()),
            builder.insert_option(URI_HOST, example_com.as_ref())
        );
        assert_eq!(Ok(()), builder.insert_option(URI_PORT, 1234));
        assert_eq!(Ok(()), builder.insert_option(IF_NONE_MATCH, ()));

        let (option_data, buffer_unused) = builder.finish();

        println!("buffer_unused: {:?}", buffer_unused);
        println!("option_data: {:?}", option_data);

        let mut iter = OptionIterator::new(option_data);

        assert_eq!(
            Ok(Some((URI_HOST.0, "example.com".as_bytes()))),
            iter.next().transpose()
        );
    }
}
