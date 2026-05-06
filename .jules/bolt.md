## 2024-05-01 - Optimizing Vector Reallocations

**Learning:** When performing repeated signal processing over audio frames (e.g., computing FFT magnitude over many overlapping windows), continuously growing vectors in a loop can cause significant memory reallocation overhead. Pre-allocating `Vec::with_capacity` using a known or estimated frame count improves performance.

**Action:** Whenever iterating through audio samples in chunks to collect results into a `Vec`, always use `Vec::with_capacity` to pre-allocate memory based on the total number of frames calculated from signal length and hop/window size.

## 2024-05-15 - Vector Preallocation in Audio Segmenter and Features Node

**Learning:** When generating audio segments or computing features, iterating without preallocating the resulting vector can lead to continuous reallocation overhead. Although simple arrays or iterator methods like `map(...).sum()` might be used, using a raw `fold` operation over large vectors can also be slightly more performant when computing sums like RMS. Additionally, explicitly pre-allocating memory using `Vec::with_capacity` via `expected_frames` based on `(y.len().saturating_sub(frame_len)) / hop_len + 1` drastically cuts memory fragmentation.

**Action:** Whenever iterating through audio samples in chunks to collect results into a `Vec`, always use `Vec::with_capacity` to pre-allocate memory based on the calculated count.

## 2024-05-20 - Iterator .sum() vs .fold() for Performance

**Learning:** Replacing idiomatic `map(...).sum()` operations with `fold(0.0, ...)` over iterators provides absolutely zero performance benefit in Rust. The standard library internalizes `sum()` for floating-point values as `fold()` under the hood, and the compiler desugars both to the exact same LLVM IR.

**Action:** Do not micro-optimize `sum()` to `fold()`. It creates unreadable code and provides false performance gains. Stick to algorithmic optimizations or concrete structural improvements like vector pre-allocation (`Vec::with_capacity`).
**Action:** Whenever iterating through audio samples in chunks to collect results into a `Vec`, always use `Vec::with_capacity` to pre-allocate memory based on the calculated count. Use `fold` instead of `map(...).sum()` when summing up squares to ensure optimum vectorization during iterations.

## 2024-05-16 - Pre-calculating scaling factors and logical branching optimizations

**Learning:** When applying constant multipliers or scalers (e.g., normalization multipliers, bit-depth max value division) to an array of audio samples, performing divisions and multiplications like `(sample / max_val) * multiplier * max_val` inside the inner loop is extremely costly. Furthermore, when computing zero crossing rates, heavy logical branches like `(prev < 0 && curr >= 0) || (prev >= 0 && curr < 0)` generate unnecessary instructions that can simply be replaced with `(curr >= 0.0) != (prev >= 0.0)`, significantly improving performance.

**Action:** Whenever iterating over audio samples to convert volume, levels or bits, always pre-calculate the combined scaling factor outside the loop and apply a single multiplication per sample. Use simplified bitwise or inequality operations for boolean conditions during iterations to eliminate branches. Avoid recalculating bounds on every loop iteration, for instance by using `i.saturating_sub(...)` instead of conditional expressions.
