import type { RegionOptions } from "./ownership-map";
import type { RegionRegistration } from "./region-registry";

export type PassOptions = RegionOptions;
export type Dispose = () => void;

export function scanLegacyPassAttributeRegistrations(): RegionRegistration[] {
    const currentDocument = globalThis.document;
    if (!currentDocument?.querySelectorAll) {
        return [];
    }

    return [...currentDocument.querySelectorAll('[data-overlay="pass"]')].map((element) => ({
        element,
        owner: "host",
        options: {},
    }));
}

export function isLegacyPassElement(element: Element): boolean {
    return element.matches?.('[data-overlay="pass"]') ?? false;
}

export function createLegacyPassApi(registerHostRegion: (element: Element, options?: PassOptions) => Dispose) {
    return (element: Element, options: PassOptions = {}): Dispose => registerHostRegion(element, options);
}
