import type { InputOwnershipPayload } from "./ownership-map";
import type { OverlayPassMapPayload } from "./pass-map";

export const INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX = "__LICHORA_INPUT_OWNERSHIP_MAP__:";
export const OVERLAY_PASS_MAP_CONSOLE_PREFIX = "__LICHORA_OVERLAY_PASS_MAP__:";

export function publishInputOwnershipMap(payload: InputOwnershipPayload): void {
    const targetConsole = globalThis.window?.console ?? globalThis.console;
    targetConsole.log(`${INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX}${JSON.stringify(payload)}`);
}

export function publishPassMap(payload: OverlayPassMapPayload): void {
    const targetConsole = globalThis.window?.console ?? globalThis.console;
    targetConsole.log(`${OVERLAY_PASS_MAP_CONSOLE_PREFIX}${JSON.stringify(payload)}`);
}
