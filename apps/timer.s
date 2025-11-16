# ----------------------------------------------------------------------

    .equ TIMER1_CTRL,   0x80002100
    .equ TIMER1_PERIOD, 0x80002104
    .equ TIMER1_COUNT,  0x80002108

    .equ TIMER2_CTRL,   0x80002120
    .equ TIMER2_PERIOD, 0x80002124
    .equ TIMER2_COUNT,  0x80002128

    .equ UART_TX,       0x80002200
    .equ UART_STATUS,   0x80002204

    .equ TIMER_ENABLED,   0x1       ; ENABLED
    .equ TIMER_IRQ_EN,    0x2       ; IRQ_ENABLED (not actually used here)
    .equ TIMER_ONE_SHOT,  0x4       ; ONE_SHOT

    .equ UART_TX_READY,   0x1       ; status bit 0: TX ready

# ----------------------------------------------------------------------

    .data
    .org 0x200
msg_start:
    .ascii "Timer test start...\n\0"

msg_done:
    .ascii "\nTimer2 one-shot done.\n\0"

# ----------------------------------------------------------------------

    .text
    .org 0x0

_start:
    ########################################################
    # Print startup message
    ########################################################
    li   r1, msg_start       # r1 = pointer to string
    jal  uart_print
    nop                      # delay slot (if you have one)

    ########################################################
    # Setup TIMER1 as periodic
    #   period = 1000000
    #   ctrl   = ENABLED (no IRQ, just polling)
    ########################################################
    li   r2, TIMER1_PERIOD   # r2 = &TIMER1_PERIOD
    li   r3, 1000000
    sw   r3, 0(r2)

    li   r2, TIMER1_CTRL     # r2 = &TIMER1_CTRL
    li   r3, TIMER_ENABLED   # periodic, IRQ off
    sw   r3, 0(r2)

    ########################################################
    # Setup TIMER2 as one-shot
    #   period = 5000000
    #   ctrl   = ENABLED | ONE_SHOT
    ########################################################
    li   r2, TIMER2_PERIOD   # r2 = &TIMER2_PERIOD
    li   r3, 5000000
    sw   r3, 0(r2)

    li   r2, TIMER2_CTRL     # r2 = &TIMER2_CTRL
    li   r3, TIMER_ENABLED
    ori  r3, r3, TIMER_ONE_SHOT   # ENABLED | ONE_SHOT
    sw   r3, 0(r2)

    ########################################################
    # Initialize previous counts for wrap detection
    ########################################################
    li   r2, TIMER1_COUNT    # r2 = &TIMER1_COUNT
    lw   r4, 0(r2)           # r4 = prev1

    li   r3, TIMER2_COUNT    # r3 = &TIMER2_COUNT
    lw   r5, 0(r3)           # r5 = prev2

    li   r6, 0               # r6 = flag: timer2_done (0 = not done, 1 = done)

main_loop:
    ########################################################
    # Read current counts
    ########################################################
    # TIMER1
    li   r2, TIMER1_COUNT
    lw   r7, 0(r2)           # r7 = curr1

    # TIMER2
    li   r3, TIMER2_COUNT
    lw   r8, 0(r3)           # r8 = curr2

    ########################################################
    # Detect TIMER1 wrap-around:
    #   if curr1 < prev1  => wrap => print '1'
    ########################################################
    # if curr1 >= prev1 => skip
    bge  r7, r4, no_wrap1
    nop

    # wrap happened
    li   r9, '1'
    jal  uart_putc
    nop

no_wrap1:
    # prev1 = curr1
    move r4, r7

    ########################################################
    # Detect TIMER2 one-shot "done":
    # Strategy: while timer is running, count will increase.
    # When it stops, count will stop changing. We treat:
    #   if (curr2 == prev2 && timer2_done == 0) => one-shot done
    ########################################################
    beq  r8, r5, maybe_done2
    nop

    # still changing, so not done yet
    move r5, r8              # prev2 = curr2
    j    after_timer2_check
    nop

maybe_done2:
    # only if we haven't reported it yet
    bne  r6, r0, after_timer2_check   # if timer2_done != 0, skip
    nop

    # mark done
    li   r6, 1

    # print '2' and a message
    li   r9, '2'
    jal  uart_putc
    nop

    li   r1, msg_done
    jal  uart_print
    nop

after_timer2_check:

    ########################################################
    # Loop forever
    ########################################################
    j    main_loop
    nop


############################################################
# UART helpers
############################################################

# void uart_putc(char c in r9)
#   waits until UART_STATUS bit0 == 1
#   then writes c to UART_TX
uart_putc:
    # wait_loop:
uart_wait:
    li   r11, UART_STATUS
    lw   r12, 0(r11)         # r12 = status

    andi r12, r12, UART_TX_READY
    beq  r12, r0, uart_wait  # if not ready, loop
    nop

    # write character
    li   r11, UART_TX
    sw   r9, 0(r11)

    jr   r31                 # return
    nop


# void uart_print(const char* s in r1)
#   prints 0-terminated string
uart_print:
    # load next char
uart_print_loop:
    lb   r9, 0(r1)           # r9 = *s
    beq  r9, r0, uart_print_end   # if 0, end
    nop

    jal  uart_putc
    nop

    addi r1, r1, 1           # s++
    j    uart_print_loop
    nop

uart_print_end:
    jr   r31
    nop