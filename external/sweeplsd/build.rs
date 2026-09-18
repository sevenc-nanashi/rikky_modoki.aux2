fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=include");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag_if_supported("/utf-8")
        .flag_if_supported("/EHsc")
        .include("include")
        .file("src/labeling.cpp")
        .file("src/sweeplsd_onepass.cpp")
        .file("src/bridge.cpp")
        .compile("sweeplsd");
}
