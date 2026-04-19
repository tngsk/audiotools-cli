## 2024-05-18 - Optimized `calculate_zcr` and `compute_magnitude`
**Learning:**
- `compute_magnitude` inner loop creates a `Vec<Complex>` on every iteration, which scales poorly due to the overhead of allocation. Mutating a pre-allocated vector avoids re-allocation.
- `calculate_zcr` used a complex condition spanning four checks which is slow due to heavy logical branches in the iteration. Replacing it with `(y[i] >= 0.0) != (y[i - 1] >= 0.0)` dramatically speeds it up.

**Action:** Whenever iterating heavily in DSP logic such as STFT loops, ensure allocations are hoisted out of the tight loops. Always simplify logical branches when calculating boolean properties over arrays.
