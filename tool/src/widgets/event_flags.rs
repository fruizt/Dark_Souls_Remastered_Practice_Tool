use libdsr::event_flags::EventFlags as EventFlagsInner;
use practice_tool_core::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};

use crate::widgets::id_list::IdList;

/// Read and flip a story flag by ID.
///
/// Useful for putting a save into a given route state: mark a boss dead, a door
/// opened, a covenant joined. The ID is the same one the game's event scripts
/// use. Main boss kills are single digit flags -- `16` is Asylum Demon, `15`
/// Gwyn, `3` the Bell Gargoyles -- while the rest are eight digit, e.g.
/// `11010901` for Taurus Demon.
struct EventFlagsWidget {
    flags: EventFlagsInner,
    known: IdList,
    id_input: String,
    status: String,
}

impl EventFlagsWidget {
    fn new(flags: EventFlagsInner) -> Self {
        Self {
            flags,
            known: IdList::new("event-flags", include_str!("event_flag_ids.json")),
            id_input: String::new(),
            status: String::new(),
        }
    }

    fn parsed_id(&self) -> Option<u32> {
        self.id_input.trim().parse::<u32>().ok()
    }

    fn refresh(&mut self) {
        let Some(id) = self.parsed_id() else {
            self.status = String::from("not a flag ID");
            return;
        };

        self.status = match self.flags.read(id) {
            Some(true) => format!("{id} is on"),
            Some(false) => format!("{id} is off"),
            None => format!("{id} unreadable"),
        };
    }

    fn set(&mut self, value: bool) {
        let Some(id) = self.parsed_id() else {
            self.status = String::from("not a flag ID");
            return;
        };

        match self.flags.write(id, value) {
            Some(()) => self.refresh(),
            None => self.status = format!("{id} not writable"),
        }
    }
}

impl Widget for EventFlagsWidget {
    fn render(&mut self, ui: &imgui::Ui) {
        let scale = scaling_factor(ui);
        let button_width = BUTTON_WIDTH * scale;

        // Picking a boss reads its flag straight away, which is the quickest way
        // to see whether a flag means what its ID claims.
        if let Some(id) = self.known.render(ui, button_width, 120.) {
            self.id_input = id.to_string();
            self.refresh();
        }

        ui.set_next_item_width(button_width);
        if ui.input_text("##event_flag_id", &mut self.id_input).hint("event flag ID").build() {
            self.refresh();
        }

        let third = (button_width - 16. * scale) / 3.;

        if ui.button_with_size("Read", [third, BUTTON_HEIGHT]) {
            self.refresh();
        }
        ui.same_line();
        if ui.button_with_size("Set", [third, BUTTON_HEIGHT]) {
            self.set(true);
        }
        ui.same_line();
        if ui.button_with_size("Clear", [third, BUTTON_HEIGHT]) {
            self.set(false);
        }

        if !self.status.is_empty() {
            ui.text(&self.status);
        }
    }
}

pub(crate) fn event_flags(flags: EventFlagsInner) -> Box<dyn Widget> {
    Box::new(EventFlagsWidget::new(flags))
}
