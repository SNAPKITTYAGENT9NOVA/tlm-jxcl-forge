module mux2(
    input wire sel,
    input wire a,
    input wire b,
    output reg y
);

    always @(*) begin
        case (sel)
            1'd0: begin
                y = a;
            end
            1'd1: begin
                y = b;
            end
        endcase
    end

endmodule
