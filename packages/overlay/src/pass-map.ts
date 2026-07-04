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

        const rect = registration.element.getBoundingClientRect();
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

    return regions;
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
