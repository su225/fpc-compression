const DEFAULT_TABLE_SIZE: u64 = 32;

const BYTE_MASK: [u64; 8] = [
    0xff_00_00_00_00_00_00_00,
    0x00_ff_00_00_00_00_00_00,
    0x00_00_ff_00_00_00_00_00,
    0x00_00_00_ff_00_00_00_00,
    0x00_00_00_00_ff_00_00_00,
    0x00_00_00_00_00_ff_00_00,
    0x00_00_00_00_00_00_ff_00,
    0x00_00_00_00_00_00_00_ff,
];

#[derive(Debug, PartialEq)]
struct FPCCompressedBlock {
    num_bytes_encoded: usize,
    encoding: Vec<u8>,
    residual: Vec<u8>,
}

fn compress(table_size: u64, fp_values: &Vec<f64>) -> FPCCompressedBlock {
    if table_size == 0 || (table_size & (table_size-1)) != 0 {
        panic!("table size must be a multiple of 2 and preferably fit in L1 cache");
    }
    let mut true_value: u64;
    let mut last_value: u64 = 0;

    let mut fcm_hash: u64 = 0;
    let mut fcm: Vec<u64> = vec![0_u64; table_size as usize];

    let mut dfcm_hash: u64 = 0;
    let mut dfcm: Vec<u64> = vec![0_u64; table_size as usize];

    let mut encoding = vec![0_u8; (fp_values.len()-1)/2+1];
    let mut residual = vec![];
    for i in 0..fp_values.len() {
        true_value = fp_values[i].to_bits();

        let fcm_prediction = fcm[fcm_hash as usize];
        fcm[fcm_hash as usize] = true_value;
        fcm_hash = ((fcm_hash << 6) ^ (true_value >> 48)) & (table_size - 1);

        let dfcm_prediction = dfcm[dfcm_hash as usize].wrapping_add(last_value);
        dfcm[dfcm_hash as usize] = true_value.wrapping_sub(last_value);
        dfcm_hash = ((dfcm_hash << 2) ^ ((true_value - last_value) >> 40)) & (table_size - 1);
        last_value = true_value;

        let fcm_diff = fcm_prediction ^ true_value;
        let dfcm_diff = dfcm_prediction ^ true_value;
        let to_encode = std::cmp::min(fcm_diff, dfcm_diff);
        let mut lzb = 0;
        for x in 0..BYTE_MASK.len() {
            if (to_encode & BYTE_MASK[x]) != 0 {
                break;
            }
            lzb += 1;
        }
        let bytes: [u8; 8] = to_encode.to_be_bytes();
        if lzb == 4 {
            // If the number of leading bytes is 4, then treat it
            // as 3 and encode an additional 0 to the residual.
            residual.extend_from_slice(&bytes[3..]);
        } else {
            residual.extend_from_slice(&bytes[lzb as usize..]);
        }
        if lzb >= 4 {
            lzb -= 1;
        }
        let mask = lzb | (if fcm_diff < dfcm_diff { 1 << 3 } else { 0 });
        let shift = if i & 1 == 0 { 4 } else { 0 };
        encoding[i>>1] = encoding[i>>1] | (mask << shift);
    }
    FPCCompressedBlock {
        num_bytes_encoded: fp_values.len(),
        encoding,
        residual,
    }
}

