# Lichora Overlay

This package is the web-side SDK boundary for controlled Lichora overlays.

Current repository work only establishes the package location and public naming. The implementation must follow `docs/design/web-host-overlay.md` and keep the public model pass-only:

- HTML attribute: `data-overlay="pass"`
- SDK API: `pass()`, `unpass()`, `enable()`, `disable()`, `refreshPassMap()`
- Internal payload model: `PassMap`, `PassRegion`, `OverlayPassMap`

Do not introduce browser/host target modes or hit markers here.
