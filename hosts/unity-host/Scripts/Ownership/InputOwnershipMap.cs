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
        private ulong _Revision;

        public ulong Revision => _Revision;

        public ulong Version
        {
            get => _Version;
            set => _Version = value;
        }

        public bool Enabled
        {
            get => _Enabled;
            set
            {
                if (_Enabled == value)
                {
                    return;
                }

                _Enabled = value;
                IncrementRevision();
            }
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
            set
            {
                var owner = IsValidOwner(value) ? value : InputOwner.Web;
                if (_DefaultOwner == owner)
                {
                    return;
                }

                _DefaultOwner = owner;
                IncrementRevision();
            }
        }

        public InputRegion[] StaticRegions
        {
            get => CopyRegions(_StaticRegions);
            set
            {
                _StaticRegions = CopyRegions(value);
                IncrementRevision();
            }
        }

        public int DynamicRegionCount => _DynamicRegions?.Length ?? 0;

        public bool HasHostVisibleRegion =>
            IsValidMap
            && (
                DefaultOwner == InputOwner.Host
                || HasRegionOwnedBy(_StaticRegions, InputOwner.Host)
                || HasRegionOwnedBy(_DynamicRegions, InputOwner.Host)
            );

        public bool HasWebVisibleRegion =>
            IsValidMap
            && (
                DefaultOwner == InputOwner.Web
                || HasRegionOwnedBy(_StaticRegions, InputOwner.Web)
                || HasRegionOwnedBy(_DynamicRegions, InputOwner.Web)
            );

        public void SetDynamicRegions(InputRegion[] regions)
        {
            var nextRegions = CopyRegions(regions);
            if (RegionsEqual(_DynamicRegions, nextRegions))
            {
                return;
            }

            _DynamicRegions = nextRegions;
            IncrementRevision();
        }

        public void ClearDynamicRegions()
        {
            if (_DynamicRegions == null || _DynamicRegions.Length == 0)
            {
                return;
            }

            _DynamicRegions = Array.Empty<InputRegion>();
            IncrementRevision();
        }

        public void ResetDynamicOwnership()
        {
            var changed =
                !_Enabled
                || _DefaultOwner != InputOwner.Web
                || (_DynamicRegions != null && _DynamicRegions.Length > 0);

            _Version = 0;
            _Enabled = true;
            _ViewportWidth = 0;
            _ViewportHeight = 0;
            _DeviceScaleFactor = 1f;
            _DefaultOwner = InputOwner.Web;
            _DynamicRegions = Array.Empty<InputRegion>();

            if (changed)
            {
                IncrementRevision();
            }
        }

        internal InputOwnershipRenderSnapshot CreateRenderSnapshot()
        {
            return new InputOwnershipRenderSnapshot(
                _Revision,
                _Enabled,
                DefaultOwner,
                HasHostVisibleRegion,
                HasWebVisibleRegion,
                _StaticRegions ?? Array.Empty<InputRegion>(),
                _DynamicRegions ?? Array.Empty<InputRegion>()
            );
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

        private void IncrementRevision()
        {
            _Revision++;
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

        private static InputRegion[] CopyRegions(InputRegion[] regions)
        {
            if (regions == null || regions.Length == 0)
            {
                return Array.Empty<InputRegion>();
            }

            var copy = new InputRegion[regions.Length];
            Array.Copy(regions, copy, regions.Length);
            return copy;
        }

        private static bool RegionsEqual(InputRegion[] left, InputRegion[] right)
        {
            if (left == null || left.Length == 0)
            {
                return right == null || right.Length == 0;
            }

            if (right == null || left.Length != right.Length)
            {
                return false;
            }

            for (var index = 0; index < left.Length; index++)
            {
                if (!RegionEquals(left[index], right[index]))
                {
                    return false;
                }
            }

            return true;
        }

        private static bool RegionEquals(InputRegion left, InputRegion right)
        {
            return left.Id == right.Id
                && left.Owner == right.Owner
                && left.Shape == right.Shape
                && Mathf.Approximately(left.Rect.x, right.Rect.x)
                && Mathf.Approximately(left.Rect.y, right.Rect.y)
                && Mathf.Approximately(left.Rect.width, right.Rect.width)
                && Mathf.Approximately(left.Rect.height, right.Rect.height)
                && Mathf.Approximately(left.Radius, right.Radius)
                && left.Disabled == right.Disabled;
        }

        private static bool HasRegionOwnedBy(InputRegion[] regions, InputOwner owner)
        {
            if (regions == null || regions.Length == 0)
            {
                return false;
            }

            foreach (var region in regions)
            {
                if (region.IsValid && region.Owner == owner)
                {
                    return true;
                }
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
