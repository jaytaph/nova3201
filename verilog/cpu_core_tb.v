`timescale 1ns/1ps

module cpu_core_tb;
  reg clk;
  reg reset;

  cpu_core uut (
      .clk(clk),
      .reset(reset)
  );

  initial begin
    clk = 0;
    forever #5 clk = ~clk;
  end

  initial begin

    $monitor("Time=%0t clk=%b reset=%b alu_op=%d rd=%h", $time, clk, reset, uut.alu_op, uut.alu_rd);
    $dumpfile("cpu_core.vcd");
    $dumpvars(0, cpu_core_tb);

    reset = 1;
    #20;
    reset = 0;

    #1000;

    $display("Test completed");
    $finish;
  end
endmodule
