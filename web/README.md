# MXBO web preview

Static page that runs the same HUD renderer in the browser.

```bat
build-web.bat
```

Then open `web/index.html` via a local static server (ES modules cannot load from `file://`):

```bat
npx --yes serve web
```

## Vercel

Import the GitHub repo in Vercel. Leave the root as the repo root — [`vercel.json`](../vercel.json) serves this folder.

After renderer changes, run `build-web.bat` from the repo root and commit `web/pkg` so the hosted demo stays current.
