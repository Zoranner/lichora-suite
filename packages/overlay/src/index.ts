export interface PassOptions {
    disabled?: boolean;
}

export interface PassRegion {
    id: number;
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

export type Dispose = () => void;

export function pass(_element: Element, _options: PassOptions = {}): Dispose {
    throw new Error("@lichora/overlay is a package boundary; implementation is not included yet.");
}

export function unpass(_element: Element): void {
    throw new Error("@lichora/overlay is a package boundary; implementation is not included yet.");
}

export function enable(): void {
    throw new Error("@lichora/overlay is a package boundary; implementation is not included yet.");
}

export function disable(): void {
    throw new Error("@lichora/overlay is a package boundary; implementation is not included yet.");
}

export function refreshPassMap(): void {
    throw new Error("@lichora/overlay is a package boundary; implementation is not included yet.");
}
