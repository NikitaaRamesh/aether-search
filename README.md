# Aether Search

> High-performance, SIMD-accelerated orthogonal indexing gateway for structural code search over GitHub monorepos.

Aether Search is an analytical information retrieval engine built in Rust. It transposes structural code syntax into an Orthogonal Bit-Matrix, allowing for sub-millisecond, vectorized set intersections using AVX-512 register-level `AND` operations. It processes real-time GitHub webhooks through a zero-copy TCP ingestion pipeline, bypassing heavy AST generation for raw `Bytes` slicing.

## Architecture Milestones & Implementation Log

### Phase 1: Systems Scaffolding
- [x] **Workspace Initialization:** Established `aether-core`, `aether-gateway`, and `aether-query` crates.
- [x] **Hardware Sympathy:** Enforced nightly toolchain and configured `+avx2` / `+avx512f` target features for the SIMD query engine.
- [x] **Memory Constraints:** Bound release profile to thin LTO and a single codegen unit for predictable benchmarking.
- [ ] **Zero-Copy Gateway:** Implement constant-time HMAC webhook verification and `BytesMut` HTTP payload slicing.
- [ ] **Orthogonal Bit-Matrix:** Define `#[repr(align(64))]` Aligned Signature Blocks to eliminate false sharing.
- [ ] **Scatter-Gather Execution:** Implement vectorized boolean intersections over the columnar bit-matrix.