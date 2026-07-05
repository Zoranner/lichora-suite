using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    [Serializable]
    public sealed class InputOwnershipMap
    {
        [SerializeField]
        private ulong _Version;

        [SerializeField]
        private bool _Enabled = true;

        [SerializeField]
        private int _ViewportWidth;

        [SerializeField]
        private int _ViewportHeight;

        [SerializeField]
        private float _DeviceScaleFactor = 1f;

        [SerializeField]
        private InputOwner _DefaultOwner = InputOwner.Web;

        [SerializeField]
        private InputRegion[] _StaticRegions = Array.Empty<InputRegion>();

        private InputRegion[] _DynamicRegions = Array.Empty<InputRegion>();

        public ulong Version
        {
            get => _Version;
            set => _Version = value;
        }

        public bool Enabled
        {
            get => _Enabled;
            set => _Enabled = value;
        }

        public int ViewportWidth
        {
            get => _ViewportWidth;
            set => _ViewportWidth = value;
        }

        public int ViewportHeight
        {
            get => _ViewportHeight;
            set => _ViewportHeight = value;
        }

        public float DeviceScaleFactor
        {
            get => _DeviceScaleFactor;
            set => _DeviceScaleFactor = value;
        }

        public InputOwner DefaultOwner
        {
            get => IsValidOwner(_DefaultOwner) ? _DefaultOwner : InputOwner.Web;
            set => _DefaultOwner = IsValidOwner(value) ? value : InputOwner.Web;
        }

        public InputRegion[] StaticRegions
        {
            get => _StaticRegions;
            set => _StaticRegions = value ?? Array.Empty<InputRegion>();
        }

        public int DynamicRegionCount => _DynamicRegions?.Length ?? 0;

        public void SetDynamicRegions(InputRegion[] regions)
        {
            _DynamicRegions = regions ?? Array.Empty<InputRegion>();
        }

        public void ClearDynamicRegions()
        {
            _DynamicRegions = Array.Empty<InputRegion>();
        }

        public InputOwner ResolveOwner(Vector2 normalizedPosition)
        {
            if (!IsValidMap || !IsValidPosition(normalizedPosition))
            {
                return InputOwner.Web;
            }

            if (TryResolveOwner(_DynamicRegions, normalizedPosition, out var dynamicOwner))
            {
                return dynamicOwner;
            }

            if (TryResolveOwner(_StaticRegions, normalizedPosition, out var staticOwner))
            {
                return staticOwner;
            }

            return DefaultOwner;
        }

        private bool IsValidMap => _Enabled && IsValidOwner(_DefaultOwner);

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
