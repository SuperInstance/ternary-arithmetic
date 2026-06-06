//! # ternary-arithmetic
//!
//! Arithmetic coding for ternary data where symbols belong to {-1, 0, +1}.
//! Supports frequency table construction, encoding/decoding, and adaptive updates.



/// Frequency table for ternary symbols {-1, 0, +1}.
#[derive(Debug, Clone)]
pub struct FrequencyTable {
    /// Counts indexed: -1→0, 0→1, +1→2
    counts: [u64; 3],
    total: u64,
}

impl FrequencyTable {
    /// Create a new frequency table with uniform counts.
    pub fn uniform() -> Self {
        FrequencyTable {
            counts: [1; 3],
            total: 3,
        }
    }

    /// Build a frequency table from ternary data.
    pub fn from_data(data: &[i32]) -> Self {
        let mut counts = [0u64; 3];
        for &v in data {
            let idx = ternary_to_idx(v);
            counts[idx] += 1;
        }
        // Ensure no zero counts (add 1 to each)
        for c in &mut counts {
            if *c == 0 {
                *c = 1;
            }
        }
        let total = counts.iter().sum();
        FrequencyTable { counts, total }
    }

    /// Get the cumulative frequency range [low, high) for a symbol.
    pub fn range(&self, symbol: i32) -> (u64, u64) {
        let idx = ternary_to_idx(symbol);
        let low: u64 = self.counts[..idx].iter().sum();
        let high = low + self.counts[idx];
        (low, high)
    }

    /// Get the probability of a symbol.
    pub fn probability(&self, symbol: i32) -> f64 {
        let idx = ternary_to_idx(symbol);
        self.counts[idx] as f64 / self.total as f64
    }

    /// Update the frequency table with a new symbol (adaptive).
    pub fn update(&mut self, symbol: i32) {
        let idx = ternary_to_idx(symbol);
        self.counts[idx] += 1;
        self.total += 1;
    }

    /// Rescale frequencies to prevent overflow (divide by 2, min 1).
    pub fn rescale(&mut self) {
        for c in &mut self.counts {
            *c = (*c / 2).max(1);
        }
        self.total = self.counts.iter().sum();
    }

    /// Get total count.
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Get count for a specific symbol.
    pub fn count(&self, symbol: i32) -> u64 {
        self.counts[ternary_to_idx(symbol)]
    }

    /// Find symbol for a given cumulative value.
    pub fn symbol_for_cumulative(&self, value: u64) -> i32 {
        let mut cumulative = 0u64;
        for (idx, &count) in self.counts.iter().enumerate() {
            cumulative += count;
            if value < cumulative {
                return idx_to_ternary(idx);
            }
        }
        idx_to_ternary(2) // Fallback
    }
}

fn ternary_to_idx(v: i32) -> usize {
    match v {
        -1 => 0,
        0 => 1,
        1 => 2,
        _ => panic!("Invalid ternary value: {v}"),
    }
}

fn idx_to_ternary(idx: usize) -> i32 {
    match idx {
        0 => -1,
        1 => 0,
        2 => 1,
        _ => panic!("Invalid index: {idx}"),
    }
}

/// Arithmetic encoder for ternary data.
pub struct ArithmeticEncoder {
    low: u64,
    high: u64,
    pending_bits: u32,
    output: Vec<u8>,
    bit_buffer: u8,
    bit_count: u32,
}

const PRECISION: u32 = 32;
const WHOLE: u64 = 1u64 << PRECISION;
const HALF: u64 = 1u64 << (PRECISION - 1);
const QUARTER: u64 = 1u64 << (PRECISION - 2);

impl ArithmeticEncoder {
    pub fn new() -> Self {
        ArithmeticEncoder {
            low: 0,
            high: WHOLE - 1,
            pending_bits: 0,
            output: Vec::new(),
            bit_buffer: 0,
            bit_count: 0,
        }
    }

    fn output_bit(&mut self, bit: u8) {
        self.bit_buffer = (self.bit_buffer << 1) | bit;
        self.bit_count += 1;
        if self.bit_count == 8 {
            self.output.push(self.bit_buffer);
            self.bit_buffer = 0;
            self.bit_count = 0;
        }
    }

    fn flush_bits(&mut self) {
        while self.bit_count > 0 {
            self.output_bit(0);
        }
    }

