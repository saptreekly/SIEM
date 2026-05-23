fn main() {
    println!("cargo:rerun-if-changed=src/asm/fnv1a_x86_64.s");
    println!("cargo:rerun-if-changed=src/asm/fnv1a_aarch64.s");
    let target = std::env::var("TARGET").unwrap();
    let mut build = cc::Build::new();

    if target.contains("x86_64") {
        println!("cargo:warning=Compiling x86_64 assembly");
        build.file("src/asm/fnv1a_x86_64.s");
        build.file("src/asm/parse_timestamp_x86_64.s");
    } else if target.contains("aarch64") {
        println!("cargo:warning=Compiling aarch64 assembly");
        build.file("src/asm/fnv1a_aarch64.s");
        build.file("src/asm/parse_timestamp_aarch64.s");
    } else {
        panic!("Unsupported architecture");
    }

    build.compile("fnv1a");
}
