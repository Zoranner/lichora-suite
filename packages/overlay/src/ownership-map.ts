import {
    getElementRegionShape,
    intersectRects,
    sampleRectPoints,
    subtractRects,
    toRectLike,
    type RectLike,
    type RegionShape,
} from "./shape";
import { isUsableElement, isVisibleElement } from "./visibility";
import type { RegionRegistration } from "./region-registry";

export const MAX_INPUT_REGIONS = 256;

export type InputOwner = "web" | "host";

export interface RegionOptions {
    disabled?: boolean;
}

export interface InputRegion {
    id: number;
    owner: InputOwner;
    shape: RegionShape;
    x: number;
    y: number;
    width: number;
    height: number;
    radius: number;
    disabled: boolean;
}

export interface InputOwnershipMap {
    version: bigint;
    viewportWidth: number;
    viewportHeight: number;
    deviceScaleFactor: number;
    enabled: boolean;
    defaultOwner: InputOwner;
    regions: InputRegion[];
}

export interface InputOwnershipPayload {
    version: string;
    viewportWidth: number;
    viewportHeight: number;
    deviceScaleFactor: number;
    enabled: boolean;
    defaultOwner: InputOwner;
    regions: InputRegion[];
}

export function createInputOwnershipMap(
    version: bigint,
    enabled: boolean,
    defaultOwner: InputOwner,
    registrations: Iterable<RegionRegistration>,
): InputOwnershipMap {
    return {
        version,
        viewportWidth: getViewportWidth(),
        viewportHeight: getViewportHeight(),
        deviceScaleFactor: getDeviceScaleFactor(),
        enabled,
        defaultOwner,
        regions: enabled ? collectRegions(defaultOwner, registrations) : [],
    };
}

export function toOwnershipPayload(ownershipMap: InputOwnershipMap): InputOwnershipPayload {
    return {
        version: ownershipMap.version.toString(),
        viewportWidth: ownershipMap.viewportWidth,
        viewportHeight: ownershipMap.viewportHeight,
        deviceScaleFactor: ownershipMap.deviceScaleFactor,
        enabled: ownershipMap.enabled,
        defaultOwner: ownershipMap.defaultOwner,
        regions: ownershipMap.regions,
    };
}

function collectRegions(defaultOwner: InputOwner, registrations: Iterable<RegionRegistration>): InputRegion[] {
    const regions: InputRegion[] = [];
    const orderedRegistrations = dedupeRegistrations(registrations);
    const ownerByElement = new Map<Element, InputOwner>(
        orderedRegistrations.map((registration) => [registration.element, registration.owner]),
    );

    for (const registration of orderedRegistrations) {
        if (
            regions.length >= MAX_INPUT_REGIONS ||
            registration.owner === defaultOwner ||
            !isUsableElement(registration.element, registration.options)
        ) {
            continue;
        }

        const elementShape = getElementRegionShape(registration.element);
        for (const rect of getVisibleRegionRects(registration, defaultOwner, ownerByElement)) {
            if (regions.length >= MAX_INPUT_REGIONS) {
                break;
            }

            regions.push({
                id: regions.length + 1,
                owner: registration.owner,
                shape: elementShape.shape,
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                radius: elementShape.radius,
                disabled: false,
            });
        }
    }

    return regions;
}

function dedupeRegistrations(registrations: Iterable<RegionRegistration>): RegionRegistration[] {
    const deduped: RegionRegistration[] = [];
    const seen = new Set<Element>();

    for (const registration of registrations) {
        if (seen.has(registration.element)) {
            continue;
        }

        seen.add(registration.element);
        deduped.push(registration);
    }

    return deduped;
}

function getVisibleRegionRects(
    registration: RegionRegistration,
    defaultOwner: InputOwner,
    ownerByElement: Map<Element, InputOwner>,
): RectLike[] {
    const rect = toRectLike(registration.element.getBoundingClientRect());
    return subtractRects(rect, getCoveringRects(registration, rect, defaultOwner, ownerByElement));
}

function getCoveringRects(
    registration: RegionRegistration,
    rect: RectLike,
    defaultOwner: InputOwner,
    ownerByElement: Map<Element, InputOwner>,
): RectLike[] {
    const currentDocument = globalThis.document;
    if (!currentDocument?.elementsFromPoint) {
        return [];
    }

    const occluders: RectLike[] = [];
    const seen = new Set<Element>();
    for (const point of sampleRectPoints(rect)) {
        for (const stackedElement of currentDocument.elementsFromPoint(point.x, point.y)) {
            if (stackedElement === registration.element) {
                break;
            }

            if (
                seen.has(stackedElement) ||
                !isVisibleElement(stackedElement)
            ) {
                continue;
            }

            if (registration.element.contains(stackedElement) || stackedElement.contains(registration.element)) {
                continue;
            }

            const stackedOwner = resolveStackedOwner(stackedElement, defaultOwner, ownerByElement);
            if (stackedOwner === registration.owner) {
                continue;
            }

            const occluder = intersectRects(rect, toRectLike(stackedElement.getBoundingClientRect()));
            if (occluder) {
                seen.add(stackedElement);
                occluders.push(occluder);
            }
        }
    }

    return occluders;
}

function resolveStackedOwner(
    element: Element,
    defaultOwner: InputOwner,
    ownerByElement: Map<Element, InputOwner>,
): InputOwner {
    return ownerByElement.get(element) ?? defaultOwner;
}

function getViewportWidth(): number {
    return globalThis.window?.innerWidth ?? 0;
}

function getViewportHeight(): number {
    return globalThis.window?.innerHeight ?? 0;
}

function getDeviceScaleFactor(): number {
    return globalThis.window?.devicePixelRatio || 1;
}
