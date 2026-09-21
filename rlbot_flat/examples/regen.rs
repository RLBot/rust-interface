// Regenerate `src/planus_flat.rs` from the flatbuffers-schema submodule:
// cargo run -p rlbot_flat --example regen
#[path = "../codegen/mod.rs"]
mod codegen;

fn main() -> eyre::Result<()> {
    codegen::main()
}
