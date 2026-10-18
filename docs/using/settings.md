# Settings and data

## The tray

The app runs in the tray. Closing the window keeps it running there, and frees the window's memory until you open it again. The tray menu has **Show** and **Quit**. Starting the app a second time shows the open window.

## Settings

- **Close to tray**: on by default. Off means closing the window quits the app.
- **Start minimized to tray**: the window stays closed when the app starts.
- **Start with Windows**: starts the app in the tray when you sign in to Windows.
- **TF2 folder**: found through Steam's libraries. **Change** picks another folder with a `tf` folder inside; **Use detected** goes back.
- **Restore original casual map selection on quit**: on by default. Off keeps the app's queue in TF2 after it quits.
- **Sign out**: deletes the saved Steam sign-in.

## The data folder

Everything the app saves is in `%APPDATA%\tf2-halloween-utils`; **Open folder** in Settings opens it. It holds your Steam sign-in, so do not share it.

On first start, the app copies your sign-in, synced contracts, manual marks and queue backup from 1.x's `%APPDATA%\tf2-halloween-contracts` folder, once. The old folder is left as it is.

## WebView2

The app's window uses Microsoft Edge WebView2, which is built into Windows 11 and comes with Edge on Windows 10. If the app does not open, install the [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/).
