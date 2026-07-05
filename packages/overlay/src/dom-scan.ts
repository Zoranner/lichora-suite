import type { InputOwner, RegionOptions } from "./ownership-map";
import type { RegionRegistration } from "./region-registry";

export function scanOwnershipAttributeRegistrations(): RegionRegistration[] {
    const currentDocument = globalThis.document;
    if (!currentDocument?.querySelectorAll) {
        return [];
    }

    return [...currentDocument.querySelectorAll('[data-lichora="host"], [data-lichora="web"]')]
        .map((element) => toOwnershipRegistration(element))
        .filter((registration): registration is RegionRegistration => registration !== null);
}

export function isOwnershipElement(element: Element): boolean {
    return element.matches?.('[data-lichora="host"], [data-lichora="web"]') ?? false;
}

function toOwnershipRegistration(element: Element): RegionRegistration | null {
    const owner = parseOwner(element.getAttribute("data-lichora"));
    if (!owner) {
        return null;
    }

    return {
        element,
        owner,
        options: readRegionOptions(element),
    };
}

function readRegionOptions(element: Element): RegionOptions {
    return {
        disabled: element.getAttribute("data-lichora-disabled") === "true",
    };
}

function parseOwner(value: string | null): InputOwner | null {
    return value === "host" || value === "web" ? value : null;
}
