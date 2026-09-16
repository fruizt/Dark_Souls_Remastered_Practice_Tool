Dark Souls: Remastered Practice Tool
====================================

An in-game practice overlay: freeze death, warp to any bonfire, spawn items,
edit stats, flip event flags, and read IGT and position live.


HOW TO RUN
----------------------------------------------------------------------------

Keep all three files together in the same folder. Anywhere will do.

  dark_souls_remastered_tool.exe            <- run this
  dark_souls_remastered_tool_binaries.dll
  dark_souls_remastered_tool.toml           <- without this the overlay is empty

  1. Disconnect from the network, or launch the game offline. See below.
  2. Start Dark Souls: Remastered and load into a save.
  3. Double-click `dark_souls_remastered_tool.exe`. It finds the game, injects,
     and exits.
  4. Alt-tab back into the game. The tool's name and indicators appear in the
     top-left corner.
  5. Press  0           to open and close the overlay.
     Press  RShift + 0  to hide it completely (hotkeys keep working).

If injection fails, run the .exe as Administrator.


READ THIS FIRST
----------------------------------------------------------------------------

PLAY OFFLINE. This tool writes to the game's memory. Using it while connected
can get your account flagged or soft-banned, and it is unfair to other players.

USE A SAVE YOU DO NOT MIND LOSING. A mis-aimed stat write or position warp can
corrupt a character.

PATCH 1.03.1 ONLY. On any other build the tool says so in the Game Version
indicator and disables every write, rather than acting on addresses that mean
nothing there.


ANTIVIRUS
----------------------------------------------------------------------------

Windows Defender may flag `dark_souls_remastered_tool.exe` as
Trojan:Win32/Wacatac.B!ml. This is a false positive and has been reported to
Microsoft.

The injector loads the overlay into the game using the standard Windows DLL
injection calls. A lot of malware does the same thing, and the heuristic that
looks at this cannot tell them apart. The code is about sixty lines, in
tool/src/inject.rs in the repository, and you are welcome to read it before you
trust it. The overlay DLL itself is not flagged - only the injector.


CONFIGURATION
----------------------------------------------------------------------------

Every hotkey and widget comes from `dark_souls_remastered_tool.toml`, which
must sit next to the DLL. It is read once at injection time, so edit it and
re-inject to pick up changes. The full hotkey list and config reference are in
the online README below.

Logs are written to `dark_souls_remastered_tool.log`, next to the DLL.


HELP
----------------------------------------------------------------------------

Full documentation, hotkey table, and troubleshooting:
  https://github.com/fruizt/Dark_Souls_Remastered_Practice_Tool#readme

Bugs and questions:
  https://github.com/fruizt/Dark_Souls_Remastered_Practice_Tool/issues


Unofficial fan project. Not affiliated with FromSoftware or Bandai Namco.
For single-player, offline practice only.
Licensed under AGPL-3.0.
