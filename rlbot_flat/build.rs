mod codegen;
fn main() -> eyre::Result<()> {
    println!("cargo:rerun-if-changed=../flatbuffers-schema");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=codegen");

    codegen::main()
}
