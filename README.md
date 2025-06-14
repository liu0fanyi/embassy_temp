# 无法编译通过的版本
 * target是`riscv64.json`在`./.cargo/config.toml`里指定
 * riscv64.json是通过`rustc -Z unstable-options --print target-spec-json --target riscv64imac-unknown-none-elf | save riscv64imc.json`获得，并去掉了`feature a`
 * 在`Cargo.toml`里添加了`portable-atomic = { version = "1", features = ["unsafe-assume-single-core"] }`，来支持CAS
 * 我在`riscv32imc`下尝试过`portable-atomic = { version = "1", features = ["unsafe-assume-single-core"] }`，虽然板子上完全跑不起来，但是可以编译过，说明这个crate有在替换相应的cas的实现
 * 然后在这里获得报错
```
error: `portable_atomic_unsafe_assume_single_core` cfg (`unsafe-assume-single-core` feature) is not compatible with target that supports atomic CAS;
       see also <https://github.com/taiki-e/portable-atomic/issues/148> for troubleshooting
   --> /home/liu/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/portable-atomic-1.11.1/src/lib.rs:355:1
    |
355 | / compile_error!(
356 | |     "`portable_atomic_unsafe_assume_single_core` cfg (`unsafe-assume-single-core` feature) \
357 | |      is not compatible with target that supports atomic CAS;\n\
358 | |      see also <https://github.com/taiki-e/portable-atomic/issues/148> for troubleshooting"
359 | | );
    | |_^
```
 * 使用`cargo build -Z build-std --release`来编译
