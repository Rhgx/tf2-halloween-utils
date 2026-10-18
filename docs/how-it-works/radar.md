# How the radar works

The [radar](../using/radar.md) needs two positions, yours and the gargoyle's, and gets both from TF2 itself.

## Your position

The launch options turn on RCON, TF2's own remote console, reachable from this PC only (`+sv_rcon_whitelist_address 127.0.0.1`) and protected by a generated password. Once per screen refresh the app sends `getpos` over RCON and reads your position and view direction from the reply. TF2 also prints each reply in its console, so the app turns on TF2's console filter for lines with `setang`; the replies over RCON are not affected.

The distance and height it shows change only once the real value is three quarters of a display step away, so your position's small jitter does not make them flicker between two numbers.

## The gargoyle

The server sends every soul gargoyle to every player, so it is in the demo TF2 records. The app runs `record gargoyle_radar` and reads `tf\gargoyle_radar.dem` as TF2 writes it.

TF2 writes demos in bursts of about 700 KB, which on a quiet server is about once a minute. To make a new gargoyle show sooner, the app sends empty `alias` commands over RCON; TF2 records them into the demo, so the bursts come sooner. After each burst it adjusts how many it sends, so a burst lands every **Demo delay** seconds. At 5 s on a quiet server the demo grows by up to about 500 MB an hour; busy servers need less filler or none.

The demo also dates the gargoyle: the tick it first appears on is when it spawned, since the server sends it to every player from the start. It despawns `tf_halloween_gift_lifetime` seconds later, 240 on any server without `sv_cheats`.

When TF2 closes, the app deletes `tf\gargoyle_radar.dem`.

## Caution

RCON tools have been banned when they automated visible actions. The radar only sends `getpos`, `record`, `alias` and the console filter settings, to TF2 on this PC, and nothing other players see.
