#!/bin/bash
if [[ "$1" == "x86_64" ]]; then
    ARCH=x86_64
    TEXT_TYPE=elf64-x86-64
    BIN_ARCH=i386:x86-64
    PREFIX=
elif [[ "$1" == "aarch64" ]]; then
    ARCH=aarch64
    TEXT_TYPE=elf64-littleaarch64
    BIN_ARCH=aarch64
    PREFIX=aarch64-elf-
elif [[ "$1" == "riscv64" ]]; then
    ARCH=riscv64
    TEXT_TYPE=elf64-littleriscv
    BIN_ARCH=riscv:rv64
    PREFIX=riscv64-unknown-elf-
    PREFIX2=riscv64-linux-musl-
# NOTE: it seems stupid to have 2 toolchain prefixes.
# riscv64-unknown-elf-* cannot produce shared objects so we have to use riscv64-linux-musl-*
# yet riscv64-linux-musl-* have some problems generating module info
    python3 ./rv_hack.py ../../kernel/targets/$ARCH.json pic
else
    echo "Not supported target"
    exit 1
fi

# NOTE: "cargo install cargo-xbuild" if you meet "no such subcommand: `xbuild`" 
echo "Step 1. Fetching dependencies according to cargo."
echo "// Dummy file" > src/lib.rs
echo '#![no_std]' >> src/lib.rs
echo "extern crate rcore;" >> src/lib.rs
cargo xbuild --target=../../kernel/targets/$ARCH.json -v --release

echo "Step 2. Compile the library"
echo '#![no_std]' > src/lib.rs
echo "extern crate rcore;" >> src/lib.rs
echo "mod main;" >> src/lib.rs
rustc --edition=2018 --crate-name hello_rust src/lib.rs \
--color always --crate-type cdylib  -C debuginfo=2 \
--out-dir ./target/$ARCH/release/objs \
--target ../../kernel/targets/$ARCH.json \
-L dependency=target/$ARCH/release/deps \
-L dependency=target/release/deps \
--emit=obj \
-L all=../../kernel/target/$ARCH/release/deps \
-L all=../../kernel/target/release/deps

echo "Step 3. Packing the library into kernel module."
"$PREFIX"objcopy --input binary --output $TEXT_TYPE \
    --binary-architecture $BIN_ARCH\
    --rename-section .data=.rcore-lkm,CONTENTS,READONLY\
    lkm_info.txt target/$ARCH/release/objs/lkm_info.o
"$PREFIX"strip target/$ARCH/release/objs/lkm_info.o
if [[ "$1" == "riscv64" ]]; then
    "$PREFIX2"gcc -shared -o target/$ARCH/release/hello_rust.ko -nostdlib target/$ARCH/release/objs/*.o
    python3 ./rv_hack.py ../../kernel/targets/$ARCH.json static
else
    "$PREFIX"gcc -shared -o target/$ARCH/release/hello_rust.ko -nostdlib target/$ARCH/release/objs/*.o
fi
#cargo xbuild --target=../../kernel/targets/x86_64.json -vv
