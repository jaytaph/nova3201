// regfile.v
// 32 x 32-bit register file
// - 2 read ports (combinational)
// - 1 write port (sequential)
// - r0 is hardwired to 0

module regfile (
    input  wire        clk,
    input  wire        reset,

    // write port
    input  wire        we,
    input  wire [4:0]  waddr,
    input  wire [31:0] wdata,

    // read port 1
    input  wire [4:0]  raddr1,
    output wire [31:0] rdata1,

    // read port 2
    input  wire [4:0]  raddr2,
    output wire [31:0] rdata2,

    output wire [31:0] debug_r1
);

    reg [31:0] regs [31:0];
    integer i;

    always @(posedge clk) begin
        if (reset) begin
            for (i = 0; i < 32; i = i + 1) begin
                regs[i] <= 32'h0000_0000;
            end
        end else begin
            if (we && (waddr != 5'd0)) begin
                regs[waddr] <= wdata;
            end
        end
    end

    assign rdata1 = (raddr1 == 5'd0) ? 32'h0000_0000 : regs[raddr1];
    assign rdata2 = (raddr2 == 5'd0) ? 32'h0000_0000 : regs[raddr2];
    assign debug_r1 = regs[1];

endmodule