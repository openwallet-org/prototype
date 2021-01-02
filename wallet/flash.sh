#!/bin/sh
cargo build --release
cargo objcopy --release --target=thumbv7m-none-eabi -- -O binary wallet.bin
openocd -f ./openocd.cfg -c "program target/thumbv7m-none-eabi/release/wallet verify exit"