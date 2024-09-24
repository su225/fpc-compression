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

        // Compute FCM prediction
        let fcm_prediction = fcm[fcm_hash as usize];
        fcm[fcm_hash as usize] = true_value;
        fcm_hash = ((fcm_hash << 6) ^ (true_value >> 48)) & (table_size - 1);

        // Compute DFCM prediction
        let dfcm_prediction = dfcm[dfcm_hash as usize].wrapping_add(last_value);
        dfcm[dfcm_hash as usize] = true_value - last_value;
        dfcm_hash = ((dfcm_hash << 2) ^ ((true_value - last_value) >> 40)) & (table_size - 1);
        last_value = true_value;

        // Compute FCM and DFCM prediction diff to see which one is close
        let fcm_diff = fcm_prediction ^ true_value;
        let dfcm_diff = dfcm_prediction ^ true_value;

        // Encode the minimum among the two because the probability of having
        // more leading zeros is higher when the prediction is closer.
        let to_encode = std::cmp::min(fcm_diff, dfcm_diff);
        let mut lzb = 0;
        for x in 0..BYTE_MASK.len() {
            if (to_encode & BYTE_MASK[x]) != 0 {
                break;
            }
            lzb += 1;
        }
        let bytes: [u8; 8] = true_value.to_be_bytes();
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
        let mask = lzb | (if fcm_diff < dfcm_diff { 1 << 4 } else { 0 });
        let shift = if i & 1 == 0 { 4 } else { 0 };
        encoding[i>>1] = encoding[i>>1] | (mask << shift);
    }
    FPCCompressedBlock {
        num_bytes_encoded: fp_values.len(),
        encoding,
        residual,
    }
}

#[cfg(test)]
mod compress_test {
    use super::*;

    #[test]
    fn test_compress_even_number_of_zeros() {
        let vals: Vec<f64> = vec![0.0; 16];
        let compressed = compress(DEFAULT_TABLE_SIZE, &vals);
        assert_eq!(compressed, FPCCompressedBlock{
            num_bytes_encoded: vals.len(),
            encoding: vec![0b01110111; 8],
            residual: vec![],
        })
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
        })
    }

    #[test]
    fn test_compress_same_positive_value() {
        let vals: Vec<f64> = vec![1.0; 16];
        let compressed = compress(DEFAULT_TABLE_SIZE, &vals);
        assert_eq!(compressed, FPCCompressedBlock{
            num_bytes_encoded: vals.len(),
            encoding: vec![
                0b00010000, 0b01110111, 0b01110111, 0b01110111,
                0b01110111, 0b01110111, 0b01110111, 0b01110111,
            ],
            residual: vec![63, 240, 0, 0, 0, 0, 0, 0, 63, 240, 0, 0, 0, 0, 0, 0],
        });
    }

    #[test]
    fn test_compress_same_negative_value() {
        let vals: Vec<f64> = vec![-1.0; 16];
        let compressed = compress(DEFAULT_TABLE_SIZE, &vals);
        assert_eq!(compressed, FPCCompressedBlock{
            num_bytes_encoded: vals.len(),
            encoding: vec![
                0b00010000, 0b01110111, 0b01110111, 0b01110111,
                0b01110111, 0b01110111, 0b01110111, 0b01110111,
            ],
            residual: vec![191, 240, 0, 0, 0, 0, 0, 0, 191, 240, 0, 0, 0, 0, 0, 0],
        });
    }
}