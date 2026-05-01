## 2024-05-01 - Optimizing Vector Reallocations

**Learning:** When performing repeated signal processing over audio frames (e.g., computing FFT magnitude over many overlapping windows), continuously growing vectors in a loop can cause significant memory reallocation overhead. Pre-allocating `Vec::with_capacity` using a known or estimated frame count improves performance.

**Action:** Whenever iterating through audio samples in chunks to collect results into a `Vec`, always use `Vec::with_capacity` to pre-allocate memory based on the total number of frames calculated from signal length and hop/window size.
