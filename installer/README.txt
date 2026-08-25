Holeshot HUD
============

Standings, relative, map, minimap, and radar for MX Bikes.

Install
-------
1. Run HoleshotHUD-Setup.exe
2. If Windows SmartScreen appears, click More info -> Run anyway
3. Setup installs to %LOCALAPPDATA%\Holeshot HUD
4. Setup finds Steam MX Bikes (or asks you to pick the folder with mxbikes.exe)
   and copies Holeshot-HUD.dlo into the game plugins folder

Use
---
1. Set MX Bikes to borderless or windowed (not exclusive fullscreen)
2. Start MX Bikes, then start Holeshot HUD from the desktop shortcut
3. Press F8 for settings
4. Hold Ctrl and drag widgets to move or resize them

Uninstall
---------
Run Uninstall.bat from this folder, or from:
  %LOCALAPPDATA%\Holeshot HUD\Uninstall.bat
That also deletes saved layout (Documents\PiBoSo\MX Bikes\Holeshot-HUD.ini)
and AppData logs / the remembered game folder path.

Notes
-----
Restart MX Bikes after installing or updating the plugin.
The overlay copies the plugin again on launch if it is missing or outdated.
The game folder is remembered in %LOCALAPPDATA%\Holeshot HUD\gamedir.txt
Layout is saved to Documents\PiBoSo\MX Bikes\Holeshot-HUD.ini
Icons use Font Awesome Free (https://fontawesome.com/license/free)
