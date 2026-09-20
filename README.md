# Aether Search

> High-performance, SIMD-accelerated orthogonal indexing gateway for structural code search over GitHub monorepos.

Aether Search is an analytical information retrieval engine built in Rust. It transposes structural code syntax into an Orthogonal Bit-Matrix, allowing for sub-millisecond, vectorized set intersections using AVX-512 register-level `AND` operations. It processes real-time GitHub webhooks through a zero-copy TCP ingestion pipeline, bypassing heavy AST generation for raw `Bytes` slicing.

## Architecture Milestones & Implementation Log

### Phase 1: Systems Scaffolding & Webhook Ingestion
- [x] **Workspace Initialization:** Established `aether-core`, `aether-gateway`, and `aether-query` crates.
- [x] **Hardware Sympathy:** Enforced nightly toolchain and configured `+avx2` / `+avx512f` target features for the SIMD query engine.
- [x] **Memory Constraints:** Bound release profile to thin LTO and a single codegen unit for predictable benchmarking.
- [x] **Constant-Time Ingress Verification:** Implemented zero-allocation HMAC-SHA256 signature validation with stack-decoded hex parsing over raw byte slices (RFC 4231 verified).
- [x] **Zero-Copy HTTP Gateway:** Constructed Tokio/Axum asynchronous routing layer that consumes contiguous `Bytes` buffers without heap-allocated JSON deserialization; hardened with exhaustive 400/401/202 branch test coverage.
- [x] **Orthogonal Bit-Matrix Core:** Defined `#[repr(align(64))]` Aligned Signature Blocks to eliminate false sharing, parameterized memory geometry constants, and implemented lock-free `AtomicU64` mutation with documented relaxed memory semantics.
- [x] **Structural Tokenizer:** Implemented a zero-copy state machine (`PayloadScanner`) to slide over raw webhook bytes, extracting string slices for repository targets and commit SHAs without heap-allocated JSON deserialization.
- [ ] **Scatter-Gather Execution:** Implement vectorized boolean intersections over the columnar bit-matrix.