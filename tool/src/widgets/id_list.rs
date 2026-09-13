use imgui::{StyleColor, StyleVar, Ui};
use practice_tool_core::widgets::scaling_factor;
use serde::Deserialize;

/// Rows a list shows before it starts scrolling.
pub(crate) const MAX_ROWS: usize = 8;

/// "8 of 67 shown" — says both that the box is hiding something and how much
/// the search narrowed it. Without this the only clue is a scrollbar.
pub(crate) fn count_label(ui: &Ui, shown: usize, total: usize) {
    if shown == total {
        ui.text_disabled(format!("{total} items"));
    } else {
        ui.text_disabled(format!("{shown} of {total}"));
    }
}

/// Draws `body` inside a child window sized to `rows`, capped at [`MAX_ROWS`].
///
/// The scrollbar is forced on and given some contrast: the default only appears
/// once content overflows, and is nearly invisible against a dark background,
/// so a long list looked like a short one. Sizing to content matters for the
/// same reason — when a full box means "there is more below", a box that is not
/// full means the opposite.
pub(crate) fn scrollable_list(ui: &Ui, tag: &str, width: f32, rows: usize, body: impl FnOnce()) {
    let row_height = ui.text_line_height_with_spacing();
    let visible = rows.clamp(1, MAX_ROWS);
    let height = row_height * visible as f32 + 6. * scaling_factor(ui);

    let _size = ui.push_style_var(StyleVar::ScrollbarSize(12.));
    let _bg = ui.push_style_color(StyleColor::ScrollbarBg, [0.10, 0.11, 0.13, 0.85]);
    let _grab = ui.push_style_color(StyleColor::ScrollbarGrab, [0.55, 0.57, 0.60, 1.0]);
    let _grab_hover =
        ui.push_style_color(StyleColor::ScrollbarGrabHovered, [0.70, 0.72, 0.75, 1.0]);

    ui.child_window(tag).size([width, height]).always_vertical_scrollbar(true).build(body);
}

/// A searchable list of named IDs, for the widgets whose input would otherwise
/// be a number you have to know already.
///
/// The name is a convenience, not a constraint: the widgets that use this keep
/// their free-form ID field, so anything missing from the list is still
/// reachable by typing it.
#[derive(Deserialize)]
struct Entry {
    id: String,
    desc: String,
}

struct Row {
    label: String,
    /// Lowercased once at load, so filtering does not allocate per frame.
    search: String,
    id: u32,
}

pub(crate) struct IdList {
    tag: &'static str,
    rows: Vec<Row>,
    /// Indices into `rows` matching the current filter, rebuilt each frame.
    matches: Vec<usize>,
    filter: String,
    selected: Option<usize>,
}

impl IdList {
    /// `json` is a flat array of `{ "id": "...", "desc": "..." }`, IDs in
    /// decimal.
    pub(crate) fn new(tag: &'static str, json: &str) -> Self {
        let rows = match serde_json::from_str::<Vec<Entry>>(json) {
            Ok(entries) => entries
                .into_iter()
                .filter_map(|e| {
                    let id = e.id.trim().parse::<u32>().ok()?;
                    Some(Row { search: e.desc.to_lowercase(), label: e.desc, id })
                })
                .collect(),
            Err(e) => {
                // Not fatal: the widget still works, you just have to type IDs.
                hudhook::tracing::error!("Couldn't parse {tag} list: {e}");
                Vec::new()
            },
        };

        Self { tag, rows, matches: Vec::new(), filter: String::new(), selected: None }
    }

    /// Draws the search box, the count and the list. Returns the ID on the
    /// frame a row is picked, and `None` otherwise.
    pub(crate) fn render(&mut self, ui: &Ui, width: f32) -> Option<u32> {
        if self.rows.is_empty() {
            return None;
        }

        {
            let _tok = ui.push_item_width(width);
            ui.input_text(format!("##{}-filter", self.tag), &mut self.filter)
                .hint("search")
                .build();
        }

        let needle = self.filter.trim().to_lowercase();

        self.matches.clear();
        self.matches.extend(
            self.rows
                .iter()
                .enumerate()
                .filter(|(_, row)| needle.is_empty() || row.search.contains(&needle))
                .map(|(index, _)| index),
        );

        count_label(ui, self.matches.len(), self.rows.len());

        let rows = &self.rows;
        let matches = &self.matches;
        let selected = &mut self.selected;
        let mut picked = None;

        scrollable_list(ui, &format!("##{}-list", self.tag), width, matches.len(), || {
            if matches.is_empty() {
                ui.text_disabled("no matches");
                return;
            }

            for row in imgui::ListClipper::new(matches.len() as i32).begin(ui).iter() {
                let Some(&index) = matches.get(row as usize) else {
                    continue;
                };
                let Some(entry) = rows.get(index) else {
                    continue;
                };

                if ui.selectable_config(&entry.label).selected(*selected == Some(index)).build() {
                    *selected = Some(index);
                    picked = Some(entry.id);
                }
            }
        });

        picked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(json: &str) -> Vec<(String, u32)> {
        serde_json::from_str::<Vec<Entry>>(json)
            .unwrap()
            .into_iter()
            .map(|e| (e.desc, e.id.trim().parse::<u32>().unwrap()))
            .collect()
    }

    #[test]
    fn bundled_lists_parse() {
        let flags = rows(include_str!("event_flag_ids.json"));
        let bonfires = rows(include_str!("bonfire_ids.json"));

        assert!(flags.len() > 20, "only {} flags", flags.len());
        assert!(bonfires.len() > 50, "only {} bonfires", bonfires.len());
    }

    /// Spot checks against the souls-modding reference and Gadget's list. Main
    /// bosses use single digit flags; the rest are eight digits.
    #[test]
    fn known_ids_are_right() {
        let flags = rows(include_str!("event_flag_ids.json"));
        let find = |name: &str| {
            flags.iter().find(|(d, _)| d == name).unwrap_or_else(|| panic!("no {name}")).1
        };

        assert_eq!(find("Asylum Demon defeated"), 16);
        assert_eq!(find("Gwyn defeated"), 15);
        assert_eq!(find("Bell Gargoyles defeated"), 3);
        assert_eq!(find("Taurus Demon defeated"), 11010901);

        let bonfires = rows(include_str!("bonfire_ids.json"));
        assert!(bonfires.iter().any(|(d, id)| d.contains("Firelink Shrine") && *id == 1022960));
    }
}
