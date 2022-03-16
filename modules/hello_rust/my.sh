ARCH=riscv64
cargo build -v --release --target ../../kernel/targets/$ARCH.json
# cargo build -v --release --target ../../kernel/targets/$ARCH.json -Z build-std=core,alloc
# rustc --crate-name hello_rust src/lib.rs --target ../../kernel/targets/$ARCH.json