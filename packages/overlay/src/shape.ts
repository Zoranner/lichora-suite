export interface RectLike {
    x: number;
    y: number;
    width: number;
    height: number;
}

export type RegionShape = "rect" | "roundedRect";

export interface ElementRegionShape {
    shape: RegionShape;
    radius: number;
}

export function getElementRegionShape(element: Element): ElementRegionShape {
    const style = globalThis.getComputedStyle?.(element);
    const radius = readBorderRadius(style);
    return {
        shape: radius > 0 ? "roundedRect" : "rect",
        radius,
    };
}

export function toRectLike(rect: DOMRect): RectLike {
    return {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    };
}

export function intersectRects(left: RectLike, right: RectLike): RectLike | null {
    const x = Math.max(left.x, right.x);
    const y = Math.max(left.y, right.y);
    const rightEdge = Math.min(left.x + left.width, right.x + right.width);
    const bottomEdge = Math.min(left.y + left.height, right.y + right.height);
    return toValidRect(x, y, rightEdge - x, bottomEdge - y);
}

export function subtractRects(source: RectLike, occluders: RectLike[]): RectLike[] {
    let remaining = [source];
    for (const occluder of occluders) {
        remaining = remaining.flatMap((rect) => subtractRect(rect, occluder));
    }
    return remaining;
}

export function sampleRectPoints(rect: RectLike): Array<{ x: number; y: number }> {
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

function toValidRect(x: number, y: number, width: number, height: number): RectLike | null {
    if (width <= 0 || height <= 0) {
        return null;
    }

    return { x, y, width, height };
}

function readBorderRadius(style: CSSStyleDeclaration | undefined): number {
    if (!style) {
        return 0;
    }

    const radius = readCssPixels(style.borderRadius);
    const cornerRadii = [
        readCssPixels(style.borderTopLeftRadius),
        readCssPixels(style.borderTopRightRadius),
        readCssPixels(style.borderBottomRightRadius),
        readCssPixels(style.borderBottomLeftRadius),
    ].filter((cornerRadius) => cornerRadius > 0);

    if (cornerRadii.length > 0) {
        return Math.min(...cornerRadii);
    }

    return radius;
}

function readCssPixels(value: string | undefined): number {
    if (!value) {
        return 0;
    }

    const firstValue = value.trim().split(/\s+/)[0];
    if (!firstValue.endsWith("px")) {
        return 0;
    }

    const parsed = Number(firstValue.slice(0, -2));
    return Number.isFinite(parsed) && parsed > 0 ? parsed : 0;
}
