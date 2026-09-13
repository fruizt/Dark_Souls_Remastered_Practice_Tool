use libdsr::funcs::BonfireWarp;
use libdsr::memedit::PointerChain;
use practice_tool_core::key::Key;
use practice_tool_core::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};

use crate::widgets::id_list::IdList;

const BONFIRE_TAG: &str = "##bonfires";

/// The warp menu the game's own one cannot be.
///
/// Pick a bonfire and go: "Warp here" writes it as your last bonfire and calls
/// the game's travel routine in one press. Set and Warp stay separate too,
/// since setting alone is what you want when you are about to die on purpose.
///
/// Unlike the in-game travel menu this needs no Lordvessel and no bonfire to
/// rest at — the destination list is ours, not one the game populates.
struct LastBonfire {
    ptr: PointerChain<u32>,
    warp: BonfireWarp,
    known: IdList,
    id_input: String,
    status: String,
    key_warp: Option<Key>,
    key_close: Key,
}

impl LastBonfire {
    fn new(
        ptr: PointerChain<u32>,
        warp: BonfireWarp,
        key_warp: Option<Key>,
        key_close: Key,
    ) -> Self {
        Self {
            ptr,
            warp,
            known: IdList::new("bonfires", include_str!("bonfire_ids.json")),
            id_input: String::new(),
            status: String::new(),
            key_warp,
            key_close,
        }
    }

    fn parsed_id(&self) -> Option<u32> {
        self.id_input.trim().parse::<u32>().ok()
    }

    /// Set the destination and travel to it, which is what "a warp menu" means.
    fn warp_to_typed(&mut self) {
        let Some(id) = self.parsed_id() else {
            self.status = String::from("not a bonfire ID");
            return;
        };

        if self.ptr.write(id).is_none() {
            self.status = String::from("write failed");
            return;
        }

        self.status = if self.warp.warp() {
            format!("warping to {id}")
        } else {
            String::from("nothing to warp with")
        };
    }

    /// Travel to whatever is already stored, without touching it. The hotkey
    /// does this rather than the typed ID: on a practice loop you want to be
    /// back where you were, not wherever the box happens to say.
    fn warp_to_stored(&mut self) {
        self.status = if self.warp.warp() {
            String::from("warping")
        } else {
            String::from("nothing to warp with")
        };
    }
}

impl Widget for LastBonfire {
    fn render(&mut self, ui: &imgui::Ui) {
        let scale = scaling_factor(ui);
        let button_width = BUTTON_WIDTH * scale;

        if ui.button_with_size("Warp menu", [button_width, BUTTON_HEIGHT]) {
            ui.open_popup(BONFIRE_TAG);
        }

        // Its own popup rather than living inline in a group: a self-sizing popup
        // cannot clip its last line the way a fixed group can.
        if let Some(_token) = ui
            .modal_popup_config(BONFIRE_TAG)
            .resizable(false)
            .movable(false)
            .title_bar(false)
            .scroll_bar(false)
            .begin_popup()
        {
            let current = self.ptr.read();

            if let Some(id) = self.known.render(ui, button_width) {
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

            // Calling into the game is the one thing here that can take the
            // process down, so these are dead unless there is something to call
            // with.
            let _token = ui.begin_disabled(!self.warp.is_ready());
            if ui.button_with_size("Warp here", [button_width, BUTTON_HEIGHT]) {
                self.warp_to_typed();
            }
            if ui.button_with_size("Warp to last bonfire", [button_width, BUTTON_HEIGHT]) {
                self.warp_to_stored();
            }
            drop(_token);

            if !self.status.is_empty() {
                ui.text(&self.status);
            }

            ui.separator();

            if ui.button_with_size("Close", [button_width, BUTTON_HEIGHT])
                || (!ui.is_any_item_active() && self.key_close.is_pressed(ui))
            {
                ui.close_current_popup();
            }
        }
    }

    fn interact(&mut self, ui: &imgui::Ui) {
        if self.key_warp.map(|k| k.is_pressed(ui)).unwrap_or(false) {
            self.warp_to_stored();
        }
    }
}

pub(crate) fn last_bonfire(
    ptr: PointerChain<u32>,
    warp: BonfireWarp,
    key_warp: Option<Key>,
    key_close: Key,
) -> Box<dyn Widget> {
    Box::new(LastBonfire::new(ptr, warp, key_warp, key_close))
}
