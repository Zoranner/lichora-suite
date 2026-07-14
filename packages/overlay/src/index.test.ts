import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import fixturePassMap from "../fixtures/pass-map.json";
import { disable, enable, pass, refresh, refreshPassMap, region, setDefaultOwner, unpass, unregion } from "./index";
import {
    INPUT_OWNERSHIP_MAP_BRIDGE_TYPE,
    INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX,
    OVERLAY_PASS_MAP_BRIDGE_TYPE,
    OVERLAY_PASS_MAP_CONSOLE_PREFIX,
    publishPassMap,
} from "./bridge";
import { createInputOwnershipMap, toOwnershipPayload } from "./ownership-map";

const BRIDGE_PREFIX = "__LICHORA_INPUT_OWNERSHIP_MAP__:";
let disposers: Array<() => void> = [];

interface FakeRect {
    x?: number;
    y?: number;
    width: number;
    height: number;
}

class FakeStyle {
    private readonly values = new Map<string, { value: string; priority: string }>();

    public setProperty(name: string, value: string, priority = ""): void {
        this.values.set(name, { value, priority });
    }

    public getPropertyValue(name: string): string {
        return this.values.get(name)?.value ?? "";
    }

    public getPropertyPriority(name: string): string {
        return this.values.get(name)?.priority ?? "";
    }

    public removeProperty(name: string): string {
        const value = this.getPropertyValue(name);
        this.values.delete(name);
        return value;
    }
}

class FakeElement {
    public readonly nodeType = 1;
    public attributes = new Map<string, string>();
    public style = new FakeStyle() as FakeStyle & Partial<CSSStyleDeclaration>;
    public parentElement: FakeElement | null = null;
    public children: FakeElement[] = [];
    private rect: DOMRect;

    public constructor(rect: FakeRect) {
        this.setRect(rect);
    }

    public setRect(rect: FakeRect): void {
        this.rect = {
            x: rect.x ?? 0,
            y: rect.y ?? 0,
            width: rect.width,
            height: rect.height,
            top: rect.y ?? 0,
            left: rect.x ?? 0,
            right: (rect.x ?? 0) + rect.width,
            bottom: (rect.y ?? 0) + rect.height,
            toJSON: () => ({}),
        } as DOMRect;
    }

    public setAttribute(name: string, value: string): void {
        this.attributes.set(name, value);
    }

    public hasAttribute(name: string): boolean {
        return this.attributes.has(name);
    }

    public getAttribute(name: string): string | null {
        return this.attributes.get(name) ?? null;
    }

    public matches(selector: string): boolean {
        return selector
            .split(",")
            .map((part) => part.trim())
            .some((part) => {
                if (part === '[data-overlay="pass"]') {
                    return this.attributes.get("data-overlay") === "pass";
                }

                if (part === '[data-lichora="host"]') {
                    return this.attributes.get("data-lichora") === "host";
                }

                if (part === '[data-lichora="web"]') {
                    return this.attributes.get("data-lichora") === "web";
                }

                return false;
            });
    }

    public contains(element: FakeElement): boolean {
        let current: FakeElement | null = element;
        while (current !== null) {
            if (current === this) {
                return true;
            }

            current = current.parentElement;
        }

        return false;
    }

    public appendChild(element: FakeElement): void {
        element.parentElement = this;
        this.children.push(element);
    }

    public querySelectorAll(selector: string): FakeElement[] {
        if (selector !== "*") {
            return [];
        }

        return this.children.flatMap((child) => [child, ...child.querySelectorAll(selector)]);
    }

    public getBoundingClientRect(): DOMRect {
        return this.rect;
    }
}

class FakeDocument {
    public elements: FakeElement[] = [];
    public documentElement = new FakeElement({ width: 800, height: 600 });

    public querySelectorAll(selector: string): FakeElement[] {
        return this.elements.filter((element) => element.matches(selector));
    }

    public elementsFromPoint(x: number, y: number): FakeElement[] {
        return this.elements
            .filter((element) => containsPoint(element.getBoundingClientRect(), x, y))
            .toReversed();
    }
}

class FakeMutationObserver {
    public static instances: FakeMutationObserver[] = [];

    public constructor(private readonly callback: (records: MutationRecord[]) => void) {
        FakeMutationObserver.instances.push(this);
    }

    public observe(): void {}

    public disconnect(): void {}

    public trigger(records: MutationRecord[] = [attributeMutation("data-lichora")]): void {
        this.callback(records);
    }
}

