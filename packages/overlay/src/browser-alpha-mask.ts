import type { InputOwnershipMap, InputRegion } from "./ownership-map";

const MASK_ID = "lichora-ownership-mask";
const MASK_PROPERTIES = [
    "mask-image",
    "mask-repeat",
    "mask-position",
    "mask-size",
    "mask-mode",
    "-webkit-mask-image",
    "-webkit-mask-repeat",
    "-webkit-mask-position",
    "-webkit-mask-size",
    "-webkit-mask-source-type",
] as const;

interface InlineStyleValue {
    name: (typeof MASK_PROPERTIES)[number];
    value: string;
    priority: string;
}

interface InlineMaskSnapshot {
    style: CSSStyleDeclaration;
    values: InlineStyleValue[];
}

let inlineMaskSnapshot: InlineMaskSnapshot | undefined;

export function syncBrowserAlphaMask(ownershipMap: InputOwnershipMap): void {
    if (!ownershipMap.enabled) {
        restoreBrowserAlphaMask();
        return;
    }

    const style = globalThis.document?.documentElement?.style;
    if (!style) {
        return;
    }

    if (inlineMaskSnapshot?.style !== style) {
        restoreBrowserAlphaMask();
    }

    inlineMaskSnapshot ??= {
        style,
        values: MASK_PROPERTIES.map((name) => ({
            name,
            value: style.getPropertyValue(name),
            priority: style.getPropertyPriority(name),
        })),
    };

    const maskImage = `url("data:image/svg+xml,${encodeURIComponent(createBrowserAlphaMaskSvg(ownershipMap))}")`;
    setMaskProperty(style, "mask-image", maskImage);
    setMaskProperty(style, "mask-repeat", "no-repeat");
    const maskPosition = `${globalThis.window?.scrollX ?? 0}px ${globalThis.window?.scrollY ?? 0}px`;
    const maskSize = `${ownershipMap.viewportWidth}px ${ownershipMap.viewportHeight}px`;
    setMaskProperty(style, "mask-position", maskPosition);
    setMaskProperty(style, "mask-size", maskSize);
    setMaskProperty(style, "mask-mode", "alpha");
    setMaskProperty(style, "-webkit-mask-image", maskImage);
    setMaskProperty(style, "-webkit-mask-repeat", "no-repeat");
    setMaskProperty(style, "-webkit-mask-position", maskPosition);
    setMaskProperty(style, "-webkit-mask-size", maskSize);
    setMaskProperty(style, "-webkit-mask-source-type", "alpha");
}

function restoreBrowserAlphaMask(): void {
    if (!inlineMaskSnapshot) {
        return;
    }

    const { style, values } = inlineMaskSnapshot;
    for (const { name, value, priority } of values) {
        if (value) {
            style.setProperty(name, value, priority);
        } else {
            style.removeProperty(name);
        }
    }
    inlineMaskSnapshot = undefined;
}

function createBrowserAlphaMaskSvg(ownershipMap: InputOwnershipMap): string {
    const width = ownershipMap.viewportWidth;
    const height = ownershipMap.viewportHeight;
    const baseFill = ownerFill(ownershipMap.defaultOwner);
    const regions = ownershipMap.regions.map(createRegionElement).join("");
    return (
        `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">` +
        `<mask id="${MASK_ID}" maskUnits="userSpaceOnUse" x="0" y="0" width="${width}" height="${height}" mask-type="luminance">` +
        `<rect width="${width}" height="${height}" fill="${baseFill}"/>${regions}</mask>` +
        `<rect width="${width}" height="${height}" fill="white" mask="url(#${MASK_ID})"/>` +
        "</svg>"
    );
}

function createRegionElement(region: InputRegion): string {
    if (region.disabled || region.width <= 0 || region.height <= 0) {
        return "";
    }

    const roundedAttributes =
        region.shape === "roundedRect" && region.radius > 0 ? ` rx="${region.radius}" ry="${region.radius}"` : "";
    return `<rect x="${region.x}" y="${region.y}" width="${region.width}" height="${region.height}"${roundedAttributes} fill="${ownerFill(region.owner)}"/>`;
}

function ownerFill(owner: InputOwnershipMap["defaultOwner"]): "white" | "black" {
    return owner === "web" ? "white" : "black";
}

function setMaskProperty(style: CSSStyleDeclaration, name: string, value: string): void {
    style.setProperty(name, value);
}
