entity decoder is
    port (
        opcode : in std_logic_vector(7 downto 0);
        valid : out std_logic;
        instr_len : out std_logic_vector(3 downto 0)
    );
end entity decoder;

architecture rtl of decoder is
begin
    process (all) is
    begin
        case opcode is
            when "00000000" =>
                valid <= '1';
                instr_len <= "0001";
            when "00000001" =>
                valid <= '1';
                instr_len <= "0011";
            when "00000010" =>
                valid <= '1';
                instr_len <= "1010";
            when "00000011" =>
                valid <= '1';
                instr_len <= "0111";
            when "00000100" =>
                valid <= '1';
                instr_len <= "0111";
            when "00000101" =>
                valid <= '1';
                instr_len <= "0010";
            when "00000110" =>
                valid <= '1';
                instr_len <= "0010";
            when "00000111" =>
                valid <= '1';
                instr_len <= "0111";
            when "00010000" =>
                valid <= '1';
                instr_len <= "0011";
            when "00010001" =>
                valid <= '1';
                instr_len <= "0011";
            when "00010010" =>
                valid <= '1';
                instr_len <= "0011";
            when "00010011" =>
                valid <= '1';
                instr_len <= "0011";
            when "00010100" =>
                valid <= '1';
                instr_len <= "0011";
            when "00010101" =>
                valid <= '1';
                instr_len <= "0011";
            when "00010110" =>
                valid <= '1';
                instr_len <= "0011";
            when "00010111" =>
                valid <= '1';
                instr_len <= "0011";
            when "00011000" =>
                valid <= '1';
                instr_len <= "0010";
            when "00011001" =>
                valid <= '1';
                instr_len <= "0010";
            when "00011010" =>
                valid <= '1';
                instr_len <= "0010";
            when "00100000" =>
                valid <= '1';
                instr_len <= "0011";
            when "00100001" =>
                valid <= '1';
                instr_len <= "0011";
            when "00100010" =>
                valid <= '1';
                instr_len <= "0011";
            when "00100011" =>
                valid <= '1';
                instr_len <= "0010";
            when "00100100" =>
                valid <= '1';
                instr_len <= "0011";
            when "00100101" =>
                valid <= '1';
                instr_len <= "0011";
            when "00100110" =>
                valid <= '1';
                instr_len <= "0100";
            when "00110000" =>
                valid <= '1';
                instr_len <= "0011";
            when "00110001" =>
                valid <= '1';
                instr_len <= "0011";
            when "00110010" =>
                valid <= '1';
                instr_len <= "0011";
            when "00110011" =>
                valid <= '1';
                instr_len <= "0011";
            when "00110100" =>
                valid <= '1';
                instr_len <= "0011";
            when "01000000" =>
                valid <= '1';
                instr_len <= "0011";
            when "01000001" =>
                valid <= '1';
                instr_len <= "0011";
            when "01010000" =>
                valid <= '1';
                instr_len <= "0101";
            when "01010001" =>
                valid <= '1';
                instr_len <= "0101";
            when "01010010" =>
                valid <= '1';
                instr_len <= "0001";
            when "01010011" =>
                valid <= '1';
                instr_len <= "0101";
            when "01010100" =>
                valid <= '1';
                instr_len <= "0101";
            when "01010101" =>
                valid <= '1';
                instr_len <= "0101";
            when "01010110" =>
                valid <= '1';
                instr_len <= "0101";
            when "01010111" =>
                valid <= '1';
                instr_len <= "0101";
            when "01011000" =>
                valid <= '1';
                instr_len <= "0101";
            when "01011001" =>
                valid <= '1';
                instr_len <= "0101";
            when "01011010" =>
                valid <= '1';
                instr_len <= "0101";
            when "01100000" =>
                valid <= '1';
                instr_len <= "0001";
            when "01100001" =>
                valid <= '1';
                instr_len <= "0011";
            when "01100010" =>
                valid <= '1';
                instr_len <= "0011";
            when "01110000" =>
                valid <= '1';
                instr_len <= "1000";
            when "01110001" =>
                valid <= '1';
                instr_len <= "0111";
            when "01110010" =>
                valid <= '1';
                instr_len <= "0001";
            when others =>
                valid <= '0';
                instr_len <= "0000";
        end case;
    end process;
end architecture rtl;

entity alu is
    port (
        op_a : in std_logic_vector(63 downto 0);
        op_b : in std_logic_vector(63 downto 0);
        alu_op : in std_logic_vector(7 downto 0);
        result : out std_logic_vector(63 downto 0)
    );
end entity alu;

architecture rtl of alu is
begin
end architecture rtl;

entity register_file is
    port (
        read_addr1 : in std_logic_vector(4 downto 0);
        read_addr2 : in std_logic_vector(4 downto 0);
        read_data1 : out std_logic_vector(63 downto 0);
        read_data2 : out std_logic_vector(63 downto 0);
        write_addr : in std_logic_vector(4 downto 0);
        write_data : in std_logic_vector(63 downto 0);
        write_enable : in std_logic
    );
end entity register_file;

architecture rtl of register_file is
begin
end architecture rtl;
