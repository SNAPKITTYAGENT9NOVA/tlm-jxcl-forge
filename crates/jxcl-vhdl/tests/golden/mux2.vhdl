entity mux2 is
    port (
        sel : in std_logic;
        a : in std_logic;
        b : in std_logic;
        y : out std_logic
    );
end entity mux2;

architecture rtl of mux2 is
begin
    process (all) is
    begin
        case sel is
            when '0' =>
                y <= a;
            when '1' =>
                y <= b;
        end case;
    end process;
end architecture rtl;