fn decompress(table_size: u64, blk: &FPCCompressedBlock) -> Vec<f64> {
    let mut res = Vec::with_capacity(blk.num_bytes_encoded);

    let mut last_value: u64 = 0;
    let mut fcm_hash: u64 = 0;
    let mut fcm: Vec<u64> = vec![0_u64; table_size as usize];
    let mut dfcm_hash: u64 = 0;
    let mut dfcm: Vec<u64> = vec![0_u64; table_size as usize];

    let mut residual_index: usize = 0;
    let mut encoded_index: usize = 0;
    while encoded_index < blk.encoding.len() {
        let cur_encoding = blk.encoding[encoded_index];
        let (first_enc, second_enc) = (cur_encoding >> 4, cur_encoding & 0xf);

        let mut is_fcm_predicted;
        let mut fcm_prediction;
        let mut dfcm_prediction;
        let mut num_leading_zero_bytes;
        let mut decoded;

        fcm_prediction = fcm[fcm_hash as usize];
        dfcm_prediction = dfcm[dfcm_hash as usize];
        is_fcm_predicted = first_enc & 0b1000 != 0;
        num_leading_zero_bytes = first_enc & 0b0111;
        if num_leading_zero_bytes >= 4 {
            num_leading_zero_bytes += 1;
        }
        decoded = 0;
        for _ in 0..(8 - num_leading_zero_bytes) {
            if residual_index >= blk.residual.len() {
                panic!("not enough residual bytes in the encoding");
            }
            decoded = (decoded << 8) | (blk.residual[residual_index] as u64);
            residual_index += 1;
        }
        decoded = decoded ^ (if is_fcm_predicted { fcm_prediction } else { dfcm_prediction.wrapping_add(last_value) });
        res.push(f64::from_bits(decoded));
        fcm[fcm_hash as usize] = decoded;
        fcm_hash = ((fcm_hash << 6) ^ (decoded >> 48)) & (table_size - 1);

        dfcm[dfcm_hash as usize] = decoded.wrapping_sub(last_value);
        dfcm_hash = ((dfcm_hash << 2) ^ ((decoded.wrapping_sub(last_value)) >> 40)) & (table_size - 1);
        last_value = decoded;

        // Now decode the second byte
        if encoded_index == blk.encoding.len()-1 && blk.num_bytes_encoded & 1 != 0 {
            break;
        }
        // todo: remove code duplication
        fcm_prediction = fcm[fcm_hash as usize];
        dfcm_prediction = dfcm[dfcm_hash as usize];
        is_fcm_predicted = second_enc & 0b1000 != 0;
        num_leading_zero_bytes = second_enc & 0b0111;
        if num_leading_zero_bytes >= 4 {
            num_leading_zero_bytes += 1;
        }
        decoded = 0;
        for _ in 0..(8 - num_leading_zero_bytes) {
            if residual_index >= blk.residual.len() {
                panic!("not enough residual bytes in the encoding");
            }
            decoded = (decoded << 8) | (blk.residual[residual_index] as u64);
            residual_index += 1;
        }
        decoded = decoded ^ (if is_fcm_predicted { fcm_prediction } else { dfcm_prediction.wrapping_add(last_value) });
        res.push(f64::from_bits(decoded));

        fcm[fcm_hash as usize] = decoded;
        fcm_hash = ((fcm_hash << 6) ^ (decoded >> 48)) & (table_size - 1);

        dfcm[dfcm_hash as usize] = decoded.wrapping_sub(last_value);
        dfcm_hash = ((dfcm_hash << 2) ^ ((decoded.wrapping_sub(last_value)) >> 40)) & (table_size - 1);
        last_value = decoded;

        encoded_index += 1;
    }
    res
}

#[cfg(test)]
mod compress_decompress_test {
    use super::*;

    #[test]
    fn test_compress_even_number_of_zeros() {
        let vals: Vec<f64> = vec![0.0; 16];
        let compressed = compress(DEFAULT_TABLE_SIZE, &vals);
        assert_eq!(compressed, FPCCompressedBlock{
            num_bytes_encoded: vals.len(),
            encoding: vec![0b01110111; 8],
            residual: vec![],
        });
        let decompressed = decompress(DEFAULT_TABLE_SIZE, &compressed);
        assert_eq!(decompressed, vec![0.0; 16]);
    }

    #[test]
    fn test_compress_odd_number_of_zeros() {
        let vals: Vec<f64> = vec![0.0; 15];
        let compressed = compress(DEFAULT_TABLE_SIZE, &vals);
        assert_eq!(compressed, FPCCompressedBlock{
            num_bytes_encoded: vals.len(),
            encoding: vec![
                0b01110111, 0b01110111, 0b01110111, 0b01110111,
                0b01110111, 0b01110111, 0b01110111, 0b01110000,
            ],
            residual: vec![],
        });
        let decompressed = decompress(DEFAULT_TABLE_SIZE, &compressed);
        assert_eq!(decompressed, vec![0.0; 15]);
    }

    #[test]
    fn test_compress_same_positive_value() {
        let vals: Vec<f64> = vec![1.0; 16];
        let compressed = compress(DEFAULT_TABLE_SIZE, &vals);
        assert_eq!(compressed, FPCCompressedBlock{
            num_bytes_encoded: vals.len(),
            encoding: vec![
                0b00001000, 0b01110111, 0b01110111, 0b01110111,
                0b01110111, 0b01110111, 0b01110111, 0b01110111,
            ],
            residual: vec![63, 240, 0, 0, 0, 0, 0, 0, 63, 240, 0, 0, 0, 0, 0, 0],
        });
        let decompressed = decompress(DEFAULT_TABLE_SIZE, &compressed);
        assert_eq!(decompressed, vals);
    }

    #[test]
    fn test_compress_same_negative_value() {
        let vals: Vec<f64> = vec![-1.0; 16];
        let compressed = compress(DEFAULT_TABLE_SIZE, &vals);
        assert_eq!(compressed, FPCCompressedBlock{
            num_bytes_encoded: vals.len(),
            encoding: vec![
                0b00001000, 0b01110111, 0b01110111, 0b01110111,
                0b01110111, 0b01110111, 0b01110111, 0b01110111,
            ],
            residual: vec![191, 240, 0, 0, 0, 0, 0, 0, 191, 240, 0, 0, 0, 0, 0, 0],
        });
        let decompressed = decompress(DEFAULT_TABLE_SIZE, &compressed);
        assert_eq!(decompressed, vals);
    }
}