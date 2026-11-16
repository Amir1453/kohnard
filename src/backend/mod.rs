pub mod x86;

pub trait CodegenBackend {
    fn lower(cutac: crate::ir::tac::CUTAC) -> anyhow::Result<String>;
}