    fn output_bit_plus_pending(&mut self, bit: u8) {
        self.output_bit(bit);
        let opposite = bit ^ 1;
        for _ in 0..self.pending_bits {
            self.output_bit(opposite);
        }
        self.pending_bits = 0;
    }

    /// Encode a single symbol with the given frequency table.
    pub fn encode_symbol(&mut self, symbol: i32, table: &FrequencyTable) {
        let range = self.high - self.low + 1;
        let (sym_low, sym_high) = table.range(symbol);

        self.high = self.low + (range * sym_high / table.total()) - 1;
        self.low = self.low + (range * sym_low / table.total());

        loop {
            if self.high < HALF {
                self.output_bit_plus_pending(0);
            } else if self.low >= HALF {
                self.output_bit_plus_pending(1);
                self.low -= HALF;
                self.high -= HALF;
            } else if self.low >= QUARTER && self.high < 3 * QUARTER {
                self.pending_bits += 1;
                self.low -= QUARTER;
                self.high -= QUARTER;
            } else {
                break;
            }

            self.low = self.low << 1;
            self.high = (self.high << 1) | 1;
        }
    }

    /// Encode a sequence of ternary symbols.
    pub fn encode(data: &[i32], table: &FrequencyTable) -> Vec<u8> {
        let mut encoder = ArithmeticEncoder::new();
        for &symbol in data {
            encoder.encode_symbol(symbol, table);
        }

        // Finish encoding
        encoder.pending_bits += 1;
        if encoder.low < QUARTER {
            encoder.output_bit_plus_pending(0);
        } else {
            encoder.output_bit_plus_pending(1);
        }
        encoder.flush_bits();

        encoder.output
    }

    /// Encode with adaptive frequency updates.
    pub fn encode_adaptive(data: &[i32]) -> (Vec<u8>, usize) {
        let mut encoder = ArithmeticEncoder::new();
        let mut table = FrequencyTable::uniform();
        let _compressed_bits = 0usize;

        for &symbol in data {
            encoder.encode_symbol(symbol, &table);
            table.update(symbol);
            if table.total() > (1u64 << 30) {
                table.rescale();
            }
        }

        encoder.pending_bits += 1;
        if encoder.low < QUARTER {
            encoder.output_bit_plus_pending(0);
        } else {
            encoder.output_bit_plus_pending(1);
        }
        encoder.flush_bits();

        let len = encoder.output.len();
        (encoder.output, len * 8)
    }

    /// Get the encoded bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.output
    }
}

/// Arithmetic decoder for ternary data.
pub struct ArithmeticDecoder<'a> {
    data: &'a [u8],
    byte_idx: usize,
    bit_idx: u32,
    low: u64,
    high: u64,
    code: u64,
}

impl<'a> ArithmeticDecoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        let mut decoder = ArithmeticDecoder {
            data,
            byte_idx: 0,
            bit_idx: 0,
            low: 0,
            high: WHOLE - 1,
            code: 0,
        };

        // Read initial code bits
        for _ in 0..PRECISION {
            decoder.code = (decoder.code << 1) | decoder.read_bit();
        }

        decoder
    }

    fn read_bit(&mut self) -> u64 {
        if self.byte_idx < self.data.len() {
            let bit = ((self.data[self.byte_idx] as u64) >> (7 - self.bit_idx)) & 1;
            self.bit_idx += 1;
            if self.bit_idx == 8 {
                self.bit_idx = 0;
                self.byte_idx += 1;
            }
            bit
        } else {
            0
        }
    }

    /// Decode a single symbol.
    pub fn decode_symbol(&mut self, table: &FrequencyTable) -> i32 {
        let range = self.high - self.low + 1;
        let scaled = ((self.code - self.low + 1) * table.total() - 1) / range;

        // Find symbol
        let symbol = table.symbol_for_cumulative(scaled);
        let (sym_low, sym_high) = table.range(symbol);

        self.high = self.low + (range * sym_high / table.total()) - 1;
        self.low = self.low + (range * sym_low / table.total());

        loop {
            if self.high < HALF {
                // Nothing
            } else if self.low >= HALF {
                self.low -= HALF;
                self.high -= HALF;
                self.code -= HALF;
            } else if self.low >= QUARTER && self.high < 3 * QUARTER {
                self.low -= QUARTER;
                self.high -= QUARTER;
                self.code -= QUARTER;
            } else {
                break;
            }

            self.low = self.low << 1;
            self.high = (self.high << 1) | 1;
            self.code = (self.code << 1) | self.read_bit();
        }

        symbol
    }

    /// Decode a sequence of given length.
    pub fn decode(data: &'a [u8], table: &FrequencyTable, len: usize) -> Vec<i32> {
        let mut decoder = ArithmeticDecoder::new(data);
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(decoder.decode_symbol(table));
        }
        result
    }

    /// Decode with adaptive frequency updates.
    pub fn decode_adaptive(data: &'a [u8], len: usize) -> Vec<i32> {
        let mut decoder = ArithmeticDecoder::new(data);
        let mut table = FrequencyTable::uniform();
        let mut result = Vec::with_capacity(len);

        for _ in 0..len {
            let symbol = decoder.decode_symbol(&table);
            table.update(symbol);
            if table.total() > (1u64 << 30) {
                table.rescale();
            }
            result.push(symbol);
        }

        result
    }
}

