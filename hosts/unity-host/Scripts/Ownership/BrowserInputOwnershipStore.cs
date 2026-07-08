using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    internal sealed class BrowserInputOwnershipStore
    {
        private readonly InputOwnershipSettings _OwnershipSettings;
        private ulong _LastLoggedVersion;
        private int _LastLoggedDynamicRegionCount = -1;

        public BrowserInputOwnershipStore(InputOwnershipSettings ownershipSettings)
        {
            _OwnershipSettings = ownershipSettings;
        }

        public void Apply(BrowserInputOwnershipMapPayload payload)
        {
            if (_OwnershipSettings == null)
            {
                return;
            }

            if (!IsValidPayload(payload))
            {
                Debug.LogWarning(
                    $"[BrowserInputOwnershipStore] Ignored invalid input ownership map. version={payload.Version}, enabled={payload.Enabled}, viewport={payload.ViewportWidth}x{payload.ViewportHeight}, scale={payload.DeviceScaleFactor}, defaultOwner={payload.DefaultOwner}"
                );
                Clear();
                return;
            }

            var dynamicRegions = BuildDynamicRegions(payload);
            ApplyOwnershipMap(payload, dynamicRegions);
            LogApplied(payload, dynamicRegions.Length);
        }

        public void Clear()
        {
            var dynamicRegionCount = _OwnershipSettings?.OwnershipMap?.DynamicRegionCount ?? 0;
            _OwnershipSettings?.ResetDynamicOwnership();
            if (dynamicRegionCount > 0)
            {
                Debug.Log("[BrowserInputOwnershipStore] Cleared input ownership map.");
            }
        }

        private void ApplyOwnershipMap(
            BrowserInputOwnershipMapPayload payload,
            InputRegion[] dynamicRegions
        )
        {
            var ownershipMap = _OwnershipSettings.OwnershipMap;
            ownershipMap.Version = payload.Version;
            ownershipMap.Enabled = payload.Enabled;
            ownershipMap.ViewportWidth = payload.ViewportWidth;
            ownershipMap.ViewportHeight = payload.ViewportHeight;
            ownershipMap.DeviceScaleFactor = payload.DeviceScaleFactor;
            ownershipMap.DefaultOwner = ToInputOwner(payload.DefaultOwner);
            _OwnershipSettings.SetDynamicRegions(dynamicRegions);
        }

        private void LogApplied(BrowserInputOwnershipMapPayload payload, int dynamicRegionCount)
        {
            if (
                payload.Version == _LastLoggedVersion
                && dynamicRegionCount == _LastLoggedDynamicRegionCount
            )
            {
                return;
            }

            _LastLoggedVersion = payload.Version;
            _LastLoggedDynamicRegionCount = dynamicRegionCount;
            Debug.Log(
                $"[BrowserInputOwnershipStore] Applied input ownership map. version={payload.Version}, regions={payload.Regions.Length}, ownershipRegions={dynamicRegionCount}, viewport={payload.ViewportWidth}x{payload.ViewportHeight}, defaultOwner={ToInputOwner(payload.DefaultOwner)}"
            );
        }

        private static InputRegion[] BuildDynamicRegions(BrowserInputOwnershipMapPayload payload)
        {
            var regions = payload.Regions ?? Array.Empty<BrowserInputOwnershipRegionPayload>();
            var inputRegions = new InputRegion[regions.Length];
            var count = 0;

            foreach (var region in regions)
            {
                if (!TryCreateDynamicRegion(payload, region, out var inputRegion))
                {
                    continue;
                }

                inputRegions[count++] = inputRegion;
            }

            if (count == inputRegions.Length)
            {
                return inputRegions;
            }

            if (count == 0)
            {
                return Array.Empty<InputRegion>();
            }

            var compactRegions = new InputRegion[count];
            Array.Copy(inputRegions, compactRegions, count);
            return compactRegions;
        }

        private static bool TryCreateDynamicRegion(
            BrowserInputOwnershipMapPayload payload,
            BrowserInputOwnershipRegionPayload region,
            out InputRegion inputRegion
        )
        {
            inputRegion = default;

            if (!IsValidOwner(region.Owner) || !IsValidShape(region.Shape))
            {
                return false;
            }

            if (
                !IsFinite(region.X)
                || !IsFinite(region.Y)
                || !IsFinite(region.Width)
                || !IsFinite(region.Height)
                || !IsFinite(region.Radius)
                || region.Width <= 0f
                || region.Height <= 0f
                || region.Radius < 0f
            )
            {
                return false;
            }

            var normalizedRect = new Rect(
                region.X / payload.ViewportWidth,
                region.Y / payload.ViewportHeight,
                region.Width / payload.ViewportWidth,
                region.Height / payload.ViewportHeight
            );
            var normalizedRadius =
                region.Radius / Mathf.Min(payload.ViewportWidth, payload.ViewportHeight);
            inputRegion = new InputRegion(
                region.Id,
                ToInputOwner(region.Owner),
                ToInputRegionShape(region.Shape),
                normalizedRect,
                normalizedRadius,
                region.Disabled
            );
            return inputRegion.IsValid;
        }

        private static bool IsValidPayload(BrowserInputOwnershipMapPayload payload)
        {
            return payload.ViewportWidth > 0
                && payload.ViewportHeight > 0
                && IsFinite(payload.DeviceScaleFactor)
                && payload.DeviceScaleFactor > 0f
                && IsValidOwner(payload.DefaultOwner);
        }

        private static bool IsValidOwner(byte owner)
        {
            return owner == (byte)InputOwner.Web || owner == (byte)InputOwner.Host;
        }

        private static bool IsValidShape(byte shape)
        {
            return shape == (byte)InputRegionShape.Rect
                || shape == (byte)InputRegionShape.RoundedRect;
        }

        private static InputOwner ToInputOwner(byte owner)
        {
            return owner == (byte)InputOwner.Host ? InputOwner.Host : InputOwner.Web;
        }

        private static InputRegionShape ToInputRegionShape(byte shape)
        {
            return shape == (byte)InputRegionShape.RoundedRect
                ? InputRegionShape.RoundedRect
                : InputRegionShape.Rect;
        }

        private static bool IsFinite(float value)
        {
            return !float.IsNaN(value) && !float.IsInfinity(value);
        }
    }
}
