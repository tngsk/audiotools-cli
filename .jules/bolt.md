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

## 2024-05-25 - Optimizing Floating-Point Divisions in Iterators

**Learning:** When applying constant multipliers or dividing by max values inside a tight loop processing millions of samples (like chunk averaging or max-value normalizations during audio load), LLVM often cannot automatically replace divisions with reciprocal multiplications due to IEEE 754 precision rules.
**Action:** Always pre-calculate the inverse of divisors outside of tight loops (e.g., `let inv_max_val = 1.0 / max_val;`) and perform multiplication (`val * inv_max_val`) within the loop. This can yield significant (often 2x-3x) speedups for simple array transformation phases.

## 2024-05-26 - Hoisting Expensive Math Operations from Hot Loops

**Learning:** When processing audio in sliding windows or tight loops (like checking threshold values), mathematical operations such as `sqrt()` and `log10()` are computationally heavy. These can severely degrade performance when executed on every frame of a high-resolution signal.
**Action:** Instead of converting a running signal to dB inside the loop (`20.0 * (sum_sq / chunk_len).sqrt().log10() >= threshold_db`), pre-calculate a squared threshold value outside the loop (`threshold_linear * threshold_linear * chunk_len`). Then, directly compare the raw `sum_sq` computed during the loop against this pre-calculated threshold. This mathematical equivalence achieves the exact same logical result while completely bypassing expensive float operations during traversal.

## 2026-05-11 - Hoisting scaling factor calculations from per-sample loop in conversion

**Learning:** Inside inner loops used to copy and convert audio samples, re-evaluating the formula `(sample as f32 / max_val_f32) * gain_multiplier` for every sample introduces redundant division and multiplication operations. Due to strict float ordering rules in LLVM (IEEE 754), these operations aren't easily hoisted automatically.
**Action:** Pre-calculate scaling factor combinations out-of-loop (e.g. `let factor = gain_multiplier / i16::MAX as f32;` or `(CHANNEL_CONVERSION_FACTOR * gain_multiplier) / i16::MAX as f32`) to ensure that inner-loop work involves at most a single multiplication per sample.
## 2024-05-27 - Hoisting Divisors in Rendering Interpolation Loops

**Learning:** When calculating `normalized` power inside the plotters `draw_series` hot loops in `spectrum-cli` render code, repeated floating-point divisions like `(power - min_db) / (max_db - min_db)` on every bin pixel prevent optimal vectorization and cost significant CPU cycles per frame.
**Action:** Always precalculate reciprocal constants (like `let inv_db_range = 1.0 / (max_db - min_db);`) outside the hot loop and replace the division with multiplication `(power - min_db) * inv_db_range`. Combined with `clamp`, this dramatically speeds up massive frame interpolations without sacrificing visual correctness.
