use anyhow::Result;

mod codegen;

fn main() -> Result<()> {
    load_dotenv();

    codegen::codegen()
}

/// Pull `DSR_INSTALLS_DIR` out of `.env` if there is one.
///
/// A missing `.env` is fine — the variable may come from the real environment
/// instead. A malformed one is not, and is easy to produce by accident: dotenv
/// 0.15 processes escapes inside double quotes, so a Windows path written as
/// `KEY="D:\Steam\..."` fails to parse and would otherwise be swallowed here,
/// leaving you with a confusing "not set" error. Single-quote such paths.
fn load_dotenv() {
    if let Err(e) = dotenv::dotenv() {
        if !e.not_found() {
            eprintln!("warning: ignoring .env: {e}");
            eprintln!(
                r"         note: single-quote Windows paths, e.g. DSR_INSTALLS_DIR='D:\Steam\common'"
            );
        }
    }
}
