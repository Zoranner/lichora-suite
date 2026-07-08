import type { InputOwnershipPayload } from "./ownership-map";
import type { OverlayPassMapPayload } from "./pass-map";

export const INPUT_OWNERSHIP_MAP_BRIDGE_TYPE = "inputOwnershipMap";
export const OVERLAY_PASS_MAP_BRIDGE_TYPE = "overlayPassMap";
export const INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX = "__LICHORA_INPUT_OWNERSHIP_MAP__:";
export const OVERLAY_PASS_MAP_CONSOLE_PREFIX = "__LICHORA_OVERLAY_PASS_MAP__:";

export function publishInputOwnershipMap(payload: InputOwnershipPayload): void {
    if (postNativeBridgeMessage(INPUT_OWNERSHIP_MAP_BRIDGE_TYPE, payload)) {
        return;
    }

    const targetConsole = globalThis.window?.console ?? globalThis.console;
    targetConsole.log(`${INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX}${JSON.stringify(payload)}`);
}

export function publishPassMap(payload: OverlayPassMapPayload): void {
    if (postNativeBridgeMessage(OVERLAY_PASS_MAP_BRIDGE_TYPE, payload)) {
        return;
    }

    const targetConsole = globalThis.window?.console ?? globalThis.console;
    targetConsole.log(`${OVERLAY_PASS_MAP_CONSOLE_PREFIX}${JSON.stringify(payload)}`);
}

function postNativeBridgeMessage(type: string, payload: unknown): boolean {
    const bridge = (globalThis.window as LichoraBridgeWindow | undefined)?.lichora;
    if (typeof bridge?.postMessage !== "function") {
        return false;
    }

    try {
        bridge.postMessage(type, payload);
        return true;
    } catch {
        return false;
    }
}

interface LichoraBridgeWindow extends Window {
    lichora?: {
        postMessage?: (type: string, payload: unknown) => void;
    };
}
