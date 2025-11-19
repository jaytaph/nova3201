module alu (
    input  wire [5:0]   alu_op,     // selects operation
    input  wire [31:0]  rs,         // Source register
    input  wire [31:0]  rt,         // Target register
    input  wire [15:0]  imm16,      // Immediate16
    input  wire [25:0]  target,     // Target address (for jumps)
    output reg  [31:0]  rd          // Destination register
);

    localparam ALU_ADD  = 6'h0;
    localparam ALU_SUB  = 6'h1;
    localparam ALU_AND  = 6'h2;
    localparam ALU_OR   = 6'h3;
    localparam ALU_XOR  = 6'h4;
    localparam ALU_ADDI = 6'h10;

    always @* begin
        case (alu_op)
            // Register-register operations
            ALU_ADD: rd = rs + rt;
            ALU_SUB: rd = rs - rt;
            ALU_AND: rd = rs & rt;
            ALU_OR:  rd = rs | rt;
            ALU_XOR: rd = rs ^ rt;

            // Immediate operations
            ALU_ADDI: rd = rs + {{16{imm16[15]}}, imm16};

            default: rd= 32'hDEAD_BEEF;
        endcase
    end

endmodule
