fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    println!("cargo:rustc-env=DYNO_TARGET={target}");
    println!("cargo:rerun-if-changed=build.rs");
}
