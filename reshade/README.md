# MX Bikes ReShade presets

Lightweight presets that clean muddy shadows and add vibrance without warming dirt tracks orange. Built for wide GPU range (including older cards).

| Preset | Use when |
| --- | --- |
| `MXB-Clean-Vibrant.ini` | Default — cheapest, all GPUs |
| `MXB-Clean-Vibrant-Plus.ini` | Same look + SMAA if you have a little headroom |

## Install

1. Download [ReShade](https://reshade.me/) and run the installer.
2. Pick `mxbikes.exe` (Steam library → `MX Bikes`).
3. Rendering API: **Direct3D 9** (MX Bikes is D3D9).
4. When asked for effects, install the **standard effects** pack so these load:
   - `Deband.fx`
   - `Curves.fx`
   - `LiftGammaGain.fx`
   - `Vibrance.fx`
   - `CAS.fx`
   - `SMAA.fx` (only needed for Plus)
5. Copy the `.ini` files from this folder into the same directory as `mxbikes.exe` (next to `dxgi.dll` / `d3d9.dll` from ReShade).
6. Launch the game, open ReShade (**Home** by default), and select **MXB-Clean-Vibrant** (or Plus) from the preset list.

## Tuning (in-game)

- Still muddy shadows → raise `RGB_Lift` a little on **all three** channels the same amount.
- Want more pop → raise `Vibrance` toward `0.30` (stop before grass/dirt look plastic).
- Dirt going orange → do **not** use Colourfulness, DPX, Technicolor, warm LUTs, or unequal Lift/Gain (extra red). Prefer Vibrance with `1,1,1` balance.
- FPS drop → use Clean Vibrant (no SMAA), or lower `CAS` Sharpening / turn Deband iterations to `1`.

## Deliberately omitted

Heavy / warm / older-GPU-hostile effects are left off: MXAO, SSR, RTGI, bloom stacks, LUTs, FilmGrain, Colourfulness, DPX, Technicolor.
