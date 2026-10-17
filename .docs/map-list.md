# Updating the map list

Run `npm run extract` when Valve adds Halloween maps. It rewrites `src/halloween-maps.json` and `public/maps/`; check the diff of `halloween-maps.json` before committing.

It needs TF2 installed on Windows, since it runs the game's `vpk.exe`, and finds it the same way the app does (`TF2_DIR` works here too).

Where the data comes from:

- Map list and casual menu placement: `items_game.txt`
- Names and modes: `tf_english.txt`
- ConTracker node ids: `proto_defs.vpd`
- Screenshots: each map's TF2 Wiki infobox, or the casual menu thumbnail from `tf2_textures_dir.vpk` when the wiki has none

Screenshots are saved as lossless WebP through `sharp`, about two thirds the size of a PNG. JPEGs from the wiki are kept as they are, since a lossless copy would keep their artifacts at about five times the size.
