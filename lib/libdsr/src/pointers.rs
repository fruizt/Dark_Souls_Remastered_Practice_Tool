use std::fmt::Display;

use log::debug;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;

use crate::codegen::base_addresses::Version;
use crate::event_flags::EventFlags;
use crate::funcs::{BonfireWarp, ItemSpawn};
use crate::memedit::{Bitflag, *};
use crate::prelude::base_addresses::BaseAddresses;
use crate::version::VERSION;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct CharacterStats {
    pub vitality: i32,
    pub unk1: i32,
    pub attunement: i32,
    pub unk2: i32,
    pub endurance: i32,
    pub unk3: i32,
    pub strength: i32,
    pub unk4: i32,
    pub dexterity: i32,
    pub unk5: i32,
    pub intelligence: i32,
    pub unk6: i32,
    pub faith: i32,
    pub unk7: i32,
    pub unk8: i32,
    pub unk9: i32,
    pub unk10: i32,
    pub humanity: i32,
    pub resistance: i32,
    pub unk11: i32,
    pub level: i32,
    pub souls: i32,
}

impl Display for CharacterStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "CharacterStats {{ }}")
    }
}

#[derive(Debug)]
pub struct PointerChains {
    pub all_no_damage: Bitflag<u8>,
    pub no_death: Bitflag<u8>,
    pub inf_stamina: Bitflag<u8>,
    pub inf_consumables: Bitflag<u8>,
    pub no_damage: Bitflag<u8>,
    pub gravity: Bitflag<u8>,
    pub collision: Bitflag<u8>,
    pub speed: PointerChain<f32>,
    pub character_stats: PointerChain<CharacterStats>,
    pub souls: PointerChain<u32>,
    pub cursor_show: Bitflag<u8>,
    pub no_hit: Bitflag<u8>,
    pub igt: PointerChain<u32>,
    pub bonfire_warp_menu: Bitflag<u8>,
    pub position: (PointerChain<f32>, PointerChain<[f32; 3]>),
    pub deathcam: Bitflag<u8>,
    pub quitout: PointerChain<i32>,
    pub event_flags: EventFlags,
    /// The bonfire you last rested at, and the game's own travel routine that
    /// sends you to it.
    pub last_bonfire: PointerChain<u32>,
    pub bonfire_warp: BonfireWarp,
    pub item_spawn: ItemSpawn,

    // The rest of the ChrDbg block. Every entry is a whole-byte boolean, so each of
    // these is masked with `0b1` rather than a real bit.
    pub player_no_dead: Bitflag<u8>,
    pub player_exterminate: Bitflag<u8>,
    pub all_no_stamina: Bitflag<u8>,
    pub all_no_mp: Bitflag<u8>,
    pub all_no_arrow: Bitflag<u8>,
    pub all_no_magic_qty: Bitflag<u8>,
    pub player_hide: Bitflag<u8>,
    pub player_silence: Bitflag<u8>,
    pub all_no_dead: Bitflag<u8>,
    pub all_no_hit: Bitflag<u8>,
    pub all_no_attack: Bitflag<u8>,
    pub all_no_move: Bitflag<u8>,
    pub ai_disable: Bitflag<u8>,

    // Render group mask: five consecutive bytes in `.data`, one per draw group.
    pub rend_map: Bitflag<u8>,
    pub rend_obj: Bitflag<u8>,
    pub rend_chr: Bitflag<u8>,
    pub rend_sfx: Bitflag<u8>,
    pub rend_cutscene: Bitflag<u8>,
}

