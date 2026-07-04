using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    internal sealed class BrowserOverlayPassMapStore
    {
        private readonly BrowserOverlaySettings _OverlaySettings;

        public BrowserOverlayPassMapStore(BrowserOverlaySettings overlaySettings)
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
                Clear();
                return;
            }

            _OverlaySettings.SetDynamicPassRects(BuildDynamicPassRects(payload));
        }

        public void Clear()
        {
            _OverlaySettings?.ClearDynamicPassRects();
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
