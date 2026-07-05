import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import fixturePassMap from "../fixtures/pass-map.json";
import { disable, enable, pass, refreshPassMap, unpass } from "./index";
import { OVERLAY_PASS_MAP_CONSOLE_PREFIX } from "./bridge";

const BRIDGE_PREFIX = "__LICHORA_OVERLAY_PASS_MAP__:";
let disposers: Array<() => void> = [];

interface FakeRect {
    x?: number;
    y?: number;
    width: number;
    height: number;
}

class FakeElement {
    public attributes = new Map<string, string>();
    public style: Partial<CSSStyleDeclaration> = {};
    public parentElement: FakeElement | null = null;
    private rect: DOMRect;

    public constructor(rect: FakeRect) {
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

    public matches(selector: string): boolean {
        return selector === '[data-overlay="pass"]' && this.attributes.get("data-overlay") === "pass";
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

    public getBoundingClientRect(): DOMRect {
        return this.rect;
    }
}

class FakeDocument {
    public elements: FakeElement[] = [];

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

    public constructor(private readonly callback: () => void) {
        FakeMutationObserver.instances.push(this);
    }

    public observe(): void {}

    public disconnect(): void {}

    public trigger(): void {
        this.callback();
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

function containsPoint(rect: DOMRect, x: number, y: number): boolean {
    return x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom;
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
    delete (globalThis as { window?: unknown }).window;
    delete (globalThis as { document?: unknown }).document;
    delete (globalThis as { MutationObserver?: unknown }).MutationObserver;
    delete (globalThis as { ResizeObserver?: unknown }).ResizeObserver;
    delete (globalThis as { Element?: unknown }).Element;
    delete (globalThis as { getComputedStyle?: unknown }).getComputedStyle;
});

describe("@lichora/overlay", () => {
    test("fixture payload uses the shared console bridge contract", () => {
        expect(OVERLAY_PASS_MAP_CONSOLE_PREFIX).toBe(BRIDGE_PREFIX);
        expect(JSON.stringify(fixturePassMap)).toBe(
            '{"version":"42","viewportWidth":1280,"viewportHeight":720,"deviceScaleFactor":1.25,"enabled":true,"regions":[{"id":1,"shape":"rect","x":120.5,"y":80.25,"width":640,"height":360,"disabled":false},{"id":2,"shape":"rect","x":32,"y":48,"width":128,"height":96,"disabled":true}]}',
        );
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
        expect(payload.regions).toEqual([
            { id: 1, shape: "rect", x: 200, y: 100, width: 80, height: 40, disabled: false },
            { id: 2, shape: "rect", x: 10, y: 20, width: 100, height: 50, disabled: false },
        ]);
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
            { id: 1, shape: "rect", x: 0, y: 0, width: 20, height: 30, disabled: false },
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
            { id: 1, shape: "rect", x: 0, y: 0, width: 100, height: 25, disabled: false },
            { id: 2, shape: "rect", x: 0, y: 75, width: 100, height: 25, disabled: false },
            { id: 3, shape: "rect", x: 0, y: 25, width: 25, height: 50, disabled: false },
            { id: 4, shape: "rect", x: 75, y: 25, width: 25, height: 50, disabled: false },
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
            { id: 1, shape: "rect", x: 0, y: 0, width: 100, height: 100, disabled: false },
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
        FakeMutationObserver.instances[0]?.trigger();
        FakeResizeObserver.instances[0]?.trigger();
        fakeWindow.dispatch("scroll");
        fakeWindow.dispatch("resize");

        expect(fakeWindow.logs).toHaveLength(0);
        fakeWindow.flushTimers();
        expect(fakeWindow.logs).toHaveLength(1);
        expect(latestPayload(fakeWindow).regions).toHaveLength(1);
    });
});
