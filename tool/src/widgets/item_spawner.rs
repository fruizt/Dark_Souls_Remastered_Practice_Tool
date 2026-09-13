use libdsr::funcs::ItemSpawn;
use practice_tool_core::crossbeam_channel::Sender;
use practice_tool_core::key::Key;
use practice_tool_core::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};
use serde::Deserialize;

/// The bundled item tree. Branches carry a name and children, leaves an ID and
/// a description; the ID's top nibble is the item's category.
#[derive(Deserialize)]
#[serde(untagged)]
enum Node {
    Branch { node: String, children: Vec<Node> },
    Leaf { id: String, desc: String },
}

struct Item {
    /// "Items / Consumables / Estus Flask", so a search matches the group too.
    label: String,
    /// Lowercased once at load, so filtering does not allocate per frame.
    search: String,
    category: u32,
    item_id: u32,
}

fn flatten(node: &Node, path: &str, out: &mut Vec<Item>) {
    match node {
        Node::Branch { node, children } => {
            let path = if path.is_empty() { node.clone() } else { format!("{path} / {node}") };
            for child in children {
                flatten(child, &path, out);
            }
        },
        Node::Leaf { id, desc } => {
            let Ok(raw) = u32::from_str_radix(id, 16) else {
                return;
            };
            let label = if path.is_empty() { desc.clone() } else { format!("{path} / {desc}") };
            out.push(Item {
                search: label.to_lowercase(),
                label,
                category: raw & 0xF000_0000,
                item_id: raw & 0x0FFF_FFFF,
            });
        },
    }
}

const SPAWNER_TAG: &str = "##item-spawner";

struct ItemSpawner {
    spawn: ItemSpawn,
    items: Vec<Item>,
    /// Indices into `items` matching the current filter, rebuilt each frame.
    matches: Vec<usize>,
    filter: String,
    quantity: String,
    selected: Option<usize>,
    label_open: String,
    key_open: Option<Key>,
    key_close: Key,
    logs: Vec<String>,
}

impl ItemSpawner {
    fn new(spawn: ItemSpawn, key_open: Option<Key>, key_close: Key) -> Self {
        let items = match serde_json::from_str::<Vec<Node>>(include_str!("item_ids.json")) {
            Ok(tree) => {
                let mut items = Vec::new();
                for node in &tree {
                    flatten(node, "", &mut items);
                }
                items
            },
            Err(e) => {
                // Not fatal: the panel still opens, it is just empty.
                hudhook::tracing::error!("Couldn't parse item_ids.json: {e}");
                Vec::new()
            },
        };

        let label_open = match key_open {
            Some(key) => format!("Item spawner ({key})"),
            None => String::from("Item spawner"),
        };

        Self {
            spawn,
            items,
            matches: Vec::new(),
            filter: String::new(),
            quantity: String::from("1"),
            selected: None,
            label_open,
            key_open,
            key_close,
            logs: Vec::new(),
        }
    }

    fn spawn_selected(&mut self) {
        let Some(item) = self.selected.and_then(|i| self.items.get(i)) else {
            self.logs.push(String::from("No item selected"));
            return;
        };

        let quantity = self.quantity.trim().parse::<u32>().unwrap_or(1).max(1);

        if self.spawn.spawn(item.category, item.item_id, quantity) {
            self.logs.push(format!("Spawned {quantity}x {}", item.label));
        } else {
            self.logs.push(String::from("Can't spawn items right now"));
        }
    }
}

impl Widget for ItemSpawner {
    fn render(&mut self, ui: &imgui::Ui) {
        let scale = scaling_factor(ui);
        let button_width = BUTTON_WIDTH * scale;

        if ui.button_with_size(&self.label_open, [button_width, BUTTON_HEIGHT]) {
            ui.open_popup(SPAWNER_TAG);
        }

        if let Some(_token) = ui
            .modal_popup_config(SPAWNER_TAG)
            .resizable(false)
            .movable(false)
            .title_bar(false)
            .scroll_bar(false)
            .begin_popup()
        {
            {
                let _tok = ui.push_item_width(button_width);
                ui.input_text("##item_filter", &mut self.filter).hint("search items").build();
            }

            let needle = self.filter.trim().to_lowercase();

            self.matches.clear();
            self.matches.extend(
                self.items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| needle.is_empty() || item.search.contains(&needle))
                    .map(|(index, _)| index),
            );

            let matches = &self.matches;
            let items = &self.items;
            let selected = &mut self.selected;

            ui.child_window("##item-list").size([button_width, 200. * scale]).build(|| {
                // Every match stays reachable by scrolling. The clipper is what
                // keeps that affordable: it draws only the rows actually on screen,
                // so the list can be as long as it likes.
                for row in imgui::ListClipper::new(matches.len() as i32).begin(ui).iter() {
                    let Some(&index) = matches.get(row as usize) else {
                        continue;
                    };
                    let Some(item) = items.get(index) else {
                        continue;
                    };

                    if ui.selectable_config(&item.label).selected(*selected == Some(index)).build()
                    {
                        *selected = Some(index);
                    }
                }
            });

            {
                let _tok = ui.push_item_width(button_width * 80. / 240.);
                ui.input_text("##item_quantity", &mut self.quantity).hint("qty").build();
            }
            ui.same_line();

            let _token = ui.begin_disabled(!self.spawn.is_ready() || self.selected.is_none());
            if ui.button_with_size("Spawn", [button_width * 156. / 240., BUTTON_HEIGHT]) {
                self.spawn_selected();
            }
            drop(_token);

            ui.separator();

            if ui.button_with_size("Close", [button_width, BUTTON_HEIGHT])
                || (!ui.is_any_item_active() && self.key_close.is_pressed(ui))
            {
                ui.close_current_popup();
            }
        }
    }

    fn interact(&mut self, ui: &imgui::Ui) {
        if self.key_open.map(|k| k.is_pressed(ui)).unwrap_or(false) {
            self.spawn_selected();
        }
    }

    fn action(&mut self) {
        self.spawn_selected();
    }

    fn log(&mut self, tx: Sender<String>) {
        for log in self.logs.drain(..) {
            tx.send(log).ok();
        }
    }
}

pub(crate) fn item_spawner(
    spawn: ItemSpawn,
    key_open: Option<Key>,
    key_close: Key,
) -> Box<dyn Widget> {
    Box::new(ItemSpawner::new(spawn, key_open, key_close))
}
