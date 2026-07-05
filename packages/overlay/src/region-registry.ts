import type { InputOwner, RegionOptions } from "./ownership-map";

export interface RegionRegistration {
    element: Element;
    owner: InputOwner;
    options: RegionOptions;
}

export class RegionRegistry {
    private readonly registrations = new Map<Element, RegionRegistration>();

    public set(element: Element, owner: InputOwner, options: RegionOptions): void {
        this.registrations.set(element, {
            element,
            owner,
            options: { ...options },
        });
    }

    public delete(element: Element): boolean {
        return this.registrations.delete(element);
    }

    public values(): RegionRegistration[] {
        return [...this.registrations.values()];
    }
}
