use libdsr::prelude::PointerChain;
use practice_tool_core::key::Key;
use practice_tool_core::widgets::store_value::{ReadWrite, StoreValue};
use practice_tool_core::widgets::{label, Widget};

#[derive(Debug)]
struct WrapMenu {
    ptr: PointerChain<u8>,
    current: u8,
    amount: u8,
    label: String,
}

impl WrapMenu {
    fn new(ptr: PointerChain<u8>) -> Self {
        WrapMenu { ptr, current: 0, amount: 1, label: "Open Wrap Menu".to_string() }
    }
}

impl ReadWrite for WrapMenu {
    fn read(&mut self) -> bool {
        if let Some(current) = self.ptr.read() {
            self.current = current;
            true
        } else {
            false
        }
    }

    fn write(&mut self) {
        self.ptr.write(self.amount);
    }

    fn label(&self) -> &str {
        &self.label
    }
}

pub(crate) fn warp_menu(ptr: PointerChain<u8>, key: Option<Key>) -> Box<dyn Widget> {
    Box::new(StoreValue::new(WrapMenu::new(ptr), key))
}
