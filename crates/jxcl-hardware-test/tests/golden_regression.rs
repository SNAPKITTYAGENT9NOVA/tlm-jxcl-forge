//! Golden-file regression tests for `jxcl-hardware`'s emitted
//! Verilog/VHDL text and the toy synthesis pass's netlist output.
//!
//! See `jxcl-hardware-test`'s crate docs for the honesty boundary: a
//! pass here means "matches the checked-in golden file," never "was
//! simulated or synthesized by a real toolchain" (none is available in
//! this environment -- `docs/HARDWARE_LIMITATIONS.md`).

#[test]
fn emitted_verilog_matches_golden_file() {
    let text = jxcl_hardware::emit_verilog();
    let expected = include_str!("golden/core_datapath.v");
    assert_eq!(
        text, expected,
        "jxcl_hardware::emit_verilog() drifted from tests/golden/core_datapath.v -- \
         if this is an intentional RTL change, regenerate the golden file"
    );
}

#[test]
fn emitted_vhdl_matches_golden_file() {
    let text = jxcl_hardware::emit_vhdl();
    let expected = include_str!("golden/core_datapath.vhdl");
    assert_eq!(
        text, expected,
        "jxcl_hardware::emit_vhdl() drifted from tests/golden/core_datapath.vhdl -- \
         if this is an intentional RTL change, regenerate the golden file"
    );
}

#[test]
fn emitted_mux2_netlist_matches_golden_file() {
    // The canonical mux2 example is the one module that actually falls
    // inside jxcl-synthesis's narrow toy scope (see jxcl-hardware's
    // NetlistReport docs for why none of the three real core datapath
    // modules do, today).
    let netlist = jxcl_hardware::emit_netlist_for(&jxcl_hdl::examples::mux2_module())
        .expect("mux2 is in the toy synthesis pass's documented scope");
    netlist
        .validate()
        .expect("synthesized netlist must be structurally well-formed");

    let text = jxcl_hardware_test::render_netlist_text(&netlist);
    let expected = include_str!("golden/mux2.netlist.txt");
    assert_eq!(
        text, expected,
        "synthesized mux2 netlist drifted from tests/golden/mux2.netlist.txt"
    );
}

#[test]
fn core_datapath_netlist_report_is_still_all_out_of_scope() {
    // A regression guard on the honesty claim itself: if a future
    // change to jxcl-rtl's generated modules or jxcl-synthesis's scope
    // ever makes one of these actually synthesizable, this test will
    // fail here (loudly, in a test named for exactly that) rather than
    // silently starting to under-report scope.
    let report = jxcl_hardware::emit_netlist();
    assert!(report.decoder.is_err());
    assert!(report.alu.is_err());
    assert!(report.register_file.is_err());
}
