#!/bin/sh

set -ex

iverilog -o nova3201 $(ls -1 *.v)

vvp nova3201
#gtkwave cpu_core.vcd test.gtkw
