# Lichora Overlay

Web SDK for describing input ownership in controlled Lichora overlays.

Input ownership answers one question: at a CSS viewport coordinate, should pointer input belong to the browser or to the host scene?

The target model is `InputOwnershipMap`:

- `defaultOwner` is either `web` or `host`.
- Regions override `defaultOwner` with `owner = web | host`.
- Elements should only be marked when their owner differs from `defaultOwner`.
- The first-stage shapes are `Rect` and `RoundedRect`.

The SDK does not change page layout and does not route host input itself. It publishes low-frequency ownership data for the host side to validate and consume.

## DOM markers

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

- `border-radius: 0` -> `Rect`
- `border-radius > 0` -> `RoundedRect`

## Current bridge

The current package still implements the old pass map chain. Until the formal ownership bridge is available, it emits pass maps through `console.log` with this prefix:

```text
__LICHORA_OVERLAY_PASS_MAP__:
```

The bytes after the prefix are JSON. `version` is serialized as a string because the SDK keeps it as a monotonic bigint internally. Rust validates this payload and converts it into typed `OverlayPassMap` output for the Unity host.

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

## Compatibility layer

`data-overlay="pass"` is the legacy marker. It is equivalent to an ownership region with:

```text
defaultOwner = web
owner = host
```

Legacy markup is still scanned:

```html
<main data-overlay="pass"></main>
```

The compatibility APIs have the same meaning:

- `pass(element)` registers a host-owned region under `defaultOwner = web`.
- `unpass(element)` removes an API registration.
- `refreshPassMap()` republishes the current compatibility map.

The SDK filters regions that are disabled, hidden, invisible, or have `width <= 0` or `height <= 0`.

Viewport metadata comes from:

- `window.innerWidth`
- `window.innerHeight`
- `window.devicePixelRatio || 1`

Refreshes are scheduled from `MutationObserver`, `ResizeObserver`, `scroll`, and `resize`, with throttling. At most 256 regions are published.

The next implementation stage should migrate the public model and bridge payload to the ownership API while keeping this compatibility layer for existing pages.

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

Current exports are compatibility APIs:

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

Registers an element as a legacy pass region, equivalent to an `owner = host` region when `defaultOwner = web`. Returns a dispose function. `options.disabled` keeps the element registered but excludes it from generated regions.

### `unpass(element)`

Removes an API registration. It does not remove `data-overlay="pass"` from the DOM.

### `enable()`

Starts observers and publishes the current compatibility map.

### `disable()`

Stops observers and publishes a disabled empty compatibility map. Existing API registrations remain in memory, so a later `enable()` can publish them again.

### `refreshPassMap()`

Immediately generates and publishes a new compatibility map.

## Development

Use bun only:

```powershell
bun test
```
