interface StyledElement extends Element {
    style?: CSSStyleDeclaration;
}

interface HostSurfaceStyleSnapshot {
    backgroundColor: string;
    backgroundImage: string;
}

const TRANSPARENT_BACKGROUND_COLOR = "transparent";
const TRANSPARENT_BACKGROUND_IMAGE = "none";

const styledHostElements = new Set<Element>();
const styleSnapshots = new WeakMap<Element, HostSurfaceStyleSnapshot>();

export function syncHostSurfaceTransparency(hostElements: Iterable<Element>): void {
    const currentHosts = new Set(hostElements);

    for (const element of [...styledHostElements]) {
        if (!currentHosts.has(element)) {
            restoreHostSurface(element);
        }
    }

    for (const element of currentHosts) {
        applyHostSurfaceTransparency(element);
    }
}

function applyHostSurfaceTransparency(element: Element): void {
    const style = getInlineStyle(element);
    if (!style) {
        return;
    }

    if (!styleSnapshots.has(element)) {
        styleSnapshots.set(element, {
            backgroundColor: style.backgroundColor,
            backgroundImage: style.backgroundImage,
        });
    }

    styledHostElements.add(element);
    style.backgroundColor = TRANSPARENT_BACKGROUND_COLOR;
    style.backgroundImage = TRANSPARENT_BACKGROUND_IMAGE;
}

function restoreHostSurface(element: Element): void {
    const style = getInlineStyle(element);
    const snapshot = styleSnapshots.get(element);
    styledHostElements.delete(element);

    if (!style || !snapshot) {
        return;
    }

    style.backgroundColor = snapshot.backgroundColor;
    style.backgroundImage = snapshot.backgroundImage;
    styleSnapshots.delete(element);
}

function getInlineStyle(element: Element): CSSStyleDeclaration | undefined {
    return (element as StyledElement).style;
}
