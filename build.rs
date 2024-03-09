// use std::env;
// use std::path::PathBuf;

fn main() {
    embuild::espidf::sysenv::output();

    // Uncomment the section below to generate bindings here instead of through esp-idf

    // let allow_vars = [
    //     "EEPROM_SIZE",
    //     "SECRET_KEY_SIZE",
    //     "CHALLENGE_SIZE",
    //     "RESP_BUF_SIZE",
    //     "YUBIKEY_AID_LENGTH",
    //     "YUBIKEY_AID",
    //     "SLOT_1",
    //     "SLOT_2",
    // ];
    //
    // let allow_functions = [
    //     // C functions to use in Rust
    //     "ykhmac_select",
    //     "ykhmac_read_serial",
    //     "ykhmac_read_version",
    //     "ykhmac_exchange_hmac",
    //     "ykhmac_find_slots",
    //     "ykhmac_enroll_key",
    //     "ykhmac_authenticate",
    //     "ykhmac_compute_hmac",
    //     // Rust functions to be used in C
    //     // Bindings for these should NOT be generated.
    //     // You must define them as `pub extern "C" fn` yourself,
    //     // but you may temporarily generate bindings to see the required signatures.
    //     // "ykhmac_data_exchange",
    //     // "ykhmac_random",
    //     // "ykhmac_presistent_write",
    //     // "ykhmac_presistent_read",
    //
    //     // Debug function. Same as above, but not mandatory.
    //     // "ykhmac_debug_print",
    // ];
    //
    // let mut builder = bindgen::Builder::default()
    //     .use_core()
    //     .header("components/ykhmac/include/ykhmac.h")
    //     .clang_arg("-Icomponents/aes/include");
    //
    // for var in allow_vars {
    //     builder = builder.allowlist_var(var);
    // }
    // for function in allow_functions {
    //     builder = builder.allowlist_function(function);
    // }
    //
    // let bindings = builder
    //     // Tell cargo to invalidate the built crate whenever any of the
    //     // included header files changed.
    //     .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
    //     // Finish the builder and generate the bindings.
    //     .generate()
    //     .expect("Unable to generate bindings");
    //
    // // Write the bindings to the $OUT_DIR/bindings.rs file.
    // let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    // bindings
    //     .write_to_file(out_path.join("bindings.rs"))
    //     .expect("Couldn't write bindings!");
}
