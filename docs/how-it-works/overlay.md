# How the overlay works

## The windows

Each widget is its own click-through, always-on-top window, drawn by the app over TF2's window in TF2's HUD style and fonts. The widgets show only while TF2 is the window in front. Their layout and switches are saved in `radar.json` in the [data folder](../using/settings.md#the-data-folder).

## The radar

It reads your position over TF2's RCON and the gargoyle from the demo TF2 records; [how the radar works](radar.md) has the details. It is the only widget that sends anything to TF2.

## The countdown

The end date is TF2's own: `contracts_end_date` of the newest `operation_halloween` entry in `tf\scripts\items\items_game.txt`, read when the app finds TF2. Valve moves it there when it extends the event. The file gives no time zone, so the app takes it as midnight UTC and shows it in your local time. The countdown sends nothing to TF2.