class FakeResizeObserver {
    public static instances: FakeResizeObserver[] = [];
    public observed: Element[] = [];

    public constructor(private readonly callback: () => void) {
        FakeResizeObserver.instances.push(this);
    }

    public observe(element: Element): void {
        this.observed.push(element);
    }

    public unobserve(element: Element): void {
        this.observed = this.observed.filter((observed) => observed !== element);
    }

    public disconnect(): void {
        this.observed = [];
    }

    public trigger(): void {
        this.callback();
    }
}

class FakeWindow {
    public innerWidth = 800;
    public innerHeight = 600;
    public devicePixelRatio = 2;
    public scrollX = 120;
    public scrollY = 340;
    public document = new FakeDocument();
    public listeners = new Map<string, Set<() => void>>();
    public MutationObserver = FakeMutationObserver;
    public ResizeObserver = FakeResizeObserver;
    public console = {
        log: (message: string) => {
            this.logs.push(message);
        },
    };
    public logs: string[] = [];
    public nativeMessages: Array<{ type: string; payload: unknown }> = [];
    public lichora?: {
        postMessage: (type: string, payload: unknown) => void;
    };
    private nextTimeoutId = 1;
    private timeouts = new Map<number, () => void>();

    public getComputedStyle(element: FakeElement): Partial<CSSStyleDeclaration> {
        return element.style;
    }

    public addEventListener(type: string, listener: () => void): void {
        const listeners = this.listeners.get(type) ?? new Set<() => void>();
        listeners.add(listener);
        this.listeners.set(type, listeners);
    }

    public removeEventListener(type: string, listener: () => void): void {
        this.listeners.get(type)?.delete(listener);
    }

    public dispatch(type: string): void {
        for (const listener of this.listeners.get(type) ?? []) {
            listener();
        }
    }

    public setTimeout(callback: () => void): number {
        const id = this.nextTimeoutId++;
        this.timeouts.set(id, callback);
        return id;
    }

    public clearTimeout(id: number): void {
        this.timeouts.delete(id);
    }

    public flushTimers(): void {
        const callbacks = [...this.timeouts.values()];
        this.timeouts.clear();
        for (const callback of callbacks) {
            callback();
        }
    }
}

function installFakeWindow(): FakeWindow {
    const fakeWindow = new FakeWindow();
    Object.defineProperty(globalThis, "window", {
        configurable: true,
        value: fakeWindow,
    });
    Object.defineProperty(globalThis, "document", {
        configurable: true,
        value: fakeWindow.document,
    });
    Object.defineProperty(globalThis, "MutationObserver", {
        configurable: true,
        value: FakeMutationObserver,
    });
    Object.defineProperty(globalThis, "ResizeObserver", {
        configurable: true,
        value: FakeResizeObserver,
    });
    Object.defineProperty(globalThis, "Element", {
        configurable: true,
        value: FakeElement,
    });
    Object.defineProperty(globalThis, "getComputedStyle", {
        configurable: true,
        value: (element: FakeElement) => fakeWindow.getComputedStyle(element),
    });
    Object.defineProperty(globalThis, "console", {
        configurable: true,
        value: fakeWindow.console,
    });
    return fakeWindow;
}

function latestPayload(fakeWindow: FakeWindow) {
    const latest = fakeWindow.logs.at(-1);
    expect(latest).toStartWith(BRIDGE_PREFIX);
    return JSON.parse(latest!.slice(BRIDGE_PREFIX.length));
}

function decodeMaskSvg(style: FakeStyle): string {
    const maskImage = style.getPropertyValue("mask-image");
    const prefix = 'url("data:image/svg+xml,';
    expect(maskImage).toStartWith(prefix);
    expect(maskImage).toEndWith('")');
    return decodeURIComponent(maskImage.slice(prefix.length, -2));
}

function containsPoint(rect: DOMRect, x: number, y: number): boolean {
    return x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom;
}

function attributeMutation(attributeName: string): MutationRecord {
    return {
        type: "attributes",
        attributeName,
        addedNodes: nodeList([]),
        removedNodes: nodeList([]),
    } as MutationRecord;
}

function childListMutation(addedNodes: Node[] = [], removedNodes: Node[] = []): MutationRecord {
    return {
        type: "childList",
        attributeName: null,
        addedNodes: nodeList(addedNodes),
        removedNodes: nodeList(removedNodes),
    } as MutationRecord;
}

