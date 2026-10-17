# How the radar works

The radar needs two positions, yours and the gargoyle's, and gets both from TF2 itself.

## Your position

The launch options turn on RCON, TF2's own remote console, reachable from this PC only (`+sv_rcon_whitelist_address 127.0.0.1`) and protected by a generated password. Once per screen refresh the app sends `getpos` over RCON and reads your position and view direction from the reply.

## The gargoyle

The server sends every soul gargoyle to every player, so it is in the demo TF2 records. The app runs `record gargoyle_radar` and reads `tf\gargoyle_radar.dem` as TF2 writes it.

TF2 writes demos in bursts of about 700 KB, which on a quiet server is about once a minute. To make a new gargoyle show sooner, the app sends empty `alias` commands over RCON; TF2 records them into the demo, so the bursts come sooner. After each burst it adjusts how many it sends, so a burst lands every **Demo delay** seconds. At 5 s on a quiet server the demo grows by up to about 500 MB an hour; busy servers need less filler or none.

When TF2 closes, the app deletes `tf\gargoyle_radar.dem`.

## The overlay

The panel is a set of click-through, always-on-top windows drawn by the app over TF2's window. The panel's settings are saved in `radar.json` in the [data folder](settings.md#the-data-folder).

## Caution

RCON tools have been banned when they automated visible actions. The radar only sends `getpos`, `record` and `alias`, to TF2 on this PC, and nothing other players see.
