module decoder(
    input wire [7:0] opcode,
    output reg valid,
    output reg [3:0] instr_len
);

    always @(*) begin
        case (opcode)
            8'd0: begin
                valid = 1'd1;
                instr_len = 4'd1;
            end
            8'd1: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd2: begin
                valid = 1'd1;
                instr_len = 4'd10;
            end
            8'd3: begin
                valid = 1'd1;
                instr_len = 4'd7;
            end
            8'd4: begin
                valid = 1'd1;
                instr_len = 4'd7;
            end
            8'd5: begin
                valid = 1'd1;
                instr_len = 4'd2;
            end
            8'd6: begin
                valid = 1'd1;
                instr_len = 4'd2;
            end
            8'd7: begin
                valid = 1'd1;
                instr_len = 4'd7;
            end
            8'd16: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd17: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd18: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd19: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd20: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd21: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd22: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd23: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd24: begin
                valid = 1'd1;
                instr_len = 4'd2;
            end
            8'd25: begin
                valid = 1'd1;
                instr_len = 4'd2;
            end
            8'd26: begin
                valid = 1'd1;
                instr_len = 4'd2;
            end
            8'd32: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd33: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd34: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd35: begin
                valid = 1'd1;
                instr_len = 4'd2;
            end
            8'd36: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd37: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd38: begin
                valid = 1'd1;
                instr_len = 4'd4;
            end
            8'd48: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd49: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd50: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd51: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd52: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd64: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd65: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd80: begin
                valid = 1'd1;
                instr_len = 4'd5;
            end
            8'd81: begin
                valid = 1'd1;
                instr_len = 4'd5;
            end
            8'd82: begin
                valid = 1'd1;
                instr_len = 4'd1;
            end
            8'd83: begin
                valid = 1'd1;
                instr_len = 4'd5;
            end
            8'd84: begin
                valid = 1'd1;
                instr_len = 4'd5;
            end
            8'd85: begin
                valid = 1'd1;
                instr_len = 4'd5;
            end
            8'd86: begin
                valid = 1'd1;
                instr_len = 4'd5;
            end
            8'd87: begin
                valid = 1'd1;
                instr_len = 4'd5;
            end
            8'd88: begin
                valid = 1'd1;
                instr_len = 4'd5;
            end
            8'd89: begin
                valid = 1'd1;
                instr_len = 4'd5;
            end
            8'd90: begin
                valid = 1'd1;
                instr_len = 4'd5;
            end
            8'd96: begin
                valid = 1'd1;
                instr_len = 4'd1;
            end
            8'd97: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd98: begin
                valid = 1'd1;
                instr_len = 4'd3;
            end
            8'd112: begin
                valid = 1'd1;
                instr_len = 4'd8;
            end
            8'd113: begin
                valid = 1'd1;
                instr_len = 4'd7;
            end
            8'd114: begin
                valid = 1'd1;
                instr_len = 4'd1;
            end
            default: begin
                valid = 1'd0;
                instr_len = 4'd0;
            end
        endcase
    end

endmodule

module alu(
    input wire [63:0] op_a,
    input wire [63:0] op_b,
    input wire [7:0] alu_op,
    output wire [63:0] result
);

endmodule

module register_file(
    input wire [4:0] read_addr1,
    input wire [4:0] read_addr2,
    output wire [63:0] read_data1,
    output wire [63:0] read_data2,
    input wire [4:0] write_addr,
    input wire [63:0] write_data,
    input wire write_enable
);

endmodule