function nodeList(nodes: Node[]): NodeList {
    return {
        length: nodes.length,
        item: (index: number) => nodes[index] ?? null,
        ...nodes,
    } as unknown as NodeList;
}

beforeEach(() => {
    FakeMutationObserver.instances = [];
    FakeResizeObserver.instances = [];
});

afterEach(() => {
    for (const dispose of disposers) {
        dispose();
    }
    disposers = [];
    disable();
    setDefaultOwner("web");
    delete (globalThis as { window?: unknown }).window;
    delete (globalThis as { document?: unknown }).document;
    delete (globalThis as { MutationObserver?: unknown }).MutationObserver;
    delete (globalThis as { ResizeObserver?: unknown }).ResizeObserver;
    delete (globalThis as { Element?: unknown }).Element;
    delete (globalThis as { getComputedStyle?: unknown }).getComputedStyle;
});

describe("@lichora/overlay", () => {
    test("fixture payload uses the shared console bridge contract", () => {
        expect(INPUT_OWNERSHIP_MAP_BRIDGE_TYPE).toBe("inputOwnershipMap");
        expect(OVERLAY_PASS_MAP_BRIDGE_TYPE).toBe("overlayPassMap");
        expect(INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX).toBe(BRIDGE_PREFIX);
        expect(OVERLAY_PASS_MAP_CONSOLE_PREFIX).toBe("__LICHORA_OVERLAY_PASS_MAP__:");
        expect(JSON.stringify(fixturePassMap)).toBe(
            '{"version":"42","viewportWidth":1280,"viewportHeight":720,"deviceScaleFactor":1.25,"enabled":true,"regions":[{"id":1,"shape":"rect","x":120.5,"y":80.25,"width":640,"height":360,"disabled":false},{"id":2,"shape":"rect","x":32,"y":48,"width":128,"height":96,"disabled":true}]}',
        );
    });

    test("native bridge is preferred and console bridge remains the fallback", () => {
        const fakeWindow = installFakeWindow();
        fakeWindow.lichora = {
            postMessage: (type, payload) => {
                fakeWindow.nativeMessages.push({ type, payload });
            },
        };
        const payload = toOwnershipPayload(createInputOwnershipMap(1n, true, "web", []));

        enable();
        publishPassMap(fixturePassMap);
        expect(fakeWindow.logs).toHaveLength(0);
        expect(fakeWindow.nativeMessages.map((message) => message.type)).toEqual([
            INPUT_OWNERSHIP_MAP_BRIDGE_TYPE,
            OVERLAY_PASS_MAP_BRIDGE_TYPE,
        ]);
        expect(fakeWindow.nativeMessages[0].payload).toEqual(payload);

        fakeWindow.lichora.postMessage = () => {
            throw new Error("native bridge unavailable");
        };
        refresh();
        expect(fakeWindow.logs).toHaveLength(1);
        expect(fakeWindow.logs[0]).toStartWith(BRIDGE_PREFIX);
    });

    test("scans data-overlay pass elements and API registrations into console pass maps", () => {
        const fakeWindow = installFakeWindow();
        const scanned = new FakeElement({ x: 10, y: 20, width: 100, height: 50 });
        scanned.setAttribute("data-overlay", "pass");
        const registered = new FakeElement({ x: 200, y: 100, width: 80, height: 40 });
        fakeWindow.document.elements.push(scanned, registered);

        disposers.push(pass(registered));
        enable();

        const payload = latestPayload(fakeWindow);
        expect(typeof payload.version).toBe("string");
        expect(payload.viewportWidth).toBe(800);
        expect(payload.viewportHeight).toBe(600);
        expect(payload.deviceScaleFactor).toBe(2);
        expect(payload.enabled).toBe(true);
        expect(payload.defaultOwner).toBe("web");
        expect(payload.regions).toEqual([
            {
                id: 1,
                owner: "host",
                shape: "rect",
                x: 200,
                y: 100,
                width: 80,
                height: 40,
                radius: 0,
                disabled: false,
            },
            {
                id: 2,
                owner: "host",
                shape: "rect",
                x: 10,
                y: 20,
                width: 100,
                height: 50,
                radius: 0,
                disabled: false,
            },
        ]);
    });

    test("scans data-lichora host elements as the primary ownership marker", () => {
        const fakeWindow = installFakeWindow();
        const scanned = new FakeElement({ x: 10, y: 20, width: 100, height: 50 });
        scanned.setAttribute("data-lichora", "host");
        fakeWindow.document.elements.push(scanned);

        enable();

        expect(latestPayload(fakeWindow).regions).toEqual([
            {
                id: 1,
                owner: "host",
                shape: "rect",
                x: 10,
                y: 20,
                width: 100,
                height: 50,
                radius: 0,
                disabled: false,
            },
        ]);
    });

    test("applies Chromium alpha before publishing and restores the original root mask on disable", () => {
        const fakeWindow = installFakeWindow();
        const scanned = new FakeElement({ x: 10, y: 20, width: 100, height: 50 });
        const rootStyle = fakeWindow.document.documentElement.style as FakeStyle;
        rootStyle.setProperty("mask-image", "linear-gradient(black, transparent)", "important");
        rootStyle.setProperty("-webkit-mask-image", "url(original-mask.svg)");
        scanned.style.borderRadius = "12px";
        scanned.setAttribute("data-lichora", "host");
        fakeWindow.document.elements.push(scanned);
        fakeWindow.lichora = {
            postMessage: (type, payload) => {
                expect(rootStyle.getPropertyValue("mask-image")).toStartWith('url("data:image/svg+xml,');
                fakeWindow.nativeMessages.push({ type, payload });
            },
        };

        enable();

        expect(rootStyle.getPropertyValue("-webkit-mask-image")).toBe(rootStyle.getPropertyValue("mask-image"));
        expect(rootStyle.getPropertyValue("mask-repeat")).toBe("no-repeat");
        expect(rootStyle.getPropertyValue("-webkit-mask-repeat")).toBe("no-repeat");
        expect(rootStyle.getPropertyValue("mask-position")).toBe("120px 340px");
        expect(rootStyle.getPropertyValue("-webkit-mask-position")).toBe("120px 340px");
        expect(rootStyle.getPropertyValue("mask-size")).toBe("800px 600px");
        expect(rootStyle.getPropertyValue("-webkit-mask-size")).toBe("800px 600px");
        expect(rootStyle.getPropertyValue("mask-mode")).toBe("alpha");
        const svg = decodeMaskSvg(rootStyle);
        expect(svg).toContain('mask-type="luminance"');
        expect(svg).toContain('<rect width="800" height="600" fill="white"/>');
        expect(svg).toContain(
            '<rect x="10" y="20" width="100" height="50" rx="12" ry="12" fill="black"/>',
        );
        expect(svg).toContain('mask="url(#lichora-ownership-mask)"');

        disable();

        expect(rootStyle.getPropertyValue("mask-image")).toBe("linear-gradient(black, transparent)");
        expect(rootStyle.getPropertyPriority("mask-image")).toBe("important");
        expect(rootStyle.getPropertyValue("-webkit-mask-image")).toBe("url(original-mask.svg)");
        expect(rootStyle.getPropertyValue("mask-repeat")).toBe("");
    });

    test("region registers ownership regions and unregion removes them", () => {
        const fakeWindow = installFakeWindow();
        const element = new FakeElement({ x: 30, y: 40, width: 120, height: 60 });

        disposers.push(region(element, "host"));
        enable();
        expect(latestPayload(fakeWindow).regions).toEqual([
            {
                id: 1,
                owner: "host",
                shape: "rect",
                x: 30,
                y: 40,
                width: 120,
                height: 60,
                radius: 0,
                disabled: false,
            },
        ]);

        unregion(element);
        refresh();
        expect(latestPayload(fakeWindow).regions).toEqual([]);
    });

    test("setDefaultOwner publishes web regions over host defaults", () => {
        const fakeWindow = installFakeWindow();
        const webPanel = new FakeElement({ x: 10, y: 20, width: 100, height: 50 });

        setDefaultOwner("host");
        disposers.push(region(webPanel, "web"));
        enable();

        const payload = latestPayload(fakeWindow);
        expect(payload.enabled).toBe(true);
        expect(payload.defaultOwner).toBe("host");
        expect(payload.regions).toEqual([
            {
                id: 1,
                owner: "web",
                shape: "rect",
                x: 10,
                y: 20,
                width: 100,
                height: 50,
                radius: 0,
                disabled: false,
            },
        ]);
    });

    test("ownership model serializes default owner, owners, and rounded rectangles", () => {
        const fakeWindow = installFakeWindow();
        const rounded = new FakeElement({ x: 10, y: 20, width: 100, height: 50 });
        rounded.style.borderRadius = "12px";

        const map = createInputOwnershipMap(42n, true, "host", [
            { element: rounded, owner: "web", options: {} },
        ]);
        const payload = toOwnershipPayload(map);

        expect(payload).toEqual({
            version: "42",
            viewportWidth: 800,
            viewportHeight: 600,
            deviceScaleFactor: 2,
            enabled: true,
            defaultOwner: "host",
            regions: [
                {
                    id: 1,
                    owner: "web",
                    shape: "roundedRect",
                    x: 10,
                    y: 20,
                    width: 100,
                    height: 50,
                    radius: 12,
                    disabled: false,
                },
            ],
        });
    });

    test("API disabled option overrides data-overlay scanning for the same element", () => {
        const fakeWindow = installFakeWindow();
        const element = new FakeElement({ x: 10, y: 20, width: 100, height: 50 });
        element.setAttribute("data-overlay", "pass");
        fakeWindow.document.elements.push(element);

        disposers.push(pass(element, { disabled: true }));
        enable();

        expect(latestPayload(fakeWindow).regions).toEqual([]);
    });

    test("filters disabled, invisible, and empty regions", () => {
        const fakeWindow = installFakeWindow();
        const disabledElement = new FakeElement({ width: 10, height: 10 });
        const hiddenElement = new FakeElement({ width: 10, height: 10 });
        const emptyElement = new FakeElement({ width: 0, height: 10 });
        const visibleElement = new FakeElement({ width: 20, height: 30 });
        hiddenElement.style.display = "none";

        disposers.push(pass(disabledElement, { disabled: true }));
        disposers.push(pass(hiddenElement));
        disposers.push(pass(emptyElement));
        disposers.push(pass(visibleElement));
        enable();

        const payload = latestPayload(fakeWindow);
        expect(payload.regions).toEqual([
            {
                id: 1,
                owner: "host",
                shape: "rect",
                x: 0,
                y: 0,
                width: 20,
                height: 30,
                radius: 0,
                disabled: false,
            },
        ]);
    });

    test("subtracts visible non-pass elements covering pass regions", () => {
        const fakeWindow = installFakeWindow();
        const passElement = new FakeElement({ x: 0, y: 0, width: 100, height: 100 });
        const dialog = new FakeElement({ x: 25, y: 25, width: 50, height: 50 });
        passElement.setAttribute("data-overlay", "pass");
        fakeWindow.document.elements.push(passElement, dialog);

        enable();

        expect(latestPayload(fakeWindow).regions).toEqual([
            { id: 1, owner: "host", shape: "rect", x: 0, y: 0, width: 100, height: 25, radius: 0, disabled: false },
            { id: 2, owner: "host", shape: "rect", x: 0, y: 75, width: 100, height: 25, radius: 0, disabled: false },
            { id: 3, owner: "host", shape: "rect", x: 0, y: 25, width: 25, height: 50, radius: 0, disabled: false },
            { id: 4, owner: "host", shape: "rect", x: 75, y: 25, width: 25, height: 50, radius: 0, disabled: false },
        ]);
    });

    test("subtracts data-lichora web panels covering host regions", () => {
        const fakeWindow = installFakeWindow();
        const hostView = new FakeElement({ x: 0, y: 0, width: 100, height: 100 });
        const webPanel = new FakeElement({ x: 25, y: 25, width: 50, height: 50 });
        hostView.setAttribute("data-lichora", "host");
        webPanel.setAttribute("data-lichora", "web");
        fakeWindow.document.elements.push(hostView, webPanel);

        enable();

        expect(latestPayload(fakeWindow).regions).toEqual([
            { id: 1, owner: "host", shape: "rect", x: 0, y: 0, width: 100, height: 25, radius: 0, disabled: false },
            { id: 2, owner: "host", shape: "rect", x: 0, y: 75, width: 100, height: 25, radius: 0, disabled: false },
            { id: 3, owner: "host", shape: "rect", x: 0, y: 25, width: 25, height: 50, radius: 0, disabled: false },
            { id: 4, owner: "host", shape: "rect", x: 75, y: 25, width: 25, height: 50, radius: 0, disabled: false },
        ]);
    });

    test("keeps pass element descendants inside pass regions", () => {
        const fakeWindow = installFakeWindow();
        const passElement = new FakeElement({ x: 0, y: 0, width: 100, height: 100 });
        const child = new FakeElement({ x: 25, y: 25, width: 50, height: 50 });
        passElement.setAttribute("data-overlay", "pass");
        child.parentElement = passElement;
        fakeWindow.document.elements.push(passElement, child);

        enable();

        expect(latestPayload(fakeWindow).regions).toEqual([
            { id: 1, owner: "host", shape: "rect", x: 0, y: 0, width: 100, height: 100, radius: 0, disabled: false },
        ]);
    });

    test("keeps pass element ancestors from occluding pass regions", () => {
        const fakeWindow = installFakeWindow();
        const ancestor = new FakeElement({ x: 0, y: 0, width: 100, height: 100 });
        const passElement = new FakeElement({ x: 0, y: 0, width: 100, height: 100 });
        passElement.setAttribute("data-overlay", "pass");
        passElement.parentElement = ancestor;
        fakeWindow.document.elements.push(passElement, ancestor);

        enable();

        expect(latestPayload(fakeWindow).regions).toEqual([
            { id: 1, owner: "host", shape: "rect", x: 0, y: 0, width: 100, height: 100, radius: 0, disabled: false },
        ]);
    });

    test("limits regions to 256 and keeps version as a JSON string", () => {
        const fakeWindow = installFakeWindow();
        for (let index = 0; index < 300; index += 1) {
            disposers.push(pass(new FakeElement({ x: index, y: 0, width: 1, height: 1 })));
        }

        enable();
        const firstVersion = BigInt(latestPayload(fakeWindow).version);
        refreshPassMap();

        const payload = latestPayload(fakeWindow);
        expect(BigInt(payload.version)).toBeGreaterThan(firstVersion);
        expect(payload.regions).toHaveLength(256);
        expect(typeof payload.version).toBe("string");
    });

    test("unpass removes API registrations and disable publishes an empty disabled map", () => {
        const fakeWindow = installFakeWindow();
        const element = new FakeElement({ width: 20, height: 20 });

        disposers.push(pass(element));
        enable();
        refreshPassMap();
        unpass(element);
        refreshPassMap();

        expect(latestPayload(fakeWindow).regions).toEqual([]);

        disable();

        const payload = latestPayload(fakeWindow);
        expect(payload.enabled).toBe(false);
        expect(payload.regions).toEqual([]);
    });

    test("mutation, resize, scroll, and window resize schedule throttled refreshes", () => {
        const fakeWindow = installFakeWindow();
        const element = new FakeElement({ width: 20, height: 20 });
        fakeWindow.document.elements.push(element);
        element.setAttribute("data-overlay", "pass");

        enable();
        fakeWindow.logs = [];
        element.setRect({ width: 24, height: 20 });
        FakeMutationObserver.instances[0]?.trigger();
        FakeResizeObserver.instances[0]?.trigger();
        fakeWindow.dispatch("scroll");
        fakeWindow.dispatch("resize");

        expect(fakeWindow.logs).toHaveLength(0);
        fakeWindow.flushTimers();
        expect(fakeWindow.logs).toHaveLength(1);
        expect(latestPayload(fakeWindow).regions).toHaveLength(1);
    });

    test("mutation observer ignores text-only child changes", () => {
        const fakeWindow = installFakeWindow();
        const element = new FakeElement({ width: 20, height: 20 });
        fakeWindow.document.elements.push(element);
        element.setAttribute("data-lichora", "host");

        enable();
        fakeWindow.logs = [];
        FakeMutationObserver.instances[0]?.trigger([childListMutation([{ nodeType: 3 } as Node])]);
        fakeWindow.flushTimers();

        expect(fakeWindow.logs).toHaveLength(0);

        const addedElement = new FakeElement({ x: 30, y: 0, width: 5, height: 5 });
        addedElement.setAttribute("data-lichora", "host");
        fakeWindow.document.elements.push(addedElement);
        FakeMutationObserver.instances[0]?.trigger([childListMutation([addedElement as Node])]);
        fakeWindow.flushTimers();

        expect(fakeWindow.logs).toHaveLength(1);
    });

    test("scheduled refresh skips unchanged ownership payloads", () => {
        const fakeWindow = installFakeWindow();
        const element = new FakeElement({ width: 20, height: 20 });
        fakeWindow.document.elements.push(element);
        element.setAttribute("data-lichora", "host");

        enable();
        fakeWindow.logs = [];

        FakeMutationObserver.instances[0]?.trigger();
        fakeWindow.flushTimers();
        expect(fakeWindow.logs).toHaveLength(0);

        element.setRect({ width: 24, height: 20 });
        FakeResizeObserver.instances[0]?.trigger();
        fakeWindow.flushTimers();

        expect(fakeWindow.logs).toHaveLength(1);
        expect(latestPayload(fakeWindow).regions[0].width).toBe(24);
    });
});
