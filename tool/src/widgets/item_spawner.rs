use libdsr::funcs::ItemSpawn;
use practice_tool_core::crossbeam_channel::Sender;
use practice_tool_core::key::Key;
use practice_tool_core::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};
use serde::Deserialize;

/// How an item can be upgraded, which decides what the panel offers and how the
/// final ID is composed.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Upgrade {
    /// Not upgradeable at all.
    None,
    /// A fixed track, no infusions: `id + level`.
    Levels(u32),
    /// Infusion path plus level, both folded into the ID.
    Infusable { restricted_paths: bool },
    /// Pyromancy flames count in hundreds: `id + level * 100`.
    Pyro(u32),
}

impl Upgrade {
    fn from_code(code: u8) -> Self {
        match code {
            1 => Upgrade::Levels(5),  // unique weapons
            2 => Upgrade::Levels(10), // armour
            3 => Upgrade::Infusable { restricted_paths: false },
            4 => Upgrade::Infusable { restricted_paths: true },
            5 => Upgrade::Pyro(15),
            6 => Upgrade::Pyro(5),
            _ => Upgrade::None,
        }
    }
}

/// Name, the value folded into the ID, the highest level, and whether the path
/// is one only fully infusable weapons can take.
const INFUSIONS: [(&str, u32, u32, bool); 10] = [
    ("Normal", 0, 15, false),
    ("Crystal", 100, 5, false),
    ("Lightning", 200, 5, false),
    ("Raw", 300, 5, true),
    ("Magic", 400, 10, false),
    ("Enchanted", 500, 5, true),
    ("Divine", 600, 10, false),
    ("Occult", 700, 5, true),
    ("Fire", 800, 10, false),
    ("Chaos", 900, 5, true),
];

/// The bundled item tree. Branches carry a name and children; leaves carry the
/// category in the ID's top nibble, a stack limit, and an upgrade code.
#[derive(Deserialize)]
#[serde(untagged)]
enum Node {
    Branch {
        node: String,
        children: Vec<Node>,
    },
    Leaf {
        id: String,
        desc: String,
        #[serde(default = "one")]
        limit: u32,
        #[serde(default)]
        upgrade: u8,
    },
}

fn one() -> u32 {
    1
}

struct Item {
    /// "Rings / Havel's Ring", so a search matches the category too.
    label: String,
    /// Lowercased once at load, so filtering does not allocate per frame.
    search: String,
    category: u32,
    item_id: u32,
    stack_limit: u32,
    upgrade: Upgrade,
}

