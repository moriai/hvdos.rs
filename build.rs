extern crate cc;
extern crate rustc_tools_util;

fn main() {
    println!(
        "cargo:rustc-env=GIT_HASH={}",
        rustc_tools_util::get_commit_hash().unwrap_or_default()
    );
    println!(
        "cargo:rustc-env=COMMIT_DATE={}",
        rustc_tools_util::get_commit_date().unwrap_or_default()
    );
    println!(
        "cargo:rustc-env=RUSTC_RELEASE_CHANNEL={}",
        rustc_tools_util::get_channel().unwrap_or_default()
    );

    println!("cargo:rustc-link-lib=c++");

    let debug = if cfg!(debug) { "1" } else { "0" };
    cc::Build::new()
        .cpp(true)
        .define("DEBUG", debug)
        .file("src/DOSKernel.cpp")
        .file("src/DOSKernelWrapper.cpp")
        .flag("-std=c++17")
        .flag("-Wno-unused-parameter")
        //.flag("-framework Hypervisor")
        .include("src")
        .compile("DOSKernel");
}
