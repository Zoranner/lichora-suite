export interface VisibilityOptions {
    disabled?: boolean;
}

export function isUsableElement(element: Element, options: VisibilityOptions): boolean {
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

    return isVisibleElement(element);
}

export function isVisibleElement(element: Element): boolean {
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
