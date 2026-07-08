import { publishInputOwnershipMap } from "./bridge";
import { OverlayObservers } from "./observers";
import { scanOwnershipAttributeRegistrations } from "./dom-scan";
import { createLegacyPassApi, scanLegacyPassAttributeRegistrations, type PassOptions } from "./legacy-pass";
import {
    createInputOwnershipMap,
    toOwnershipPayload,
    type InputOwner,
    type InputOwnershipMap,
    type InputOwnershipPayload,
    type InputRegion,
    type RegionOptions,
} from "./ownership-map";
import type { OverlayPassMapPayload, PassMap, PassRegion } from "./pass-map";
import { RegionRegistry, type RegionRegistration } from "./region-registry";

export type {
    InputOwner,
    InputOwnershipMap,
    InputOwnershipPayload,
    InputRegion,
    OverlayPassMapPayload,
    PassMap,
    PassOptions,
    PassRegion,
    RegionOptions,
};

export type Dispose = () => void;

const REFRESH_THROTTLE_MS = 50;

const regionRegistry = new RegionRegistry();
let observers: OverlayObservers | undefined;
let enabled = false;
let defaultOwner: InputOwner = "web";
let version = 0n;
let scheduledRefresh: ReturnType<typeof setTimeout> | undefined;
let lastPublishedSignature: string | undefined;

export function setDefaultOwner(owner: InputOwner): void {
    defaultOwner = owner;
    scheduleRefresh();
}

export function region(element: Element, owner: InputOwner, options: RegionOptions = {}): Dispose {
    regionRegistry.set(element, owner, options);
    observers?.observe([element]);
    scheduleRefresh();

    return () => unregion(element);
}

export function unregion(element: Element): void {
    if (!regionRegistry.delete(element)) {
        return;
    }

    observers?.unobserve(element);
    scheduleRefresh();
}

export const pass = createLegacyPassApi((element: Element, options: PassOptions = {}) =>
    region(element, "host", options),
);

export function unpass(element: Element): void {
    unregion(element);
}

export function refresh(): void {
    refreshNow(false);
}

function refreshScheduled(): void {
    refreshNow(true);
}

function refreshNow(skipUnchanged: boolean): void {
    clearScheduledRefresh();
    const registrations = collectRegistrations();
    observers?.observe(registrations.map((registration) => registration.element));

    const ownershipMap = createInputOwnershipMap(version + 1n, enabled, defaultOwner, registrations);
    const signature = createOwnershipSignature(ownershipMap);
    if (skipUnchanged && signature === lastPublishedSignature) {
        return;
    }

    version += 1n;
    ownershipMap.version = version;
    lastPublishedSignature = signature;
    publishOwnershipMap(ownershipMap);
}

export function refreshPassMap(): void {
    refresh();
}

export function enable(): void {
    if (enabled) {
        return;
    }

    enabled = true;
    ensureObservers();
    refresh();
}

export function disable(): void {
    if (!enabled && !observers) {
        clearScheduledRefresh();
        return;
    }

    enabled = false;
    observers?.stop();
    observers = undefined;
    clearScheduledRefresh();
    refresh();
}

function publishOwnershipMap(ownershipMap: InputOwnershipMap): void {
    publishInputOwnershipMap(toOwnershipPayload(ownershipMap));
}

function createOwnershipSignature(ownershipMap: InputOwnershipMap): string {
    const payload = toOwnershipPayload(ownershipMap);
    return JSON.stringify({
        viewportWidth: payload.viewportWidth,
        viewportHeight: payload.viewportHeight,
        deviceScaleFactor: payload.deviceScaleFactor,
        enabled: payload.enabled,
        defaultOwner: payload.defaultOwner,
        regions: payload.regions,
    });
}

function ensureObservers(): void {
    if (observers) {
        return;
    }

    observers = new OverlayObservers(scheduleRefresh);
    observers.start(collectRegistrations().map((registration) => registration.element));
}

function collectRegistrations(): RegionRegistration[] {
    return [
        ...regionRegistry.values(),
        ...scanOwnershipAttributeRegistrations(),
        ...scanLegacyPassAttributeRegistrations(),
    ];
}

function scheduleRefresh(): void {
    if (!enabled || scheduledRefresh !== undefined) {
        return;
    }

    const currentWindow = globalThis.window;
    const setTimer = currentWindow?.setTimeout?.bind(currentWindow) ?? globalThis.setTimeout.bind(globalThis);
    scheduledRefresh = setTimer(refreshScheduled, REFRESH_THROTTLE_MS);
}

function clearScheduledRefresh(): void {
    if (scheduledRefresh === undefined) {
        return;
    }

    const currentWindow = globalThis.window;
    const clearTimer = currentWindow?.clearTimeout?.bind(currentWindow) ?? globalThis.clearTimeout.bind(globalThis);
    clearTimer(scheduledRefresh);
    scheduledRefresh = undefined;
}
