use std::collections::{HashMap, hash_map::Entry};
use std::path::PathBuf;

use anyhow::bail;

use crate::backend::Asm;
use crate::frontend::ast::{Declaration, Program};
use crate::frontend::{BXParser, Lexer, RetChecker, SemChecker, Symbol, TypeChecker};
use crate::globals;
use crate::ir::MM;
// use crate::opt::Optimizer;
use crate::options::CompilerOptions;
use crate::types::{ProcType, SemanticType};

pub struct Driver {}

impl Driver {
    /// The CLI entrypoint of the compiler, where the CompilerOptions are provided by shell
    /// arguments.
    pub fn cli() -> anyhow::Result<()> {
        let options = CompilerOptions::from_env()?;

        Self::entrypoint(options)
    }

    /// The testing entrypoint of the compiler, intended for use with integration tests.
    pub fn drive(cu: &PathBuf, out_path: &PathBuf) -> anyhow::Result<()> {
        let options = CompilerOptions::from_paths(cu, out_path);

        Self::entrypoint(options)
    }

    /// All entrances are done through here, where we create the session globals.
    /// An Error might contain a Symbol that is only meaningful when the session globals exist.
    /// This case must be handled.
    fn entrypoint(options: CompilerOptions) -> anyhow::Result<()> {
        globals::create_session_globals_then(|| {
            Driver::compile(options).map_err(|error| anyhow::anyhow!("{error:#}"))
        })
    }

    /// The main compilation pipeline of the compiler, lexing, parsing, semantic checking, type
    /// checking, munching, optimizations, and lowering are all done here.
    fn compile(options: CompilerOptions) -> anyhow::Result<()> {
        // Generate the parser
        let parser = BXParser::new();

        // Accumulate compilation unit programs
        let mut cu_programs: Vec<(PathBuf, Program)> = Vec::new();
        for cu in options.compilation_units.into_iter() {
            let cu_source_code = std::fs::read_to_string(&cu)?;
            let cu_lexer = Lexer::new(&cu_source_code);
            let cu_program = parser.parse(cu_lexer)?;

            cu_programs.push((cu, cu_program));
        }

        let mut global_decls: HashMap<Symbol, SemanticType> = HashMap::new();

        let mut insert_global_decl = |name: Symbol, ty: SemanticType| match global_decls.entry(name)
        {
            Entry::Vacant(entry) => {
                entry.insert(ty);
                Ok(())
            }
            Entry::Occupied(entry) => {
                bail!("duplicate global declaration: {}", entry.key())
            }
        };

        for (_, cu_program) in &cu_programs {
            for decl in &cu_program.0 {
                match decl {
                    Declaration::Variable(variable) => {
                        let var = variable.as_ref();

                        for name in var.names.clone().into_iter() {
                            insert_global_decl(name, SemanticType::SimpleType(var.ty))?
                        }
                    }

                    Declaration::Proc {
                        proc_name,
                        proc_args,
                        return_type,
                        ..
                    } => {
                        let ty = SemanticType::ProcType(ProcType {
                            args_type: proc_args.clone(),
                            return_type: *return_type,
                        });
                        insert_global_decl(*proc_name, ty)?
                    }
                }
            }
        }

        // Making sure a main function exists
        let main_sym = Symbol::intern("main");
        if !(global_decls.contains_key(&main_sym)
            && matches!(global_decls.get(&main_sym), Some(SemanticType::ProcType(_))))
        {
            bail!("No main() ! Maybe I am a linker ?")
        }

        for (cu, mut cu_program) in cu_programs.into_iter() {
            SemChecker::check(&cu_program, global_decls.keys().cloned().collect())?;
            TypeChecker::check(&mut cu_program, global_decls.clone())?;
            RetChecker::check(&cu_program)?;

            let cutac = MM::munch(cu_program);
            // let cutac = Optimizer::optimize(cutac);
            let asm = Asm::lower(cutac);

            let out_path = options.out_path.clone().unwrap_or_else(|| {
                let mut path = PathBuf::from(
                    cu.file_stem()
                        .and_then(|s| s.to_str())
                        .expect("file stem should be valid UTF-8"),
                );
                path.set_extension("s");
                path
            });

            std::fs::write(out_path, &asm)?;
        }

        Ok(())
    }
}
