using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    internal sealed class LegacyOverlayPassMapStore
    {
        private readonly InputOwnershipSettings _OwnershipSettings;
        private ulong _LastLoggedVersion;
        private int _LastLoggedDynamicRegionCount = -1;

        public LegacyOverlayPassMapStore(InputOwnershipSettings ownershipSettings)
        {
            _OwnershipSettings = ownershipSettings;
        }

        public void Apply(BrowserOverlayPassMapPayload payload)
        {
            if (_OwnershipSettings == null)
            {
                return;
            }

            if (!IsValidPayload(payload))
            {
                Debug.LogWarning(
                    $"[LegacyOverlayPassMapStore] Ignored invalid overlay pass map. version={payload.Version}, enabled={payload.Enabled}, viewport={payload.ViewportWidth}x{payload.ViewportHeight}, scale={payload.DeviceScaleFactor}"
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
                Debug.Log("[LegacyOverlayPassMapStore] Cleared overlay pass map.");
            }
        }

        private void ApplyOwnershipMap(
            BrowserOverlayPassMapPayload payload,
            InputRegion[] dynamicRegions
        )
        {
            if (_OwnershipSettings == null)
            {
                return;
            }

            var ownershipMap = _OwnershipSettings.OwnershipMap;
            ownershipMap.Version = payload.Version;
            ownershipMap.Enabled = payload.Enabled;
            ownershipMap.ViewportWidth = payload.ViewportWidth;
            ownershipMap.ViewportHeight = payload.ViewportHeight;
            ownershipMap.DeviceScaleFactor = payload.DeviceScaleFactor;
            ownershipMap.DefaultOwner = InputOwner.Web;
            _OwnershipSettings.SetDynamicRegions(dynamicRegions);
        }

        private void LogApplied(BrowserOverlayPassMapPayload payload, int dynamicRegionCount)
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
                $"[LegacyOverlayPassMapStore] Applied overlay pass map. version={payload.Version}, regions={payload.Regions.Length}, ownershipRegions={dynamicRegionCount}, viewport={payload.ViewportWidth}x{payload.ViewportHeight}"
            );
        }

        private static InputRegion[] BuildDynamicRegions(BrowserOverlayPassMapPayload payload)
        {
            var regions = payload.Regions ?? Array.Empty<BrowserOverlayPassRegionPayload>();
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
            BrowserOverlayPassMapPayload payload,
            BrowserOverlayPassRegionPayload region,
            out InputRegion inputRegion
        )
        {
            inputRegion = default;

            if (region.Disabled || region.Shape != 1)
            {
                return false;
            }

            if (
                !IsFinite(region.X)
                || !IsFinite(region.Y)
                || !IsFinite(region.Width)
                || !IsFinite(region.Height)
                || region.Width <= 0f
                || region.Height <= 0f
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
            inputRegion = new InputRegion(region.Id, InputOwner.Host, normalizedRect);
            return inputRegion.IsValid;
        }

        private static bool IsValidPayload(BrowserOverlayPassMapPayload payload)
        {
            return payload.Enabled
                && payload.ViewportWidth > 0
                && payload.ViewportHeight > 0
                && IsFinite(payload.DeviceScaleFactor)
                && payload.DeviceScaleFactor > 0f;
        }

        private static bool IsFinite(float value)
        {
            return !float.IsNaN(value) && !float.IsInfinity(value);
        }
    }
}