fn flatten(node: &Node, path: &str, out: &mut Vec<Item>) {
    match node {
        Node::Branch { node, children } => {
            let path = if path.is_empty() { node.clone() } else { format!("{path} / {node}") };
            for child in children {
                flatten(child, &path, out);
            }
        },
        Node::Leaf { id, desc, limit, upgrade } => {
            let Ok(raw) = u32::from_str_radix(id, 16) else {
                return;
            };
            let label = if path.is_empty() { desc.clone() } else { format!("{path} / {desc}") };
            out.push(Item {
                search: label.to_lowercase(),
                label,
                category: raw & 0xF000_0000,
                item_id: raw & 0x0FFF_FFFF,
                stack_limit: (*limit).max(1),
                upgrade: Upgrade::from_code(*upgrade),
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
    quantity: i32,
    level: i32,
    infusion: usize,
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
            quantity: 1,
            level: 0,
            infusion: 0,
            selected: None,
            label_open,
            key_open,
            key_close,
            logs: Vec::new(),
        }
    }

    /// Infusion paths the item can take, as (label, value, highest level).
    fn paths(item: &Item) -> Vec<(&'static str, u32, u32)> {
        match item.upgrade {
            Upgrade::Infusable { restricted_paths } => INFUSIONS
                .iter()
                .filter(|(_, _, _, restricted)| !(restricted_paths && *restricted))
                .map(|(name, value, max, _)| (*name, *value, *max))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn max_level(item: &Item, infusion: usize) -> u32 {
        match item.upgrade {
            Upgrade::None => 0,
            Upgrade::Levels(max) | Upgrade::Pyro(max) => max,
            Upgrade::Infusable { .. } => {
                Self::paths(item).get(infusion).map(|(_, _, max)| *max).unwrap_or(0)
            },
        }
    }

    /// Base ID plus level plus infusion, the way the game encodes an upgrade.
    fn composed_id(item: &Item, level: u32, infusion: usize) -> u32 {
        let level = level.min(Self::max_level(item, infusion));

        let mut id = match item.upgrade {
            Upgrade::Pyro(_) => item.item_id + level * 100,
            _ => item.item_id + level,
        };

        if let Upgrade::Infusable { .. } = item.upgrade {
            if let Some((_, value, _)) = Self::paths(item).get(infusion) {
                id += value;
            }
        }

        id
    }

    fn spawn_selected(&mut self) {
        let Some(item) = self.selected.and_then(|i| self.items.get(i)) else {
            self.logs.push(String::from("No item selected"));
            return;
        };

        let quantity = (self.quantity.max(1) as u32).min(item.stack_limit);
        let id = Self::composed_id(item, self.level.max(0) as u32, self.infusion);
        let category = item.category;
        let label = item.label.clone();

        if self.spawn.spawn(category, id, quantity) {
            self.logs.push(format!("Spawned {quantity}x {label} (id {id})"));
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
            let mut changed = false;

            ui.child_window("##item-list").size([button_width, 200. * scale]).build(|| {
                // Every match stays reachable by scrolling. The clipper is what
                // keeps that affordable: it draws only the rows actually on
                // screen, so the list can be as long as it likes.
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
                        changed = true;
                    }
                }
            });

            // A new selection starts from a plain, un-upgraded item rather than
            // inheriting whatever the last one was set to.
            if changed {
                self.level = 0;
                self.infusion = 0;
                self.quantity = 1;
            }

            if let Some(index) = self.selected {
                let (paths, max_level, stack_limit) = {
                    let item = &self.items[index];
                    (Self::paths(item), Self::max_level(item, self.infusion), item.stack_limit)
                };

                if !paths.is_empty() {
                    let names: Vec<&str> = paths.iter().map(|(name, ..)| *name).collect();
                    let _tok = ui.push_item_width(button_width);
                    if ui.combo_simple_string("##item_infusion", &mut self.infusion, &names) {
                        // Paths cap out at different levels, so an infusion change
                        // can strand the slider above its new maximum.
                        self.level = 0;
                    }
                }

                if max_level > 0 {
                    let _tok = ui.push_item_width(button_width);
                    ui.slider("##item_level", 0, max_level as i32, &mut self.level);
                }

                if stack_limit > 1 {
                    let _tok = ui.push_item_width(button_width);
                    ui.slider("##item_quantity", 1, stack_limit as i32, &mut self.quantity);
                } else {
                    self.quantity = 1;
                }
            }

            let _token = ui.begin_disabled(!self.spawn.is_ready() || self.selected.is_none());
            if ui.button_with_size("Spawn", [button_width, BUTTON_HEIGHT]) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn load() -> Vec<Item> {
        let tree: Vec<Node> = serde_json::from_str(include_str!("item_ids.json")).unwrap();
        let mut items = Vec::new();
        for node in &tree {
            flatten(node, "", &mut items);
        }
        items
    }

    fn find<'a>(items: &'a [Item], label: &str) -> &'a Item {
        items.iter().find(|i| i.label == label).unwrap_or_else(|| panic!("no item {label:?}"))
    }

    #[test]
    fn item_data_parses() {
        let items = load();
        assert!(items.len() > 800, "only {} items parsed", items.len());
    }

    /// The IDs that sent a nameless entry into the inventory: these are DSR's,
    /// not DS3's, where Havel's Ring is 20020.
    #[test]
    fn ids_are_dark_souls_remastered() {
        let items = load();

        let havels = find(&items, "Rings / Havel's Ring");
        assert_eq!(havels.category, 0x2000_0000);
        assert_eq!(havels.item_id, 100);

        let blossom = find(&items, "Consumables / Green Blossom");
        assert_eq!(blossom.category, 0x4000_0000);
        assert_eq!(blossom.item_id, 260);
        assert_eq!(blossom.stack_limit, 99);
    }

    #[test]
    fn upgrades_fold_into_the_id() {
        let items = load();

        // Rings do not upgrade, so a level cannot leak into the ID.
        let havels = find(&items, "Rings / Havel's Ring");
        assert_eq!(ItemSpawner::composed_id(havels, 5, 0), 100);

        // Infusable weapon: normal +5 is base+5, and lightning (+200) at +5 is
        // base+205. Path order follows INFUSIONS.
        let dagger = find(&items, "Melee Weapons / Dagger");
        assert!(matches!(dagger.upgrade, Upgrade::Infusable { .. }));
        assert_eq!(ItemSpawner::composed_id(dagger, 5, 0), dagger.item_id + 5);
        let lightning = ItemSpawner::paths(dagger).iter().position(|(n, ..)| *n == "Lightning");
        assert_eq!(ItemSpawner::composed_id(dagger, 5, lightning.unwrap()), dagger.item_id + 205);

        // Levels are clamped to what the path allows: lightning stops at +5.
        assert_eq!(ItemSpawner::composed_id(dagger, 99, lightning.unwrap()), dagger.item_id + 205);
    }

    #[test]
    fn restricted_weapons_offer_fewer_paths() {
        let items = load();
        let infusable = items
            .iter()
            .find(|i| matches!(i.upgrade, Upgrade::Infusable { restricted_paths: false }))
            .unwrap();
        let restricted = items
            .iter()
            .find(|i| matches!(i.upgrade, Upgrade::Infusable { restricted_paths: true }))
            .unwrap();

        assert_eq!(ItemSpawner::paths(infusable).len(), 10);
        assert_eq!(ItemSpawner::paths(restricted).len(), 6);
    }
}