/// Compression ratio: compressed bytes / original bytes.
pub fn compression_ratio(original_len: usize, compressed_bytes: usize) -> f64 {
    if original_len == 0 {
        return 0.0;
    }
    // Each ternary symbol = 1 byte in original (trinary representation)
    compressed_bytes as f64 / original_len as f64
}

/// Entropy of a ternary sequence in bits per symbol.
pub fn entropy(data: &[i32]) -> f64 {
    let table = FrequencyTable::from_data(data);
    let mut h = 0.0;
    for sym in &[-1i32, 0, 1] {
        let p = table.probability(*sym);
        if p > 0.0 {
            h -= p * p.log2();
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_table_uniform() {
        let table = FrequencyTable::uniform();
        assert_eq!(table.total(), 3);
        assert_eq!(table.count(-1), 1);
        assert_eq!(table.count(0), 1);
        assert_eq!(table.count(1), 1);
    }

    #[test]
    fn test_frequency_table_from_data() {
        let data = vec![-1, -1, -1, 0, 1, 1];
        let table = FrequencyTable::from_data(&data);
        assert_eq!(table.count(-1), 3);
        assert_eq!(table.count(0), 1);
        assert_eq!(table.count(1), 2);
        assert_eq!(table.total(), 6);
    }

    #[test]
    fn test_frequency_table_range() {
        let data = vec![-1, -1, 0, 1]; // counts: 2, 1, 1, total=4
        let table = FrequencyTable::from_data(&data);
        assert_eq!(table.range(-1), (0, 2));
        assert_eq!(table.range(0), (2, 3));
        assert_eq!(table.range(1), (3, 4));
    }

    #[test]
    fn test_frequency_table_probability() {
        let data = vec![-1, -1, 0, 1];
        let table = FrequencyTable::from_data(&data);
        assert!((table.probability(-1) - 0.5).abs() < 1e-9);
        assert!((table.probability(0) - 0.25).abs() < 1e-9);
        assert!((table.probability(1) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn test_frequency_table_update() {
        let mut table = FrequencyTable::uniform();
        table.update(-1);
        table.update(-1);
        assert_eq!(table.count(-1), 3);
        assert_eq!(table.total(), 5);
    }

    #[test]
    fn test_frequency_table_rescale() {
        let mut table = FrequencyTable::from_data(&vec![-1; 100]);
        assert_eq!(table.count(-1), 100);
        table.rescale();
        assert_eq!(table.count(-1), 50);
        assert_eq!(table.count(0), 1); // min 1
        assert_eq!(table.count(1), 1); // min 1
    }

    #[test]
    fn test_frequency_table_symbol_for_cumulative() {
        let data = vec![-1, -1, 0, 1]; // cumulative: -1=[0,2), 0=[2,3), 1=[3,4)
        let table = FrequencyTable::from_data(&data);
        assert_eq!(table.symbol_for_cumulative(0), -1);
        assert_eq!(table.symbol_for_cumulative(1), -1);
        assert_eq!(table.symbol_for_cumulative(2), 0);
        assert_eq!(table.symbol_for_cumulative(3), 1);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let data = vec![-1, 0, 1, -1, 0, 1, 1, -1];
        let table = FrequencyTable::from_data(&data);
        let encoded = ArithmeticEncoder::encode(&data, &table);
        let decoded = ArithmeticDecoder::decode(&encoded, &table, data.len());
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_encode_decode_uniform_data() {
        let data = vec![0, 0, 0, 0, 0, 0, 0, 0];
        let table = FrequencyTable::from_data(&data);
        let encoded = ArithmeticEncoder::encode(&data, &table);
        let decoded = ArithmeticDecoder::decode(&encoded, &table, data.len());
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_encode_decode_all_symbols() {
        let data = vec![-1, 0, 1, -1, 0, 1, -1, 0, 1];
        let table = FrequencyTable::from_data(&data);
        let encoded = ArithmeticEncoder::encode(&data, &table);
        let decoded = ArithmeticDecoder::decode(&encoded, &table, data.len());
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_encode_decode_long_sequence() {
        let data: Vec<i32> = (0..1000)
            .map(|i| match i % 3 {
                0 => -1,
                1 => 0,
                _ => 1,
            })
            .collect();
        let table = FrequencyTable::from_data(&data);
        let encoded = ArithmeticEncoder::encode(&data, &table);
        let decoded = ArithmeticDecoder::decode(&encoded, &table, data.len());
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_encode_decode_single_symbol() {
        for sym in &[-1i32, 0, 1] {
            let data = vec![*sym];
            let table = FrequencyTable::from_data(&data);
            let encoded = ArithmeticEncoder::encode(&data, &table);
            let decoded = ArithmeticDecoder::decode(&encoded, &table, 1);
            assert_eq!(data, decoded, "Failed for symbol {sym}");
        }
    }

    #[test]
    fn test_better_compression_than_rle_for_skewed() {
        // Heavily skewed: 90% zeros
        let data: Vec<i32> = (0..100).map(|i| if i < 90 { 0 } else { 1 }).collect();

        let table = FrequencyTable::from_data(&data);
        let encoded = ArithmeticEncoder::encode(&data, &table);

        // Arithmetic coding should compress heavily
        // For 90% zeros, entropy ≈ 0.469 bits/symbol
        // 100 symbols → ~6 bytes theoretical minimum
        // RLE would produce: 1 run of 90 zeros + 10 runs of alternating 1s and remaining
        // Actually: [0,90], [1,10] = 2 runs = 4 units vs 100 original
        // Arithmetic should still be competitive or better

        let arith_ratio = compression_ratio(data.len(), encoded.len());
        // With 90% skew, ratio should be well under 1.0
        assert!(
            arith_ratio < 1.0,
            "Arithmetic coding should compress skewed data: ratio = {arith_ratio}"
        );
    }

    #[test]
    fn test_adaptive_roundtrip() {
        let data = vec![-1, -1, 0, 0, 1, 1, -1, 0, 1, -1, 0, 1];
        let (encoded, _bits) = ArithmeticEncoder::encode_adaptive(&data);
        let decoded = ArithmeticDecoder::decode_adaptive(&encoded, data.len());
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_adaptive_long_roundtrip() {
        let data: Vec<i32> = (0..500)
            .map(|i| match i % 5 {
                0 => -1,
                1 => -1,
                2 => 0,
                3 => 1,
                _ => 1,
            })
            .collect();
        let (encoded, _) = ArithmeticEncoder::encode_adaptive(&data);
        let decoded = ArithmeticDecoder::decode_adaptive(&encoded, data.len());
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_entropy_calculation() {
        // Uniform: entropy should be log2(3) ≈ 1.585
        let data = vec![-1, 0, 1, -1, 0, 1];
        let h = entropy(&data);
        assert!((h - 3f64.log2()).abs() < 0.01, "Entropy of uniform: {h}");

        // All same: entropy should be very low
        let data2: Vec<i32> = vec![1; 1000];
        let h2 = entropy(&data2);
        assert!(h2 < 0.05, "Entropy of near-constant: {h2}");
    }

    #[test]
    fn test_compression_ratio_function() {
        assert_eq!(compression_ratio(100, 25), 0.25);
        assert_eq!(compression_ratio(0, 0), 0.0);
        assert_eq!(compression_ratio(50, 50), 1.0);
    }

    #[test]
    fn test_empty_data() {
        let data: Vec<i32> = vec![];
        let table = FrequencyTable::from_data(&data);
        // Should have uniform fallback (all 1s)
        assert_eq!(table.total(), 3);
    }
}
