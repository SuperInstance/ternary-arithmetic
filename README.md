# ternary-arithmetic

Arithmetic coding for ternary data where symbols belong to the set **{-1, 0, +1}**. Provides frequency-adaptive compression with both static and dynamic (adaptive) encoding modes.

## Overview

Arithmetic coding is a form of entropy encoding that represents an entire message as a single fractional number in the interval [0, 1). Unlike Huffman coding which assigns integer-length codes to symbols, arithmetic coding can achieve compression rates arbitrarily close to the theoretical Shannon entropy limit.

This crate implements arithmetic coding specifically for ternary data (symbols -1, 0, +1). Ternary data appears in sentiment classification, ternary logic systems, trinary computing architectures, and quantized signal processing. The fixed alphabet size allows for efficient implementations without the overhead of generic symbol handling.

## Features

### Frequency Table
- **Construction** from raw ternary data with automatic zero-count prevention
- **Probability** and **cumulative range** calculations for encoding intervals
- **Adaptive updates** — incrementally update frequencies as symbols are processed
- **Rescaling** — prevent overflow by periodically halving counts (minimum 1)

### Arithmetic Encoder
- **Static encoding** — encode using a pre-computed frequency table
- **Adaptive encoding** — update frequencies on-the-fly for better compression on non-stationary data
- Full 32-bit precision interval arithmetic with proper carry management

### Arithmetic Decoder
- **Exact round-trip** decoding for both static and adaptive modes
- Bit-by-bit reconstruction of the original ternary sequence

### Utilities
- **Entropy calculation** — compute Shannon entropy of a ternary sequence in bits/symbol
- **Compression ratio** — compare compressed vs original data sizes

## Usage

### Static Encoding/Decoding

```rust
use ternary_arithmetic::{FrequencyTable, ArithmeticEncoder, ArithmeticDecoder};

let data = vec![-1, -1, -1, 0, 1, 1, -1, 0, 1];
let table = FrequencyTable::from_data(&data);

// Encode
let compressed = ArithmeticEncoder::encode(&data, &table);
println!("Compressed {} symbols into {} bytes", data.len(), compressed.len());

// Decode
let decoded = ArithmeticDecoder::decode(&compressed, &table, data.len());
assert_eq!(data, decoded);
```

### Adaptive Encoding/Decoding

```rust
use ternary_arithmetic::{ArithmeticEncoder, ArithmeticDecoder};

let data = vec![-1, 0, 1, 1, 1, 0, -1, -1, -1, 0];

// Adaptive: frequencies update as each symbol is processed
let (compressed, bits) = ArithmeticEncoder::encode_adaptive(&data);
let decoded = ArithmeticDecoder::decode_adaptive(&compressed, data.len());
assert_eq!(data, decoded);
```

### Frequency Table Operations

```rust
use ternary_arithmetic::FrequencyTable;

let mut table = FrequencyTable::uniform();  // Equal probability for {-1, 0, +1}

table.update(-1);  // Saw a -1
table.update(-1);  // Saw another -1
table.update(1);   // Saw a +1

println!("P(-1) = {:.3}", table.probability(-1));  // ~0.5
println!("P(0)  = {:.3}", table.probability(0));    // ~0.167
println!("P(+1) = {:.3}", table.probability(1));    // ~0.333

// Get cumulative range for encoding
let (low, high) = table.range(0);
println!("Symbol 0 occupies range [{}, {})", low, high);
```

### Entropy Calculation

```rust
use ternary_arithmetic::entropy;

let uniform_data = vec![-1, 0, 1, -1, 0, 1];
println!("Entropy: {:.3} bits/symbol", entropy(&uniform_data));
// Output: ~1.585 bits/symbol (= log₂(3))

let skewed_data: Vec<i32> = (0..100).map(|i| if i < 90 { 0 } else { 1 }).collect();
println!("Entropy: {:.3} bits/symbol", entropy(&skewed_data));
// Output: ~0.469 bits/symbol
```

## Compression Performance

| Data Distribution | Entropy (bits/symbol) | Arithmetic Coding | RLE |
|:-:|:-:|:-:|:-:|
| Uniform (⅓ each) | 1.585 | ~1.585 bits/sym | Variable |
| 90% one symbol | ~0.469 | ~0.47 bits/sym | Often better |
| Alternating | 1.585 | ~1.585 bits/sym | Expansion |
| Long runs | 0.0–0.5 | Near entropy | Often better |

Arithmetic coding excels when:
- Symbol probabilities are **skewed** (not uniform)
- You need **optimal** compression close to Shannon entropy
- Data has **no simple repetitive structure** (where RLE would be better)

## Algorithm Details

The encoder maintains an interval [low, high) in 32-bit integer arithmetic:
1. For each symbol, narrow the interval proportionally to its cumulative probability
2. When the interval falls entirely in the upper or lower half, output the corresponding bit and rescale
3. Handle near-convergence (interval straddles the midpoint) with pending bit counting

The decoder mirrors this process: reading bits to maintain a code value that tracks the encoder's interval, identifying which symbol's range contains the code, then narrowing identically to the encoder.

Adaptive mode updates the frequency table after each symbol, allowing the coder to adapt to changing statistics mid-stream. Rescaling prevents overflow while preserving relative frequencies.

## License

MIT
