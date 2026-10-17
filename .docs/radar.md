# Gargoyle radar

A panel over TF2 that points to your Halloween soul gargoyle: an arrow for the direction, the distance, and a pin that shows whether it is above or below you. Within 2.5 m it shows **HERE**. Clicks pass through it to the game.

It needs TF2 in **Windowed (no border)** mode. Nothing in the game files changes and TF2's memory is not read; [how the radar works](how-the-radar-works.md) explains what it does instead.

<img src="screenshot-radar.webp" alt="The Gargoyle radar tab: status, overlay preview and settings" width="720">

## Setup

1. Open the **Gargoyle radar** tab and copy the launch options line it shows.
2. In Steam, right-click the game, open Properties, and paste the line into Launch Options.
3. Restart TF2.

The setup steps fold away once the radar connects. The line contains a password made for you, so do not post it. It also includes `-condebug`, which the Contracts tab uses to follow your matches.

## Using it

Turn the radar on with the switch in its tab. The tab shows whether it is connected and whether your gargoyle has been found, with a preview of the panel.

- **Opacity** and **Scale** change how the panel looks. **Reset layout** puts it back.
- **Demo delay** (default 5 s) is the longest wait before a new gargoyle shows. Lower is faster, but the demo file TF2 records grows faster.
- **RCON port**: change it only if the port is taken, then paste the new launch options line into Steam.
- **Follow any gargoyle** is for testing on a local server: `sv_cheats 1`, then `ent_create tf_halloween_gift_pickup`.

## Moving the panel

**Move radar**, or the edit hotkey in game, starts edit mode. Press Esc in TF2 to free the mouse, then drag the panel, or drag its corner to resize it. The arrow keys nudge it (Shift for 10 px), `+` and `-` resize, PgUp and PgDn change the opacity, and Home resets. Press the edit hotkey again to save.

## Hotkeys

- **Pause**: turns the overlay on and off.
- **Scroll Lock**: starts edit mode, and saves it.

To change them, open **Settings**, click a key field under Radar hotkeys and press a key or Mouse 4/5. Esc cancels. The keys edit mode uses cannot be hotkeys.

## If it does not connect

- **Cannot reach TF2's RCON**: the launch options are missing, or TF2 was not restarted after adding them.
- **TF2 rejected the RCON password**: the line in Steam is not the one in the tab. Copy it again.
