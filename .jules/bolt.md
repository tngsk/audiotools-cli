## 2024-05-18 - Optimized `calculate_zcr` and `compute_magnitude`
**Learning:**
- `compute_magnitude` inner loop creates a `Vec<Complex>` on every iteration, which scales poorly due to the overhead of allocation. Mutating a pre-allocated vector avoids re-allocation.
- `calculate_zcr` used a complex condition spanning four checks which is slow due to heavy logical branches in the iteration. Replacing it with `(y[i] >= 0.0) != (y[i - 1] >= 0.0)` dramatically speeds it up.

**Action:** Whenever iterating heavily in DSP logic such as STFT loops, ensure allocations are hoisted out of the tight loops. Always simplify logical branches when calculating boolean properties over arrays.

## 2024-05-19 - Cached FftPlanner for huge performance boost
**Learning:** `rustfft::FftPlanner::new()` and `plan_fft_forward()` are extremely expensive to call inside an audio processing loop (like `process_frame` in `spectrum-cli`). They allocate memory and compute plans that could be reused.
**Action:** When performing STFT or processing multiple audio frames, initialize the `FftPlanner` once and cache the resulting `Arc<dyn rustfft::Fft<f32>>` in the processor struct. Pre-allocate any buffers used within the loop and copy values instead of using `iter().map().collect()` to prevent continuous reallocation overhead.

## 2024-06-25 - Combined running averages in `calculate_spectral_features`
**Learning:**
- In `calculate_spectral_features` (used by `features-cli`), the code previously collected `centroids`, `rolloffs`, and `flatnesses` inside a frame-loop as intermediate `Vec<f32>`, and also contained multiple inner loops calculating things like sum and log sum over `magnitudes`.
- Converting these multiple distinct `Vec` accumulations into running scalar sums, and combining the multiple iterations into a single `zip` loop avoids allocation overhead and redundant iterations over elements.

**Action:** Whenever calculating multiple metrics across a vector simultaneously, iterate over the vector *once* where possible, computing all required metrics in a single pass. Don't use a `Vec` for intermediate frames if you only need the final average.
