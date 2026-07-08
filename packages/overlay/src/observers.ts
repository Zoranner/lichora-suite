export type RefreshScheduler = () => void;

const OWNERSHIP_ATTRIBUTES = [
    "data-lichora",
    "data-lichora-disabled",
    "data-overlay",
    "style",
    "class",
    "hidden",
    "disabled",
];
const ELEMENT_NODE_TYPE = 1;

export class OverlayObservers {
    private mutationObserver?: MutationObserver;
    private resizeObserver?: ResizeObserver;
    private readonly observedElements = new Set<Element>();
    private scrollListener?: () => void;
    private resizeListener?: () => void;

    public constructor(private readonly scheduleRefresh: RefreshScheduler) {}

    public start(elements: Iterable<Element>): void {
        this.stop();

        const currentDocument = globalThis.document;
        if (currentDocument && globalThis.MutationObserver) {
            this.mutationObserver = new MutationObserver((records) => {
                if (records.some(shouldRefreshForMutation)) {
                    this.scheduleRefresh();
                }
            });
            this.mutationObserver.observe(currentDocument, {
                attributes: true,
                attributeFilter: OWNERSHIP_ATTRIBUTES,
                childList: true,
                subtree: true,
            });
        }

        if (globalThis.ResizeObserver) {
            this.resizeObserver = new ResizeObserver(() => this.scheduleRefresh());
            this.observe(elements);
        }

        this.scrollListener = () => this.scheduleRefresh();
        this.resizeListener = () => this.scheduleRefresh();
        globalThis.window?.addEventListener("scroll", this.scrollListener, true);
        globalThis.window?.addEventListener("resize", this.resizeListener);
    }

    public observe(elements: Iterable<Element>): void {
        if (!this.resizeObserver) {
            return;
        }

        for (const element of elements) {
            if (this.observedElements.has(element)) {
                continue;
            }

            this.observedElements.add(element);
            this.resizeObserver.observe(element);
        }
    }

    public unobserve(element: Element): void {
        if (!this.observedElements.delete(element)) {
            return;
        }

        this.resizeObserver?.unobserve(element);
    }

    public stop(): void {
        this.mutationObserver?.disconnect();
        this.resizeObserver?.disconnect();
        if (this.scrollListener) {
            globalThis.window?.removeEventListener("scroll", this.scrollListener, true);
        }
        if (this.resizeListener) {
            globalThis.window?.removeEventListener("resize", this.resizeListener);
        }

        this.mutationObserver = undefined;
        this.resizeObserver = undefined;
        this.scrollListener = undefined;
        this.resizeListener = undefined;
        this.observedElements.clear();
    }
}

function shouldRefreshForMutation(record: MutationRecord): boolean {
    if (record.type === "attributes") {
        return !!record.attributeName && OWNERSHIP_ATTRIBUTES.includes(record.attributeName);
    }

    if (record.type !== "childList") {
        return false;
    }

    return hasElementNode(record.addedNodes) || hasElementNode(record.removedNodes);
}

function hasElementNode(nodes: NodeList): boolean {
    for (var index = 0; index < nodes.length; index++) {
        if (nodes[index]?.nodeType === ELEMENT_NODE_TYPE) {
            return true;
        }
    }

    return false;
}
