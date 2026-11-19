mod backend;
mod frontend;
mod ir;
mod opt;
mod structs;

pub mod driver;

mod globals;
mod index;
mod options;
mod span;
mod symbol;
mod types;

use globals::with_session_globals;
use symbol::Symbol;
