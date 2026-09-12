//! Story flags, addressed by the IDs the game's own event scripts use.
//!
//! The flag storage is one large bitfield reached through two hops from
//! `EventFlagMan`. An eight digit flag ID encodes where in that bitfield the
//! flag sits: `G AAA S NNN`, where `G` selects a coarse group, `AAA` an area,
//! `S` a section within the area, and `NNN` the flag itself. Anything outside
//! the tables below is not a flag the game knows about.

use crate::memedit::PointerChain;

/// Byte offset of each flag group within the bitfield.
const GROUPS: [(u32, usize); 5] =
    [(0, 0x00000), (1, 0x00500), (5, 0x05F00), (6, 0x0B900), (7, 0x11300)];

/// Area code to its index. Areas are 0x500 bytes apart.
const AREAS: [(u32, usize); 18] = [
    (0, 0),
    (100, 1),
    (101, 2),
    (102, 3),
    (110, 4),
    (120, 5),
    (121, 6),
    (130, 7),
    (131, 8),
    (132, 9),
    (140, 10),
    (141, 11),
    (150, 12),
    (151, 13),
    (160, 14),
    (170, 15),
    (180, 16),
    (181, 17),
];

#[derive(Debug, Clone)]
pub struct EventFlags {
    event_flag_man: usize,
}

impl EventFlags {
    pub fn new(event_flag_man: usize) -> Self {
        Self { event_flag_man }
    }

    /// The word holding `id`, and the mask selecting it within that word.
    ///
    /// `None` when the ID does not decode to a real flag, which is the common
    /// case for a typo in the UI.
    pub fn resolve(&self, id: u32) -> Option<(PointerChain<u32>, u32)> {
        if id > 99_999_999 {
            return None;
        }

        let group = id / 10_000_000;
        let area = (id / 10_000) % 1_000;
        let section = (id / 1_000) % 10;
        let number = id % 1_000;

        let group_offset = GROUPS.iter().find(|(g, _)| *g == group).map(|(_, o)| *o)?;
        let area_index = AREAS.iter().find(|(a, _)| *a == area).map(|(_, i)| *i)?;

        let offset = group_offset
            + area_index * 0x500
            + section as usize * 128
            + (number - number % 32) as usize / 8;

        // Two hops to the bitfield, then the flag's word.
        let chain = PointerChain::new(&[self.event_flag_man, 0x0, offset]);
        let mask = 0x8000_0000u32 >> (number % 32);

        Some((chain, mask))
    }

    /// `None` if the ID is unknown or the chain does not resolve — no world
    /// loaded, usually.
    pub fn read(&self, id: u32) -> Option<bool> {
        let (chain, mask) = self.resolve(id)?;
        chain.read().map(|word| word & mask != 0)
    }

    pub fn write(&self, id: u32, value: bool) -> Option<()> {
        let (chain, mask) = self.resolve(id)?;
        let word = chain.read()?;
        chain.write(if value { word | mask } else { word & !mask })
    }
}
