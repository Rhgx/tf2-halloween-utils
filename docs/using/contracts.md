# Contracts

The Contracts tab lists all 65 Halloween maps, grouped the way TF2's casual menu groups them, with the ConTracker folder each contract sits in.

## Sign in once

Click **Sign in with Steam** and scan the QR code with the Steam mobile app. No password is typed anywhere. The app then syncs which contracts you have completed. See [Steam sync](../how-it-works/steam-sync.md) for what is stored.

## Queue the maps you need

Click maps to queue them, or **Queue all to do**. Then open the casual map selection in TF2 and click **Restore**: TF2 loads the app's queue. When you quit the app, your original selection is put back (you can turn that off in [Settings](settings.md)).

## Follow your matches

Add `-condebug` to TF2's launch options in Steam (right-click the game, Properties, Launch Options). The app then shows the map you are on and its ConTracker folder, and asks whether you finished the contract when a Halloween match ends. Without `-condebug` everything else still works.

## Keep it current

Sync is blocked while TF2 runs, because Steam allows one game session per account. After TF2 closes, the app syncs by itself. In the meantime, mark a contract done by hand on its tile.

**Launch TF2** syncs first, then starts the game.