impl From<BaseAddresses> for PointerChains {
    fn from(value: BaseAddresses) -> Self {
        debug!("{:#?}", value);
        let BaseAddresses {
            game_data_man,
            world_chr_man,
            menu_man,
            chr_dbg,
            group_mask,
            event_flag_man,
            chr_class_warp,
            bonfire_warp_fn,
            item_get_fn,
            ..
        } = value;

        // Indices into the ChrDbg flag block. Each entry is a whole-byte boolean, so
        // the masks below are 0b1 rather than real bitmasks.
        let off_all_no_damage = 0x9;
        let offs_igt = match *VERSION {
            Version::V1_03_1 => 0xa4,
        };

        PointerChains {
            all_no_damage: bitflag!(0b1; chr_dbg + off_all_no_damage as usize),
            no_death: bitflag!(0b100000; world_chr_man, 0x68, 0x524),
            inf_stamina: bitflag!(0b100; world_chr_man, 0x68, 0x525),
            inf_consumables: bitflag!(0b1; world_chr_man, 0x68, 0x527),
            // ChrFlags1 is at 0x2A4 on this build (0x284 plus the 0x20 the struct
            // gained in 1.03), so NoGravity's 0x4000 is bit 6 of the byte at 0x2A5.
            // The 0x245 this used to read is not a flag at all: nothing in the binary
            // tests a single bit anywhere in 0x244..0x247, which is why the toggle
            // appeared to do nothing.
            gravity: bitflag!(0b1000000; world_chr_man, 0x68, 0x2a5),
            collision: bitflag!(0b1000; world_chr_man, 0x68,0x68, 0x104),
            speed: pointer_chain!(world_chr_man, 0x68, 0x68, 0x18, 0xa8),
            character_stats: pointer_chain!(game_data_man, 0x10, 0x40),
            souls: pointer_chain!(game_data_man, 0x10, 0x94),
            cursor_show: bitflag!(0b1; menu_man as _, 0xa8),
            no_damage: bitflag!(0b100000; world_chr_man, 0x68, 0x524),
            no_hit: bitflag!(0b1; world_chr_man, 0x80, 0x18, 0x1c0),
            igt: pointer_chain!(game_data_man as _, offs_igt),
            bonfire_warp_menu: bitflag!(0b1; menu_man, 0xc0),
            position: (
                pointer_chain!(world_chr_man, 0x68, 0x68, 0x28, 0x4), // angle
                pointer_chain!(world_chr_man, 0x68, 0x68, 0x28, 0x10), // position
            ),
            deathcam: bitflag!(0b1; world_chr_man, 0x70),
            // Menu kick. Writing 2 here is what quits out to the main menu.
            quitout: pointer_chain!(menu_man, 0x24c),
            event_flags: EventFlags::new(event_flag_man),
            // 0xB24 on 1.01.x; the warp block gained 0x10 in 1.01.2.
            last_bonfire: pointer_chain!(chr_class_warp, 0xb34),
            bonfire_warp: BonfireWarp::new(game_data_man, bonfire_warp_fn),
            item_spawn: ItemSpawn::new(game_data_man, item_get_fn),

            player_no_dead: bitflag!(0b1; chr_dbg + 0x0),
            player_exterminate: bitflag!(0b1; chr_dbg + 0x1),
            all_no_stamina: bitflag!(0b1; chr_dbg + 0x2),
            all_no_mp: bitflag!(0b1; chr_dbg + 0x3),
            all_no_arrow: bitflag!(0b1; chr_dbg + 0x4),
            all_no_magic_qty: bitflag!(0b1; chr_dbg + 0x5),
            player_hide: bitflag!(0b1; chr_dbg + 0x6),
            player_silence: bitflag!(0b1; chr_dbg + 0x7),
            all_no_dead: bitflag!(0b1; chr_dbg + 0x8),
            all_no_hit: bitflag!(0b1; chr_dbg + 0xa),
            all_no_attack: bitflag!(0b1; chr_dbg + 0xb),
            all_no_move: bitflag!(0b1; chr_dbg + 0xc),
            ai_disable: bitflag!(0b1; chr_dbg + 0xd),

            rend_map: bitflag!(0b1; group_mask + 0x0),
            rend_obj: bitflag!(0b1; group_mask + 0x1),
            rend_chr: bitflag!(0b1; group_mask + 0x2),
            rend_sfx: bitflag!(0b1; group_mask + 0x3),
            rend_cutscene: bitflag!(0b1; group_mask + 0x4),
        }
    }
}

impl Default for PointerChains {
    fn default() -> Self {
        Self::new()
    }
}

impl PointerChains {
    pub fn new() -> Self {
        let base_module_address = unsafe { GetModuleHandleA(None) }.unwrap().0 as usize;
        let base_addresses = BaseAddresses::from(*crate::version::VERSION)
            .with_module_base_addr(base_module_address);

        base_addresses.into()
    }
}
