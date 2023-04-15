# VMP_ANALYZER

### NOTE: *Work in progress, many features are missing. This is the very first release. This is a educational project. Don't use it for any malicous purposes, I am not taking any resplonsibility for your actions*


Usage:

* cargo run --release path_to_exe.exe begin_addr until_addr

example:

* cargo run --release protected.exe 0x1400118d9 0x1400118F1

### How it works

It starts execution from begin_addr, and stops when reaches the until_addr (not actually, there is a bug). While doing that, it also traces the executed instructions and filters them to give you a nice view of important parts of executed code. I'd say it's a good start for people only getting hands on vmprotect for the first time and trying to understand it's internals.

Later, I will release a good documentation which will explain how both vmprotect and this project works. Currently it's capable of running somewhat properly functions which are doing things like addition/multiplications/substractions/etc, but no branching. But still can handle (hopefully) calls to other functions from protected functions.


### Controls:

Up/Down - next/previous vm instruction

Left/Right - next/previous vm

S/s - save current vm trace to file (will be called vm_(number).asm)

Esc/Q/q - exit program



### Known bugs/limitations (WIP):
* Tested only with VMProtect 3.5 (and a bit 3.6)
* Not supporting switching VSP and VIP
* Branching (conditional jumps)
* Any sorts of anti-debug/anti-vm checks
* Heaven's gate technique
* 32 bit binaries
* No high-level optimizations and poor vm handler level optimizations as well
* Lots of features are missing
* etc