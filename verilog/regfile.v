// regfile.v
// 32 x 32-bit register file
// - 2 read ports (combinational)
// - 1 write port (sequential)
// - r0 is hardwired to 0

module regfile (
    input  wire        clk,
    input  wire        reset,

    // write port
    input  wire        we,          // Write is enabled
    input  wire [4:0]  rd_addr,     // Register to write to (rd)
    input  wire [31:0] rd_data,     // Data to write

    // read port 1
    input  wire [4:0]  rs_addr,      // Register to read from (rs)
    output wire [31:0] rs_data,      // Data read

    // read port 2
    input  wire [4:0]  rt_addr,      // Register to read from (rt)
    output wire [31:0] rt_data,      // Data read

    output wire [31:0] debug_r1
);

    reg [31:0] regs [31:0];
    integer i;

    always @(posedge clk) begin
        if (reset) begin
            // On a high reset, we clear all registers to 0
            for (i = 0; i < 32; i = i + 1) begin
                regs[i] <= 32'h0000_0000;
            end
        end else begin
            // If we need to write, and the address is not r0, then write
            if (we && (rd_addr != 5'd0)) begin
                regs[rd_addr] <= rd_data;
            end
        end
    end

    // Combinatorial read port. Will update immediately without waiting for clock edge
    assign rs_data = (rs_addr == 5'd0) ? 32'h0000_0000 : regs[rs_addr];
    assign rt_data = (rt_addr == 5'd0) ? 32'h0000_0000 : regs[rt_addr];

    assign debug_r1 = regs[1];  // We use this for debugging

endmodule