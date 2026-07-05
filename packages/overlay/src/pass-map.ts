export const MAX_PASS_REGIONS = 256;

export interface PassOptions {
    disabled?: boolean;
}

export interface PassRegion {
    id: number;
    shape: "rect";
    x: number;
    y: number;
    width: number;
    height: number;
    disabled: boolean;
}

export interface PassMap {
    version: bigint;
    viewportWidth: number;
    viewportHeight: number;
    deviceScaleFactor: number;
    enabled: boolean;
    regions: PassRegion[];
}

export interface OverlayPassMapPayload {
    version: string;
    viewportWidth: number;
    viewportHeight: number;
    deviceScaleFactor: number;
    enabled: boolean;
    regions: PassRegion[];
}

export interface PassRegistration {
    element: Element;
    options: PassOptions;
}

interface RectLike {
    x: number;
    y: number;
    width: number;
    height: number;
}

export function createPassMap(
    version: bigint,
    enabled: boolean,
    registrations: Iterable<PassRegistration>,
): PassMap {
    return {
        version,
        viewportWidth: getViewportWidth(),
        viewportHeight: getViewportHeight(),
        deviceScaleFactor: getDeviceScaleFactor(),
        enabled,
        regions: enabled ? collectRegions(registrations) : [],
    };
}

export function toPayload(passMap: PassMap): OverlayPassMapPayload {
    return {
        version: passMap.version.toString(),
        viewportWidth: passMap.viewportWidth,
        viewportHeight: passMap.viewportHeight,
        deviceScaleFactor: passMap.deviceScaleFactor,
        enabled: passMap.enabled,
        regions: passMap.regions,
    };
}

export function scanAttributeRegistrations(): PassRegistration[] {
    const currentDocument = globalThis.document;
    if (!currentDocument?.querySelectorAll) {
        return [];
    }

    return [...currentDocument.querySelectorAll('[data-overlay="pass"]')].map((element) => ({
        element,
        options: {},
    }));
}

export function isUsablePassElement(element: Element, options: PassOptions): boolean {
    if (options.disabled) {
        return false;
    }

    if (element.hasAttribute("hidden")) {
        return false;
    }

    if (element.hasAttribute("disabled") || (element as { disabled?: boolean }).disabled === true) {
        return false;
    }

    const rect = element.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) {
        return false;
    }

    const style = globalThis.getComputedStyle?.(element);
    if (style?.display === "none" || style?.visibility === "hidden" || style?.visibility === "collapse") {
        return false;
    }

    if (style?.opacity !== undefined && Number(style.opacity) <= 0) {
        return false;
    }

    return true;
}

function collectRegions(registrations: Iterable<PassRegistration>): PassRegion[] {
    const regions: PassRegion[] = [];
    const seen = new Set<Element>();

    for (const registration of registrations) {
        if (regions.length >= MAX_PASS_REGIONS || seen.has(registration.element)) {
            continue;
        }

        seen.add(registration.element);
        if (!isUsablePassElement(registration.element, registration.options)) {
            continue;
        }

        for (const rect of getVisiblePassRects(registration.element)) {
            if (regions.length >= MAX_PASS_REGIONS) {
                break;
            }

            regions.push({
                id: regions.length + 1,
                shape: "rect",
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                disabled: false,
            });
        }
    }

    return regions;
}

function getVisiblePassRects(element: Element): RectLike[] {
    const rect = toRectLike(element.getBoundingClientRect());
    return subtractRects(rect, getCoveringNonPassRects(element, rect));
}

function getCoveringNonPassRects(element: Element, rect: RectLike): RectLike[] {
    const currentDocument = globalThis.document;
    if (!currentDocument?.elementsFromPoint) {
        return [];
    }

    const occluders: RectLike[] = [];
    const seen = new Set<Element>();
    for (const point of sampleRectPoints(rect)) {
        for (const stackedElement of currentDocument.elementsFromPoint(point.x, point.y)) {
            if (stackedElement === element) {
                break;
            }

            if (
                seen.has(stackedElement) ||
                element.contains(stackedElement) ||
                stackedElement.contains(element) ||
                isPassElement(stackedElement) ||
                !isVisibleElement(stackedElement)
            ) {
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

function sampleRectPoints(rect: RectLike): Array<{ x: number; y: number }> {
    const x1 = rect.x + 0.5;
    const x2 = rect.x + rect.width / 2;
    const x3 = rect.x + rect.width - 0.5;
    const y1 = rect.y + 0.5;
    const y2 = rect.y + rect.height / 2;
    const y3 = rect.y + rect.height - 0.5;
    return [
        { x: x1, y: y1 },
        { x: x2, y: y1 },
        { x: x3, y: y1 },
        { x: x1, y: y2 },
        { x: x2, y: y2 },
        { x: x3, y: y2 },
        { x: x1, y: y3 },
        { x: x2, y: y3 },
        { x: x3, y: y3 },
    ];
}

function subtractRects(source: RectLike, occluders: RectLike[]): RectLike[] {
    let remaining = [source];
    for (const occluder of occluders) {
        remaining = remaining.flatMap((rect) => subtractRect(rect, occluder));
    }
    return remaining;
}

function subtractRect(source: RectLike, occluder: RectLike): RectLike[] {
    const intersection = intersectRects(source, occluder);
    if (!intersection) {
        return [source];
    }

    return [
        toValidRect(source.x, source.y, source.width, intersection.y - source.y),
        toValidRect(
            source.x,
            intersection.y + intersection.height,
            source.width,
            source.y + source.height - intersection.y - intersection.height,
        ),
        toValidRect(source.x, intersection.y, intersection.x - source.x, intersection.height),
        toValidRect(
            intersection.x + intersection.width,
            intersection.y,
            source.x + source.width - intersection.x - intersection.width,
            intersection.height,
        ),
    ].filter((rect): rect is RectLike => rect !== null);
}

function intersectRects(left: RectLike, right: RectLike): RectLike | null {
    const x = Math.max(left.x, right.x);
    const y = Math.max(left.y, right.y);
    const rightEdge = Math.min(left.x + left.width, right.x + right.width);
    const bottomEdge = Math.min(left.y + left.height, right.y + right.height);
    return toValidRect(x, y, rightEdge - x, bottomEdge - y);
}

function toValidRect(x: number, y: number, width: number, height: number): RectLike | null {
    if (width <= 0 || height <= 0) {
        return null;
    }

    return { x, y, width, height };
}

function toRectLike(rect: DOMRect): RectLike {
    return {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    };
}

function isPassElement(element: Element): boolean {
    return element.matches?.('[data-overlay="pass"]') ?? false;
}

function isVisibleElement(element: Element): boolean {
    if (element.hasAttribute("hidden")) {
        return false;
    }

    const rect = element.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) {
        return false;
    }

    const style = globalThis.getComputedStyle?.(element);
    if (style?.display === "none" || style?.visibility === "hidden" || style?.visibility === "collapse") {
        return false;
    }

    if (style?.opacity !== undefined && Number(style.opacity) <= 0) {
        return false;
    }

    return true;
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
