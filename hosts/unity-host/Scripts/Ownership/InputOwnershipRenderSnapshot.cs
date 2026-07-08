using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    internal readonly struct InputOwnershipRenderSnapshot
    {
        public InputOwnershipRenderSnapshot(
            ulong revision,
            bool enabled,
            InputOwner defaultOwner,
            bool hasHostVisibleRegion,
            bool hasWebVisibleRegion,
            InputRegion[] staticRegions,
            InputRegion[] dynamicRegions
        )
        {
            Revision = revision;
            Enabled = enabled;
            DefaultOwner = IsValidOwner(defaultOwner) ? defaultOwner : InputOwner.Web;
            HasHostVisibleRegion = hasHostVisibleRegion;
            HasWebVisibleRegion = hasWebVisibleRegion;
            StaticRegions = staticRegions ?? Array.Empty<InputRegion>();
            DynamicRegions = dynamicRegions ?? Array.Empty<InputRegion>();
        }

        public ulong Revision { get; }
        public bool Enabled { get; }
        public InputOwner DefaultOwner { get; }
        public bool HasHostVisibleRegion { get; }
        public bool HasWebVisibleRegion { get; }
        public InputRegion[] StaticRegions { get; }
        public InputRegion[] DynamicRegions { get; }

        public InputOwner ResolveOwner(Vector2 normalizedPosition)
        {
            if (!Enabled || !IsValidPosition(normalizedPosition))
            {
                return InputOwner.Web;
            }

            if (TryResolveOwner(DynamicRegions, normalizedPosition, out var dynamicOwner))
            {
                return dynamicOwner;
            }

            if (TryResolveOwner(StaticRegions, normalizedPosition, out var staticOwner))
            {
                return staticOwner;
            }

            return DefaultOwner;
        }

        private static bool TryResolveOwner(
            InputRegion[] regions,
            Vector2 normalizedPosition,
            out InputOwner owner
        )
        {
            owner = InputOwner.Web;

            if (regions == null || regions.Length == 0)
            {
                return false;
            }

            for (var i = regions.Length - 1; i >= 0; i--)
            {
                var region = regions[i];
                if (!region.Contains(normalizedPosition))
                {
                    continue;
                }

                owner = region.Owner;
                return true;
            }

            return false;
        }

        private static bool IsValidPosition(Vector2 normalizedPosition)
        {
            return IsFinite(normalizedPosition.x)
                && IsFinite(normalizedPosition.y)
                && normalizedPosition.x >= 0f
                && normalizedPosition.x <= 1f
                && normalizedPosition.y >= 0f
                && normalizedPosition.y <= 1f;
        }

        private static bool IsValidOwner(InputOwner owner)
        {
            return owner == InputOwner.Web || owner == InputOwner.Host;
        }

        private static bool IsFinite(float value)
        {
            return !float.IsNaN(value) && !float.IsInfinity(value);
        }
    }
}
