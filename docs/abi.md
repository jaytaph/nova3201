# Nova CPU ABI v1.0

## 1. Register Conventions

### 1.1 Register Assignments

| Register | Name   | Purpose                          | Preserved? |
|----------|--------|----------------------------------|------------|
| r0       | zero   | Always zero (hardwired)          | N/A        |
| r1       | at     | Assembler temporary              | No         |
| r2-r3    | v0-v1  | Function return values           | No         |
| r4-r7    | a0-a3  | Function arguments               | No         |
| r8-r15   | t0-t7  | Temporary (caller-saved)         | No         |
| r16-r23  | s0-s7  | Saved registers (callee-saved)   | Yes        |
| r24-r25  | t8-t9  | More temporaries                 | No         |
| r26-r27  | k0-k1  | Kernel reserved (interrupts)     | N/A        |
| r28      | gp     | Global pointer                   | Yes        |
| r29      | sp     | Stack pointer                    | Yes        |
| r30      | fp     | Frame pointer                    | Yes        |
| r31      | ra     | Return address                   | Special    |

### 1.2 Register Usage Rules

**Caller-saved (volatile):**
- r1 (at), r2-r3 (v0-v1), r4-r7 (a0-a3), r8-r15 (t0-t7), r24-r25 (t8-t9)
- The calling function must save these if needed after a function call

**Callee-saved (non-volatile):**
- r16-r23 (s0-s7), r28 (gp), r29 (sp), r30 (fp)
- Called functions must preserve these values

## 2. Calling Convention

### 2.1 Function Arguments

- **First 4 arguments:** Passed in r4-r7 (a0-a3)
- **Additional arguments:** Passed on stack (right-to-left push)
- **Return value:** r2 (v0), or r2-r3 (v0-v1) for 64-bit returns

### 2.2 Function Prologue/Epilogue

**Standard prologue:**
```assembly
function_name:
    ADDI sp, sp, -frame_size    # Allocate stack frame
    SW   ra, frame_size-4(sp)   # Save return address
    SW   fp, frame_size-8(sp)   # Save frame pointer
    ADDI fp, sp, frame_size     # Set new frame pointer
    # Save callee-saved registers if used
```

**Standard epilogue:**
```assembly
    # Restore callee-saved registers if used
    LW   fp, frame_size-8(sp)   # Restore frame pointer
    LW   ra, frame_size-4(sp)   # Restore return address
    ADDI sp, sp, frame_size     # Deallocate stack frame
    JR   ra                     # Return
```

### 2.3 Stack Frame Layout
```
High addresses
+------------------+
| Arguments 5+     | (if any)
+------------------+
| Return address   | fp-4
+------------------+
| Saved fp         | fp-8
+------------------+
| Saved s0-s7      | (if used)
+------------------+
| Local variables  |
+------------------+
| Spill space      |
+------------------+ <- sp (current)
Low addresses
```

## 3. Stack Conventions

### 3.1 Stack Properties

- **Growth direction:** Downward (decreasing addresses)
- **Alignment:** 8-byte aligned at function boundaries
- **Initial SP:** 0x7FFF_FFFC (grows down from high memory)

### 3.2 Stack Frame Requirements

- Minimum frame size: 16 bytes (even for leaf functions that save ra/fp)
- Frame size must be multiple of 8 bytes
- SP must always be 8-byte aligned

## 4. Data Types and Alignment

| Type           | Size    | Alignment |
|----------------|---------|-----------|
| char           | 1 byte  | 1 byte    |
| short          | 2 bytes | 2 bytes   |
| int            | 4 bytes | 4 bytes   |
| long           | 4 bytes | 4 bytes   |
| long long      | 8 bytes | 8 bytes   |
| pointer        | 4 bytes | 4 bytes   |
| float          | 4 bytes | 4 bytes   |
| double         | 8 bytes | 8 bytes   |

## 5. Memory Map
```
0x0000_0000 - 0x0000_0003 : Reset Vector
0x0000_0004 - 0x0000_0007 : Trap/Exception Vector
0x0000_0008 - 0x0FFF_FFFF : Program ROM / Flash
0x1000_0000 - 0x1FFF_FFFF : Data RAM
0x2000_0000 - 0x2000_00FF : UART
0x2000_0100 - 0x2000_01FF : Timer0
0x2000_0200 - 0x2000_02FF : Timer1
0x2000_0300 - 0x2000_03FF : GPIO
0x2000_0400 - 0x2FFF_FFFF : Reserved MMIO
0x3000_0000 - 0xFFFF_FFFF : Unmapped
```

## 6. Function Call Example
```assembly
# Calling: result = add(5, 3)

    ADDI a0, zero, 5      # First argument = 5
    ADDI a1, zero, 3      # Second argument = 3
    JAL  add              # Call function
    # Result now in v0
    ADD  s0, v0, zero     # Save result to s0

add:
    ADDI sp, sp, -16      # Allocate frame
    SW   ra, 12(sp)       # Save return address
    ADD  v0, a0, a1       # result = a0 + a1
    LW   ra, 12(sp)       # Restore return address
    ADDI sp, sp, 16       # Deallocate frame
    JR   ra               # Return
```

## 7. System Call Convention

### 7.1 System Call Interface

- **Syscall number:** r2 (v0)
- **Arguments:** r4-r7 (a0-a3), additional on stack
- **Return value:** r2 (v0)
- **Instruction:** `SYSCALL` (trap to kernel)

### 7.2 Standard System Calls

| Number | Name       | Arguments          | Returns    |
|--------|------------|--------------------|------------|
| 1      | exit       | a0 = exit code     | -          |
| 2      | print_int  | a0 = integer       | -          |
| 3      | print_str  | a0 = string addr   | -          |
| 4      | read_int   | -                  | v0 = int   |

## 8. Linker Conventions

### 8.1 Symbol Naming

- Global functions: `function_name`
- Static functions: `file.function_name`
- Global variables: `variable_name`
- Static variables: `file.variable_name`

### 8.2 Section Layout
```
.text    : Code section (read-only, executable)
.rodata  : Read-only data (constants, strings)
.data    : Initialized data
.bss     : Uninitialized data (zero-initialized)
```

## 9. Special Considerations

### 9.1 Position-Independent Code (PIC)

- Use gp (r28) as base for data access within +/- 32KB
- Load gp in function prologue if needed

### 9.2 Tail Call Optimization

- Allowed when:
    - No callee-saved registers need restoration
    - Stack frame can be deallocated
    - Use `J target` instead of `JAL target; JR ra`

### 9.3 Leaf Functions

- Functions that don't call others can skip saving ra
- Still must maintain stack alignment if using stack

## 10. ABI Compliance Checklist

- [ ] Stack 8-byte aligned at function boundaries
- [ ] Callee-saved registers (s0-s7, fp, sp, gp) preserved
- [ ] Arguments passed in a0-a3, then stack
- [ ] Return values in v0-v1
- [ ] Frame pointer maintained if using dynamic stack
- [ ] Return address saved/restored if function calls others

---

**Version:** 1.0
**Date:** 2025-11-19  
**Status:** Draft