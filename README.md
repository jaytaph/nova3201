# A CPU design based on risc-v. 

## nova3201 stands for Nova CPU 32bit v01

- No ABI yet
- Simple applications seem to work

## Features
- r0-r31 general purpose registers
- r0 always zero
- pc program counter
- simple mmio with uart and two timers


## How to run:

```
$ cargo run --bin nvasm -- apps/app_01.s
$ cargo run --bin nova3201_cli -- apps/app_01.nvb
```

This should output:

```
Loading program 'apps/app_01.nvb'
Loading section at 0x00000000, size 9 words
HI
CPU halted.
```
