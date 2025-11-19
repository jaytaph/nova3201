// instr_rom.v
// Simple instruction ROM, word addressed

module instr_rom (
    input  wire [9:0]   addr,   // word address (not byte)
    output reg  [31:0]  data
);

    // Tiny program:
    // 0: ADDI r1, r1, 1
    // 1: ADDI r2, r2, 2
    // 2: J    0
    //
    // So r1,r2 increments forever.

    // helper function to build I-type (ADDI-style) instructions
    function [31:0] mk_addi;
        input [4:0] rd;
        input [4:0] rs;
        input [15:0] imm;
        begin
            mk_addi = {6'h10, rd, rs, imm}; // opcode 0x10
        end
    endfunction

    function [31:0] mk_j;
        input [25:0] imm26;
        begin
            mk_j = {6'h28, imm26}; // opcode 0x28
        end
    endfunction

    always @* begin
        case (addr)
            10'd0: data = mk_addi(5'd1, 5'd1, 16'd1);      // ADDI r1, r1, 1
            10'd1: data = mk_addi(5'd2, 5'd2, 16'd2);      // ADDI r2, r2, 2
            10'd2: data = mk_j(26'd0);                     // J 0
            default: data = 32'h00000000;                  // NOP
        endcase
    end

endmodule
