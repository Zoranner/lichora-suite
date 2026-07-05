import type { InputOwnershipMap, InputRegion, RegionOptions } from "./ownership-map";

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

export type PassOptions = RegionOptions;

export function toCompatibilityPassMap(ownershipMap: InputOwnershipMap): PassMap {
    return {
        version: ownershipMap.version,
        viewportWidth: ownershipMap.viewportWidth,
        viewportHeight: ownershipMap.viewportHeight,
        deviceScaleFactor: ownershipMap.deviceScaleFactor,
        enabled: ownershipMap.enabled && ownershipMap.defaultOwner === "web",
        regions:
            ownershipMap.enabled && ownershipMap.defaultOwner === "web"
                ? ownershipMap.regions.filter(isHostRegion).map(toPassRegion)
                : [],
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

function isHostRegion(region: InputRegion): boolean {
    return region.owner === "host" && !region.disabled;
}

function toPassRegion(region: InputRegion): PassRegion {
    return {
        id: region.id,
        shape: "rect",
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
        disabled: false,
    };
}
