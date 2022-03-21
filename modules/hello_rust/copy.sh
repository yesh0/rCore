# copy hello_rust.ko into filesystem rootfs directory
ARCH=$1
cp target/$ARCH/release/hello_rust.ko ../../user/build/$ARCH/
