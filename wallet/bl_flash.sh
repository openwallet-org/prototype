#!/bin/sh
cargo build --release
cargo objcopy --release --target=thumbv7m-none-eabi -- -O binary wallet.bin
BOOT_ADDR=0x08000000
PROG_ADDR=0x08000400
openocd -f ./openocd.cfg -c "program wallet.bin $PROG_ADDR verify exit"
openocd -f ./openocd.cfg -c "program boot.bin $BOOT_ADDR verify reset exit"