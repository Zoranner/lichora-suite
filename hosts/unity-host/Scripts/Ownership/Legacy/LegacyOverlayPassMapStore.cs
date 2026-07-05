using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    internal sealed class LegacyOverlayPassMapStore
    {
        private readonly BrowserOverlaySettings _OverlaySettings;
        private ulong _LastLoggedVersion;
        private int _LastLoggedDynamicRectCount = -1;

        public LegacyOverlayPassMapStore(BrowserOverlaySettings overlaySettings)
        {
            _OverlaySettings = overlaySettings;
        }

        public void Apply(BrowserOverlayPassMapPayload payload)
        {
            if (_OverlaySettings == null)
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

            var dynamicPassRects = BuildDynamicPassRects(payload);
            _OverlaySettings.SetDynamicPassRects(dynamicPassRects);
            LogApplied(payload, dynamicPassRects.Length);
        }

        public void Clear()
        {
            var dynamicRectCount = _OverlaySettings?.PassMap?.DynamicPassRectCount ?? 0;
            _OverlaySettings?.ClearDynamicPassRects();
            if (dynamicRectCount > 0)
            {
                Debug.Log("[LegacyOverlayPassMapStore] Cleared overlay pass map.");
            }
        }

        private void LogApplied(BrowserOverlayPassMapPayload payload, int dynamicRectCount)
        {
            if (
                payload.Version == _LastLoggedVersion
                && dynamicRectCount == _LastLoggedDynamicRectCount
            )
            {
                return;
            }

            _LastLoggedVersion = payload.Version;
            _LastLoggedDynamicRectCount = dynamicRectCount;
            Debug.Log(
                $"[LegacyOverlayPassMapStore] Applied overlay pass map. version={payload.Version}, regions={payload.Regions.Length}, dynamicRects={dynamicRectCount}, viewport={payload.ViewportWidth}x{payload.ViewportHeight}"
            );
        }

        private static PassRegion[] BuildDynamicPassRects(BrowserOverlayPassMapPayload payload)
        {
            var regions = payload.Regions ?? Array.Empty<BrowserOverlayPassRegionPayload>();
            var passRects = new PassRegion[regions.Length];
            var count = 0;

            foreach (var region in regions)
            {
                if (!TryCreateDynamicPassRect(payload, region, out var passRect))
                {
                    continue;
                }

                passRects[count++] = passRect;
            }

            if (count == passRects.Length)
            {
                return passRects;
            }

            if (count == 0)
            {
                return Array.Empty<PassRegion>();
            }

            var compactPassRects = new PassRegion[count];
            Array.Copy(passRects, compactPassRects, count);
            return compactPassRects;
        }

        private static bool TryCreateDynamicPassRect(
            BrowserOverlayPassMapPayload payload,
            BrowserOverlayPassRegionPayload region,
            out PassRegion passRect
        )
        {
            passRect = default;

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
            passRect = new PassRegion(normalizedRect);
            return passRect.IsValid;
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
