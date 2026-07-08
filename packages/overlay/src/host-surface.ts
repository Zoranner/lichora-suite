interface StyledElement extends Element {
    style?: CSSStyleDeclaration;
}

interface HostSurfaceStyleSnapshot {
    backgroundColor: string;
    backgroundImage: string;
    caretColor: string;
    color: string;
    textShadow: string;
}

const TRANSPARENT_BACKGROUND_COLOR = "transparent";
const TRANSPARENT_BACKGROUND_IMAGE = "none";
const TRANSPARENT_TEXT_COLOR = "transparent";
const TRANSPARENT_TEXT_SHADOW = "none";

const styledSurfaceElements = new Set<Element>();
const styleSnapshots = new WeakMap<Element, HostSurfaceStyleSnapshot>();

export function syncHostSurfaceTransparency(hostElements: Iterable<Element>): void {
    const currentSurfaceElements = collectHostSurfaceElements(hostElements);

    for (const element of [...styledSurfaceElements]) {
        if (!currentSurfaceElements.has(element)) {
            restoreHostSurface(element);
        }
    }

    for (const element of currentSurfaceElements) {
        applyHostSurfaceTransparency(element);
    }
}

function collectHostSurfaceElements(hostElements: Iterable<Element>): Set<Element> {
    const surfaceElements = new Set<Element>();
    for (const hostElement of hostElements) {
        surfaceElements.add(hostElement);
        for (const descendant of hostElement.querySelectorAll("*")) {
            surfaceElements.add(descendant);
        }
    }

    return surfaceElements;
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
            caretColor: style.caretColor,
            color: style.color,
            textShadow: style.textShadow,
        });
    }

    styledSurfaceElements.add(element);
    style.backgroundColor = TRANSPARENT_BACKGROUND_COLOR;
    style.backgroundImage = TRANSPARENT_BACKGROUND_IMAGE;
    style.caretColor = TRANSPARENT_TEXT_COLOR;
    style.color = TRANSPARENT_TEXT_COLOR;
    style.textShadow = TRANSPARENT_TEXT_SHADOW;
}

function restoreHostSurface(element: Element): void {
    const style = getInlineStyle(element);
    const snapshot = styleSnapshots.get(element);
    styledSurfaceElements.delete(element);

    if (!style || !snapshot) {
        return;
    }

    style.backgroundColor = snapshot.backgroundColor;
    style.backgroundImage = snapshot.backgroundImage;
    style.caretColor = snapshot.caretColor;
    style.color = snapshot.color;
    style.textShadow = snapshot.textShadow;
    styleSnapshots.delete(element);
}

function getInlineStyle(element: Element): CSSStyleDeclaration | undefined {
    return (element as StyledElement).style;
}
