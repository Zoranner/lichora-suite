# Lichora Overlay

Web SDK for controlled Lichora overlays.

The first version describes browser regions that should pass pointer input through to the host scene. The SDK does not change page layout and does not route host input itself. It only publishes a pass map for Rust to validate and convert into typed `OverlayPassMap` output. Unity consumes only that typed payload.

## Temporary console bridge

Until the formal browser bridge is available, pass maps are emitted through `console.log` with this prefix:

```text
__LICHORA_OVERLAY_PASS_MAP__:
```

The bytes after the prefix are JSON. `version` is serialized as a string because the SDK keeps it as a monotonic bigint internally.

Example payload:

```json
{
    "version": "3",
    "viewportWidth": 1280,
    "viewportHeight": 720,
    "deviceScaleFactor": 1,
    "enabled": true,
    "regions": [
        {
            "id": 1,
            "shape": "rect",
            "x": 320,
            "y": 80,
            "width": 640,
            "height": 520,
            "disabled": false
        }
    ]
}
```

## Automatic scan

Any element with `data-overlay="pass"` is included in the pass map:

```html
<main data-overlay="pass"></main>
```

The SDK uses `getBoundingClientRect()` and CSS viewport coordinates. It filters regions that are disabled, hidden, invisible, or have `width <= 0` or `height <= 0`.

Viewport metadata comes from:

- `window.innerWidth`
- `window.innerHeight`
- `window.devicePixelRatio || 1`

Refreshes are scheduled from `MutationObserver`, `ResizeObserver`, `scroll`, and `resize`, with throttling. At most 256 regions are published.

## Example page

`examples/pass-map.html` is a self-contained browser example for package development. In the Unity BrowserRenderer project, the same test page is copied to:

```text
BrowserAssets/demo/overlay-pass-map.html
```

Use this Unity address for manual testing:

```text
local://BrowserAssets/demo/overlay-pass-map.html
```

## API

```ts
import { disable, enable, pass, refreshPassMap, unpass } from "@lichora/overlay";

const dispose = pass(element);
const disabledDispose = pass(otherElement, { disabled: true });

enable();
refreshPassMap();

unpass(element);
disabledDispose();
dispose();
disable();
```

### `pass(element, options?)`

Registers an element as a pass-through region and returns a dispose function. `options.disabled` keeps the element registered but excludes it from generated regions.

### `unpass(element)`

Removes an API registration. It does not remove `data-overlay="pass"` from the DOM.

### `enable()`

Starts observers and publishes the current pass map.

### `disable()`

Stops observers and publishes a disabled empty pass map. Existing API registrations remain in memory, so a later `enable()` can publish them again.

### `refreshPassMap()`

Immediately generates and publishes a new pass map.

## Development

Use bun only:

```powershell
bun test
```
