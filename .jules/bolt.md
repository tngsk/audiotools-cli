## 2024-05-18 - Optimized `calculate_zcr` and `compute_magnitude`
**Learning:**
- `compute_magnitude` inner loop creates a `Vec<Complex>` on every iteration, which scales poorly due to the overhead of allocation. Mutating a pre-allocated vector avoids re-allocation.
- `calculate_zcr` used a complex condition spanning four checks which is slow due to heavy logical branches in the iteration. Replacing it with `(y[i] >= 0.0) != (y[i - 1] >= 0.0)` dramatically speeds it up.

**Action:** Whenever iterating heavily in DSP logic such as STFT loops, ensure allocations are hoisted out of the tight loops. Always simplify logical branches when calculating boolean properties over arrays.

## 2024-05-19 - Cached FftPlanner for huge performance boost
**Learning:** `rustfft::FftPlanner::new()` and `plan_fft_forward()` are extremely expensive to call inside an audio processing loop (like `process_frame` in `spectrum-cli`). They allocate memory and compute plans that could be reused.
**Action:** When performing STFT or processing multiple audio frames, initialize the `FftPlanner` once and cache the resulting `Arc<dyn rustfft::Fft<f32>>` in the processor struct. Pre-allocate any buffers used within the loop and copy values instead of using `iter().map().collect()` to prevent continuous reallocation overhead.

## 2024-05-20 - Avoid sqrt when computing Decibels from Complex Magnitudes
**Learning:**
- In `compute_magnitude_spectrum`, computing the square root (`norm()`) of every frequency bin's complex number is an expensive and unnecessary operation when you eventually need to convert the result to decibels (`20 * log10(x)`).
- We can instead calculate the magnitude squared (`norm_sqr()`), which avoids the `sqrt` operation completely. Since `10 * log10(x^2)` is mathematically equivalent to `20 * log10(x)`, we can substitute it into the loop along with a squared scaling factor, yielding ~25% faster execution for large spectrogram conversions.

**Action:** Whenever converting complex numbers to decibels, prefer using `norm_sqr()` combined with `10.0 * log10()` instead of `norm()` and `20.0 * log10()`. Ensure any scale factors applied to the magnitude are squared beforehand outside of the loop.

## 2026-04-25 - Hoisted channel conversion logic outside of sample loop
**Learning:** The channel configuration does not change during the file processing, so putting the `if input_channels == 1 && output_channels == 2 { ... }` logic inside the sample `loop` causes millions of branch evaluations per audio file, degrading performance.
**Action:** Hoist static conditional checks outside of tight processing loops. Determine the processing path once, and then use dedicated loops for each processing strategy to avoid per-sample branch evaluation.

## 2026-04-26 - Optimize overlapping window iterations using sliding window approach
**Learning:** Calculating an aggregated value (such as a sum or average squared value for RMS) over a window iteratively across a large array of audio samples can become a major bottleneck if computed naively, resulting in O(N*W) complexity.
**Action:** Use sliding window sums for rolling window calculations over large audio sample arrays. Maintain the running sum incrementally by adding the new incoming sample to the window edge and subtracting the outgoing sample from the opposite edge. This brings the time complexity down to O(N) which scales massively better on long audio streams.
