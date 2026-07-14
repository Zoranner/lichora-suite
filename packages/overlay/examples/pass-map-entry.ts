import { disable, enable, refresh } from "../src/index";

const BRIDGE_PREFIX = "__LICHORA_INPUT_OWNERSHIP_MAP__:";

const hostZone = document.getElementById("host-zone");
const log = document.getElementById("log");
const summary = document.getElementById("summary");
const dialog = document.getElementById("dialog");
const toggleHost = document.getElementById("toggle-host") ?? document.getElementById("toggle-pass");
const resizeHost = document.getElementById("resize-host") ?? document.getElementById("resize-pass");
let enabled = true;
let compact = false;

const originalLog = console.log.bind(console);
console.log = (...args: unknown[]) => {
    originalLog(...args);
    const message = String(args[0] ?? "");
    if (!message.startsWith(BRIDGE_PREFIX)) {
        return;
    }

    const payload = JSON.parse(message.slice(BRIDGE_PREFIX.length)) as {
        version: string;
        enabled: boolean;
        regions: unknown[];
    };
    if (summary) {
        summary.textContent = `version=${payload.version}, enabled=${payload.enabled}, regions=${payload.regions.length}`;
    }
    if (log) {
        log.textContent = JSON.stringify(payload, null, 2);
    }
};

toggleHost?.addEventListener("click", () => {
    enabled = !enabled;
    if (enabled) {
        enable();
    } else {
        disable();
    }
    toggleHost.textContent = enabled ? "禁用中间 Host 区域" : "启用中间 Host 区域";
});

document.getElementById("toggle-dialog")?.addEventListener("click", () => {
    if (dialog) {
        dialog.hidden = !dialog.hidden;
        refresh();
    }
});

document.getElementById("close-dialog")?.addEventListener("click", () => {
    if (dialog) {
        dialog.hidden = true;
        refresh();
    }
});

resizeHost?.addEventListener("click", () => {
    compact = !compact;
    if (hostZone) {
        hostZone.style.margin = compact ? "64px 96px" : "0";
    }
    refresh();
});

document.getElementById("refresh")?.addEventListener("click", refresh);

enable();
