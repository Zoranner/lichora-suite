# Lichora Overlay

Web SDK for describing input ownership in controlled Lichora overlays.

Input ownership answers one question: at a CSS viewport coordinate, should pointer input belong to the browser or to the host scene?

The public model is `InputOwnershipMap`:

- `defaultOwner` is either `web` or `host`.
- Regions override `defaultOwner` with `owner = web | host`.
- Elements should only be marked when their owner differs from `defaultOwner`.
- The first-stage shapes are `rect` and `roundedRect`.

The SDK does not change page layout and does not route host input itself. It publishes low-frequency ownership data for the host side to validate and consume.

## DOM Markers

Use `data-lichora="host|web"` as the public ownership marker:

```html
<div class="host-view" data-lichora="host"></div>
<section class="web-panel" data-lichora="web"></section>
```

When `defaultOwner = web`, only host regions need to be marked:

```html
<div class="earth-view" data-lichora="host"></div>
```

When `defaultOwner = host`, only web regions need to be marked:

```html
<section class="tool-panel" data-lichora="web"></section>
```

Coordinates come from `getBoundingClientRect()` in CSS viewport space. Shape is derived from the element box:

- `border-radius: 0` -> `rect`
- `border-radius > 0` -> `roundedRect`

## API

Primary ownership API:

```ts
import { disable, enable, refresh, region, setDefaultOwner, unregion } from "@lichora/overlay";

setDefaultOwner("web");

const disposeHostRegion = region(hostViewportElement, "host");
const disposeWebPanel = region(panelElement, "web", { disabled: false });

enable();
refresh();

unregion(panelElement);
disposeHostRegion();
disposeWebPanel();
disable();
```

### `setDefaultOwner(owner)`

Sets the owner used when no region matches. `owner` must be `"web"` or `"host"`.

### `region(element, owner, options?)`

Registers an element as an ownership region. Returns a dispose function.

`options.disabled` keeps the element registered but excludes it from generated regions.

### `unregion(element)`

Removes an API ownership registration. It does not remove DOM attributes.

### `enable()`

Starts observers and publishes the current ownership state through the current bridge.

### `disable()`

Stops observers and publishes a disabled empty ownership map. Existing API registrations remain in memory, so a later `enable()` can publish them again.

### `refresh()`

Immediately generates and publishes a new ownership state.

## Compatibility Layer

`data-overlay="pass"` is the legacy marker. It is equivalent to:

```text
defaultOwner = web
owner = host
```

Legacy markup is still scanned:

```html
<main data-overlay="pass"></main>
```

Compatibility APIs remain available:

```ts
import { pass, refreshPassMap, unpass } from "@lichora/overlay";

const dispose = pass(element);
const disabledDispose = pass(otherElement, { disabled: true });

refreshPassMap();

unpass(element);
disabledDispose();
dispose();
```

- `pass(element)` maps to `region(element, "host")`.
- `unpass(element)` maps to `unregion(element)`.
- `refreshPassMap()` maps to `refresh()`.

The SDK filters regions that are disabled, hidden, invisible, or have `width <= 0` or `height <= 0`.

Viewport metadata comes from:

- `window.innerWidth`
- `window.innerHeight`
- `window.devicePixelRatio || 1`

Refreshes are scheduled from `MutationObserver`, `ResizeObserver`, `scroll`, and `resize`, with throttling. At most 256 regions are published.

## Current Bridge

The package publishes ownership maps through `window.lichora.postMessage(type, payload)` when a native Lichora bridge is available.

The current ownership message type is:

```text
inputOwnershipMap
```

If the native bridge is not available, the SDK falls back to the temporary console bridge with this prefix:

```text
__LICHORA_INPUT_OWNERSHIP_MAP__:
```

The bytes after the console prefix are JSON. `version` is serialized as a string because the SDK keeps it as a monotonic bigint internally.

Current ownership payload example:

```json
{
    "version": "3",
    "viewportWidth": 1280,
    "viewportHeight": 720,
    "deviceScaleFactor": 1,
    "enabled": true,
    "defaultOwner": "web",
    "regions": [
        {
            "id": 1,
            "owner": "host",
            "shape": "rect",
            "x": 320,
            "y": 80,
            "width": 640,
            "height": 520,
            "radius": 0,
            "disabled": false
        }
    ]
}
```

The old compatibility prefix is still exported for legacy callers:

```text
__LICHORA_OVERLAY_PASS_MAP__:
```

The legacy native bridge message type is:

```text
overlayPassMap
```

`refreshPassMap()` now publishes the ownership bridge payload; legacy pass markup and APIs are converted into Host-owned regions before publishing.

## Example Page

`examples/pass-map.html` is a self-contained browser example for the current ownership bridge. In the Unity BrowserRenderer project, the same test page is copied to:

```text
BrowserAssets/demo/overlay-pass-map.html
```

Use this Unity address for manual testing:

```text
local://BrowserAssets/demo/overlay-pass-map.html
```

## Development

Use bun only:

```powershell
bun test
bun run typecheck
bun run check-package
bun run check
```

`bun run check` is the package quality entry point. It runs the test suite, strict TypeScript checking, and the package export/file contract check without starting a browser or Unity Editor.
