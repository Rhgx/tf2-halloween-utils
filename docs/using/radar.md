# Gargoyle radar

An [overlay](overlay.md) widget that points to your Halloween soul gargoyle: an arrow for the direction, the distance, a pin that shows whether it is above or below you, and how long until it despawns (4 minutes after it spawns; orange for the last 30 seconds). Within 2.5 m it shows **HERE**. The despawn time needs the radar to have seen the gargoyle spawn, so it is missing for one that was already there when the radar started recording.

[How the radar works](../how-it-works/radar.md) explains where it gets both positions.

## Setup

It reads TF2 through RCON, which needs launch options once:

1. In the **Overlay** tab, select **Gargoyle radar** and copy the launch options line it shows.
2. In Steam, right-click the game, open Properties, and paste the line into Launch Options.
3. Restart TF2.

The setup steps fold away once the radar connects. The line contains a password made for you, so do not post it. It also includes `-condebug`, which the Contracts tab uses to follow your matches.

## Settings

- **Demo delay** (default 5 s) is the longest wait before a new gargoyle shows. Lower is faster, but the demo file TF2 records grows faster.
- **Units**: metres (default), feet or Hammer units (TF2's own, 0.75 inch) for the distance and the height.
- **Despawn countdown** (on by default) shows the time left under the distance; turn it off for a two-line panel.
- **RCON port**: change it only if the port is taken, then paste the new launch options line into Steam.
- **Follow any gargoyle** is for testing on a local server: `sv_cheats 1`, then `ent_create tf_halloween_gift_pickup`.

## If it does not connect

- **Cannot reach TF2's RCON**: the launch options are missing, or TF2 was not restarted after adding them.
- **TF2 rejected the RCON password**: the line in Steam is not the one in the tab. Copy it again.
