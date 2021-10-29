//! Internal decode functions.

use crate::aper::AperCodecData;
use crate::aper::AperCodecError;

// Decode "Normally Small" Length Determinent
//
// This type of "length" determinent is used to encode bitmap length in the SEQUENCE extensions,
// TODO: Support for the case when the length is greater than 64. We almost never come across this
// case in practice, so right now it just Errors, if in real life we actually see this error for
// any time it might have to be implemented to take care of that case.
pub(super) fn decode_normally_small_length_determinent(
    data: &mut AperCodecData,
) -> Result<usize, AperCodecError> {
    let is_small = data.decode_bool()?;
    if !is_small {
        Ok(data.decode_bits_as_integer(6)? as usize + 1_usize)
    } else {
        decode_unconstrained_length_determinent(data)
    }
}

pub(super) fn decode_constrained_length_detereminent(
    data: &mut AperCodecData,
    lb: usize,
    ub: usize,
) -> Result<usize, AperCodecError> {
    let range = ub - lb + 1;

    if range < 65536 {
        // Almost always for our use cases, so let's just use it.
        let length = decode_constrained_whole_number(data, lb as i128, ub as i128)?;
        eprintln!("length : {}", length);
        Ok(length as usize)
    } else {
        unimplemented!("Lengths larger than 65536 are not supported yet.")
    }
}

pub(super) fn decode_unconstrained_length_determinent(
    data: &mut AperCodecData,
) -> Result<usize, AperCodecError> {
    let _ = data.decode_align()?;
    let first = data.decode_bool()?;
    let length = if !first {
        data.decode_bits_as_integer(7)?
    } else {
        let second = data.decode_bool()?;
        if second {
            data.decode_bits_as_integer(14)?
        } else {
            let length = data.decode_bits_as_integer(6)?;
            if length > 4 || length < 1 {
                return Err(AperCodecError::new("The value should be 1 to 4"));
            } else {
                length * 16384
            }
        }
    };
    Ok(length as usize)
}

// Section 10.8 X.691
pub(super) fn decode_unconstrained_whole_number(
    data: &mut AperCodecData,
) -> Result<i128, AperCodecError> {
    let length = decode_unconstrained_length_determinent(data)?;
    eprintln!("unconstrained length: {}", length);
    let bits = length * 8;
    data.decode_bits_as_integer(bits)
}

// Section 10.7 X.691
pub(super) fn decode_semi_constrained_whole_number(
    data: &mut AperCodecData,
    lb: i128,
) -> Result<i128, AperCodecError> {
    let length = decode_unconstrained_length_determinent(data)?;
    eprintln!("unconstrained length: {}", length);
    let bits = length * 8;
    let val = data.decode_bits_as_integer(bits)?;
    Ok(val + lb)
}

// Decode a 'constrained' whole number where both `lb` and `ub` are available.
//
// From Section 10.5
pub(super) fn decode_constrained_whole_number(
    data: &mut AperCodecData,
    lb: i128,
    ub: i128,
) -> Result<i128, AperCodecError> {
    let range = ub - lb + 1;
    if range <= 0 {
        Err(AperCodecError::new(
            "Range for the Integer Constraint is negative.",
        ))
    } else {
        eprintln!("range: {}", range);
        let value = if range < 256 {
            let bits = match range as u8 {
                0..=1 => 0,
                2 => 1,
                3..=4 => 2,
                5..=8 => 3,
                9..=16 => 4,
                17..=32 => 5,
                33..=64 => 6,
                65..=128 => 7,
                129..=255 => 8,
            };
            data.decode_bits_as_integer(bits)?
        } else if range == 256 {
            let _ = data.decode_align()?;
            data.decode_bits_as_integer(8)?
        } else if range < 65536 {
            let _ = data.decode_align()?;
            data.decode_bits_as_integer(16)?
        } else {
            let bytes_needed = bytes_needed_for_range(range);
            eprintln!("bytes_needed : {}", bytes_needed);
            let length = decode_constrained_length_detereminent(data, 1, bytes_needed as usize)?;
            let bits = (length + 1) * 8;
            let _ = data.decode_align()?;
            data.decode_bits_as_integer(bits)?
        };
        Ok(value + lb)
    }
}

fn bytes_needed_for_range(range: i128) -> u8 {
    let bits_needed: u8 = 128 - range.leading_zeros() as u8;
    let mut bytes_needed = bits_needed / 8;
    if bits_needed % 8 != 0 {
        bytes_needed += 1
    }
    bytes_needed
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_decode_constrained_whole_number_range_0() {
        let data = &[0x70u8, 0, 0, 0];
        let mut codec_data = AperCodecData::from_slice(data);
        codec_data.advance_maybe_err(1, false).unwrap();
        let value = decode_constrained_whole_number(&mut codec_data, 14, 14);
        assert!(value.is_ok());
        let value = value.unwrap();
        assert_eq!(value, 14i128);
    }

    #[test]
    fn test_decode_constrained_whole_number_lt_256() {
        let data = &[0x70u8, 0, 0, 0];
        let mut codec_data = AperCodecData::from_slice(data);
        codec_data.advance_maybe_err(1, false).unwrap();
        let value = decode_constrained_whole_number(&mut codec_data, 7, 14);
        assert!(value.is_ok());
        let value = value.unwrap();
        assert_eq!(value, 14i128);
    }

    #[test]
    fn test_decode_constrained_whole_number_eq_256() {
        let data = &[0x80u8, 0x70u8, 0, 0];
        let mut codec_data = AperCodecData::from_slice(data);
        codec_data.advance_maybe_err(1, false).unwrap();
        let value = decode_constrained_whole_number(&mut codec_data, 0, 255);
        assert!(value.is_ok(), "{:#?}", value.err());
        let value = value.unwrap();
        assert_eq!(value, 0x70i128);
    }

    #[test]
    fn test_decode_constrained_whole_number_lt_64k() {
        let data = &[0x00u8, 0x70u8, 0x00, 1];
        let mut codec_data = AperCodecData::from_slice(data);
        codec_data.advance_maybe_err(12, false).unwrap();
        let value = decode_constrained_whole_number(&mut codec_data, 0, 64000);
        assert!(value.is_ok(), "{:#?}", value.err());
        let value = value.unwrap();
        assert_eq!(value, 1_i128);
    }

    #[test]
    fn test_decode_constrained_whole_number_gt_64k() {
        let data = &[0x00u8, 0x78u8, 0x01, 1, 0x01, 0x02];
        let mut codec_data = AperCodecData::from_slice(data);
        codec_data.advance_maybe_err(12, false).unwrap();
        let value = decode_constrained_whole_number(&mut codec_data, 0, 20_000_000);
        assert!(value.is_ok(), "{:#?}", value.err());
        let value = value.unwrap();
        assert_eq!(value, 16843010_i128);
    }
}