        .text
        .org 0x00000000

reset_vector:
        j       start

        .org 0x00000100

start:
        lui     r1, 0x8000
        ori     r1, r1, 0x2200

        addi    r2, r0, the_string

loop:
        lb      r3, 0(r2)
        beq     r3, r0, end_loop
        sb      r3, 0(r1)
        addi    r2, r2, 1
        j       loop

end_loop:
        halt

        .org 0x00002000

the_string:
        .string "Hello world from the nova simulator!\n"

        .org 0x00003000

buffer:
        .bss 64        ; 64 words (256 bytes) of zeroed memory
