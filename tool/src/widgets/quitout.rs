use libdsr::memedit::PointerChain;
use practice_tool_core::key::Key;
use practice_tool_core::widgets::store_value::{ReadWrite, StoreValue};
use practice_tool_core::widgets::Widget;

/// Quit to the main menu without touching the pause screen.
///
/// The game's menu manager exposes a "kick" field; writing 2 into it is what
/// the quit-to-menu path itself does, so the save is written normally on the
/// way out.
struct Quitout {
    ptr: PointerChain<i32>,
}

impl ReadWrite for Quitout {
    fn read(&mut self) -> bool {
        self.ptr.eval().is_some()
    }

    fn write(&mut self) {
        self.ptr.write(2);
    }

    fn label(&self) -> &str {
        "Quitout"
    }
}

pub(crate) fn quitout(ptr: PointerChain<i32>, key: Option<Key>) -> Box<dyn Widget> {
    Box::new(StoreValue::new(Quitout { ptr }, key))
}
