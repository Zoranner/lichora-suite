import { publishPassMap } from "./bridge";
import { OverlayObservers } from "./observers";
import {
    createPassMap,
    scanAttributeRegistrations,
    toPayload,
    type OverlayPassMapPayload,
    type PassMap,
    type PassOptions,
    type PassRegion,
    type PassRegistration,
} from "./pass-map";

export type { OverlayPassMapPayload, PassMap, PassOptions, PassRegion };

export type Dispose = () => void;

const REFRESH_THROTTLE_MS = 50;

const apiRegistrations = new Map<Element, PassOptions>();
let observers: OverlayObservers | undefined;
let enabled = false;
let version = 0n;
let scheduledRefresh: ReturnType<typeof setTimeout> | undefined;

export function pass(element: Element, options: PassOptions = {}): Dispose {
    apiRegistrations.set(element, { ...options });
    observers?.observe([element]);
    scheduleRefresh();

    return () => unpass(element);
}

export function unpass(element: Element): void {
    if (!apiRegistrations.delete(element)) {
        return;
    }

    observers?.unobserve(element);
    scheduleRefresh();
}

export function enable(): void {
    if (enabled) {
        return;
    }

    enabled = true;
    ensureObservers();
    refreshPassMap();
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
    refreshPassMap();
}

export function refreshPassMap(): void {
    clearScheduledRefresh();
    version += 1n;
    const registrations = collectRegistrations();
    observers?.observe(registrations.map((registration) => registration.element));

    publishPassMap(toPayload(createPassMap(version, enabled, registrations)));
}

function ensureObservers(): void {
    if (observers) {
        return;
    }

    observers = new OverlayObservers(scheduleRefresh);
    observers.start(collectRegistrations().map((registration) => registration.element));
}

function collectRegistrations(): PassRegistration[] {
    return [
        ...[...apiRegistrations.entries()].map(([element, options]) => ({ element, options })),
        ...scanAttributeRegistrations(),
    ];
}

function scheduleRefresh(): void {
    if (!enabled || scheduledRefresh !== undefined) {
        return;
    }

    const currentWindow = globalThis.window;
    const setTimer = currentWindow?.setTimeout?.bind(currentWindow) ?? globalThis.setTimeout.bind(globalThis);
    scheduledRefresh = setTimer(refreshPassMap, REFRESH_THROTTLE_MS);
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
