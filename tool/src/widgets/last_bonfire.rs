use libdsr::memedit::PointerChain;
use practice_tool_core::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};

/// Edit the bonfire the game will send you to on the next load.
///
/// Paired with quitout this is a warp: set the ID, quit out, load back in. IDs
/// are the game's own bonfire entity IDs, e.g. `1512960` for Firelink Shrine.
struct LastBonfire {
    ptr: PointerChain<u32>,
    id_input: String,
    status: String,
}

impl LastBonfire {
    fn new(ptr: PointerChain<u32>) -> Self {
        Self { ptr, id_input: String::new(), status: String::new() }
    }
}

impl Widget for LastBonfire {
    fn render(&mut self, ui: &imgui::Ui) {
        let scale = scaling_factor(ui);
        let button_width = BUTTON_WIDTH * scale;

        let current = self.ptr.read();

        ui.set_next_item_width(button_width);
        ui.input_text("##last_bonfire_id", &mut self.id_input).hint("bonfire ID").build();

        let half = (button_width - 8. * scale) / 2.;

        let _token = ui.begin_disabled(current.is_none());
        if ui.button_with_size("Read bonfire", [half, BUTTON_HEIGHT]) {
            match current {
                Some(id) => {
                    self.id_input = id.to_string();
                    self.status = format!("last bonfire is {id}");
                },
                None => self.status = String::from("no character loaded"),
            }
        }
        ui.same_line();
        if ui.button_with_size("Set bonfire", [half, BUTTON_HEIGHT]) {
            match self.id_input.trim().parse::<u32>() {
                Ok(id) => {
                    self.status = match self.ptr.write(id) {
                        Some(()) => format!("last bonfire set to {id}"),
                        None => String::from("write failed"),
                    };
                },
                Err(_) => self.status = String::from("not a bonfire ID"),
            }
        }
        drop(_token);

        if !self.status.is_empty() {
            ui.text(&self.status);
        }
    }
}

pub(crate) fn last_bonfire(ptr: PointerChain<u32>) -> Box<dyn Widget> {
    Box::new(LastBonfire::new(ptr))
}
