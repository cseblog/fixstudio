// Library façade – exposes core logic for benchmarks and external use.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod dictionary;
pub mod model;
pub mod parser;
pub mod sample;
pub mod simd;
