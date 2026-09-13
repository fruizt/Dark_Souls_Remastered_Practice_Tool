use libdsr::funcs::BonfireWarp;
use libdsr::memedit::PointerChain;
use practice_tool_core::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};

use crate::widgets::id_list::IdList;

/// Set the bonfire you last rested at, and travel to it.
///
/// IDs are the game's own bonfire IDs, e.g. `1022960` for Firelink Shrine and
/// `1012962` for Undead Burg. Setting one alone changes where you wake up after
/// dying; Warp calls the game's travel routine and takes you there now.
struct LastBonfire {
    ptr: PointerChain<u32>,
    warp: BonfireWarp,
    known: IdList,
    id_input: String,
    status: String,
}

impl LastBonfire {
    fn new(ptr: PointerChain<u32>, warp: BonfireWarp) -> Self {
        Self {
            ptr,
            warp,
            known: IdList::new("bonfires", include_str!("bonfire_ids.json")),
            id_input: String::new(),
            status: String::new(),
        }
    }
}

impl Widget for LastBonfire {
    fn render(&mut self, ui: &imgui::Ui) {
        let scale = scaling_factor(ui);
        let button_width = BUTTON_WIDTH * scale;

        let current = self.ptr.read();

        if let Some(id) = self.known.render(ui, button_width, 120.) {
            self.id_input = id.to_string();
            self.status = format!("{id} selected, not set yet");
        }

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

        // Calling into the game is the one thing here that can take the process
        // down, so the button is dead unless there is something to call with.
        let _token = ui.begin_disabled(!self.warp.is_ready());
        if ui.button_with_size("Warp to bonfire", [button_width, BUTTON_HEIGHT]) {
            self.status = if self.warp.warp() {
                String::from("warping")
            } else {
                String::from("nothing to warp to")
            };
        }
        drop(_token);

        if !self.status.is_empty() {
            ui.text(&self.status);
        }
    }
}

pub(crate) fn last_bonfire(ptr: PointerChain<u32>, warp: BonfireWarp) -> Box<dyn Widget> {
    Box::new(LastBonfire::new(ptr, warp))
}
