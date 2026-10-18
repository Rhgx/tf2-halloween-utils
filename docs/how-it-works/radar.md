# How the radar works

The [radar](../using/radar.md) needs two positions, yours and the gargoyle's, and gets both from TF2 itself.

## Your position

The launch options turn on RCON, TF2's own remote console, and protect it with a generated password. TF2 listens for RCON on all network interfaces, so other devices on your local network can reach the port; a router normally keeps it off the internet. Source also bans an address after repeated wrong passwords. `+sv_rcon_whitelist_address 127.0.0.1` only keeps this PC from ever being banned. Once per screen refresh the app sends `getpos` over RCON and reads your position and view direction from the reply. TF2 also prints each reply in its console, so the app turns on TF2's console filter for lines with `setang`; the replies over RCON are not affected.

The distance and height it shows change only once the real value is three quarters of a display step away, so your position's small jitter does not make them flicker between two numbers.

## The gargoyle

The server sends every soul gargoyle to every player, so it is in the demo TF2 records. The app runs `record gargoyle_radar` and reads `tf\gargoyle_radar.dem` as TF2 writes it.

TF2 writes demos in bursts of about 700 KB, which on a quiet server is about once a minute. To make a new gargoyle show sooner, the app sends empty `alias` commands over RCON; TF2 records them into the demo, so the bursts come sooner. After each burst it adjusts how many it sends, so a burst lands every **Demo delay** seconds. At 5 s on a quiet server TF2 writes up to about 500 MB an hour; busy servers need less filler or none.

The demo also dates the gargoyle: the tick it first appears on is when it spawned, since the server sends it to every player from the start. It despawns `tf_halloween_gift_lifetime` seconds later, 240 on any server without `sv_cheats`.

So the file does not keep growing on a long map, the app starts the recording over (`stop`, then `record`, which overwrites it) once it passes 100 MB and no gargoyle is being followed. It never touches a demo you record yourself. When TF2 closes, the app deletes `tf\gargoyle_radar.dem`.

## Caution

RCON tools have been banned when they automated visible actions. The radar only sends `getpos`, `record`, `stop` (to start its own demo over), `alias` and the console filter settings, to TF2 on this PC, and nothing other players see.
