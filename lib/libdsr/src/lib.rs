pub mod codegen;
pub mod event_flags;
pub mod funcs;
pub mod memedit;
pub mod pointers;
pub mod version;

pub mod prelude {
    pub use crate::codegen::*;
    pub use crate::event_flags::*;
    pub use crate::funcs::*;
    pub use crate::memedit::*;
    pub use crate::pointers::*;
    pub use crate::version::*;
}

fn main() {
    println!("Hello, world!");
}
