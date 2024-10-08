use crate::{snake_to_camel, BuildMode, Edger8r, EnclaveSharedObjectBuilder, Env, SgxSigner, TrustedProxyBuilder, UntrustedProxyBuilder};

pub fn build_sgx_app() {
    build_enclave_objs();
    println!(
        "cargo:rustc-link-search=native={}",
        Env::sgx_lib_path().display()
    );
    match Env::sgx_mode().as_str() {
        "SIM" | "SW" => println!("cargo:rustc-link-lib=dylib=sgx_urts_sim"),
        "HYPER" => println!("cargo:rustc-link-lib=dylib=sgx_urts_hyper"),
        "HW" => println!("cargo:rustc-link-lib=dylib=sgx_urts"),
        _ => println!("cargo:rustc-link-lib=dylib=sgx_urts"),
    }
}

pub fn build_enclave_objs() {
    let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap();
    let cargo_sgx_output = match Env::cargo_sgx_output() {
        Some(n) => n,
        None => {
            println!("cargo:warning={} is intended to build from `cargo sgx build`, please try install it by `cargo install cargo-sgx`", pkg_name);
            return;
        }
    };
    let mode = BuildMode::BuildScript;
    let out_dir = Env::out_dir();
    let proxy_trusted_dir = out_dir.join("proxy_trusted");
    let proxy_untrusted_dir = out_dir.join("proxy_untrusted");
    for enclave in &cargo_sgx_output.metadata {
        mode.trace_file(&enclave.enclave_archive);
        let edl_name = enclave.edl.file_stem().unwrap().to_str().unwrap();
        let enclave_name = enclave
            .enclave_archive
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        println!(
            "cargo:rustc-env=ENCLAVE_{}=1",
            snake_to_camel(enclave_name.trim_start_matches("lib"))
        );
        let proxy_trusted_source = Edger8r::new(mode).build(&enclave.edl, true, &proxy_trusted_dir);
        let proxy_untrusted_source =
            Edger8r::new(mode).build(&enclave.edl, false, &proxy_untrusted_dir);

        UntrustedProxyBuilder::new(mode).build(
            &proxy_untrusted_source,
            &proxy_untrusted_dir.join(format!("{}_u.o", edl_name)),
        );
        TrustedProxyBuilder::new(mode).build(
            &proxy_trusted_source,
            &proxy_trusted_dir.join(format!("{}_t.o", edl_name)),
        );
        EnclaveSharedObjectBuilder::new(mode).build(
            &proxy_trusted_dir.join(format!("{}_t.o", edl_name)),
            &enclave.enclave_archive,
            &enclave.lds,
            &out_dir.join(format!("{}.so", enclave_name)),
        );
        SgxSigner::new(mode).sign(
            &enclave.config,
            &enclave.output_signed_so,
            &out_dir.join(format!("{}.so", enclave_name)),
            &enclave.key,
        );
    }
}
