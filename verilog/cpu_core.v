// cpu_core.v
// Minimal single-cycle Nova-like core:
// - PC
// - regfile
// - ALU
// - instruction ROM
// No RAM, no loads/stores yet.

module cpu_core (
    input  wire        clk,         // We have a clock input
    input  wire        reset,       // We have a reset input

    output reg  [31:0] pc,         // expose PC for debug
    output wire [31:0] reg1_value  // expose r1 for debug/LED
);

    // === Instruction ROM ===

    wire [31:0] instr;

    // word address = pc[11:2] (assuming 4-byte instructions)
    instr_rom rom (
        .addr(pc[11:2]),
        .data(instr)
    );

    // === Decode ===

    wire [5:0]  opcode = instr[31:26];      // 6 bits opcode
    wire [4:0]  rd     = instr[25:21];      // 5 bit destination register
    wire [4:0]  rs     = instr[20:16];      // 5 bit source register
    wire [4:0]  rt     = instr[15:11];      // 5 bit target register
    wire [15:0] imm16  = instr[15:0];       // This is an overlap with the bits in RT, we can't use RT AND imm16 at the same time
    wire [25:0] target = instr[25:0];       // 26bit target for J-type

    // sign-extend imm16
    wire [31:0] imm_sext = {{16{imm16[15]}}, imm16};        // extend 11 bit immediate to 32 bits

    // Opcodes
    localparam OP_ADD  = 6'h00;
    localparam OP_SUB  = 6'h01;
    localparam OP_AND  = 6'h02;
    localparam OP_OR   = 6'h03;
    localparam OP_XOR  = 6'h04;
    localparam OP_ADDI = 6'h10;
    localparam OP_J    = 6'h28;

    // === Register file ===

    wire [31:0] rs_val;
    wire [31:0] rt_val;

    reg         rf_we;
    reg  [4:0]  rf_waddr;
    reg  [31:0] rf_wdata;

    regfile rf (
        .clk    (clk),
        .reset  (reset),
        .we     (rf_we),        // Write enable
        .waddr  (rf_waddr),
        .wdata  (rf_wdata),
        .raddr1 (rs),
        .rdata1 (rs_val),
        .raddr2 (rt),
        .rdata2 (rt_val),
        .debug_r1 (reg1_value)
    );

    // === ALU ===

    reg  [5:0]  alu_op;
    wire [31:0] alu_rd;

    // This connects the ALU
    alu the_alu (
        .alu_op  (alu_op),
        .rs      (rs_val),
        .rt      (rt_val),
        .imm16   (imm16),
        .target  (target),
        .rd      (alu_rd)
    );

    // === Next PC logic & control ===

    reg [31:0] next_pc;

    always @* begin
        // Default values
        rf_we    = 1'b0;
        rf_waddr = 5'd0;
        rf_wdata = 32'h0000_0000;
        alu_op   = 6'h3E;
        next_pc  = pc + 32'd4;

        case (opcode)
            OP_ADD: begin
                alu_op   = 6'd0;        // ALU_ADD
                rf_we    = 1'b1;        // write enabled
                rf_waddr = rd;          // rd is the destination
                rf_wdata = alu_rd;      // Data is alu_rd result
            end

            OP_ADDI: begin
                alu_op   = 6'd5;        // ALU_ADDI
                rf_we    = 1'b1;        // write enabled
                rf_waddr = rd;          // rd is the destination
                rf_wdata = alu_rd;      // data is alu_rd
            end

            OP_J: begin
                // absolute jump using imm16 << 2 (toy)
                // real design would use full 26 bits etc.
                next_pc = {pc[31:16], imm16, 2'b00};
            end

            default: begin
                // unimplemented opcodes => NOP
            end
        endcase
    end

    // PC register
    always @(posedge clk) begin
        if (reset) begin
            pc <= 32'h0000_0000;
        end else begin
            pc <= next_pc;
        end
    end

endmodule
