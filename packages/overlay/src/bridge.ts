import type { OverlayPassMapPayload } from "./pass-map";

export const OVERLAY_PASS_MAP_CONSOLE_PREFIX = "__LICHORA_OVERLAY_PASS_MAP__:";

export function publishPassMap(payload: OverlayPassMapPayload): void {
    const targetConsole = globalThis.window?.console ?? globalThis.console;
    targetConsole.log(`${OVERLAY_PASS_MAP_CONSOLE_PREFIX}${JSON.stringify(payload)}`);
}
