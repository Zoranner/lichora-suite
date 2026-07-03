using UnityEngine;

namespace KimoTech.LichoraHost
{
    public sealed class PassHitFilter
    {
        private readonly BrowserCoordinateMapper _CoordinateMapper;
        private BrowserOverlaySettings _Settings;

        public PassHitFilter(
            BrowserCoordinateMapper coordinateMapper,
            BrowserOverlaySettings settings
        )
        {
            _CoordinateMapper = coordinateMapper;
            _Settings = settings;
        }

        public BrowserOverlaySettings Settings
        {
            get => _Settings;
            set => _Settings = value;
        }

        public bool IsBrowserHit(Vector2 screenPosition, Camera eventCamera)
        {
            if (_Settings == null)
            {
                return true;
            }

            var browserPosition = _CoordinateMapper.ScreenPointToBrowserNormalized(
                screenPosition,
                eventCamera
            );

            if (!IsValidBrowserPosition(browserPosition))
            {
                return true;
            }

            return !_Settings.ShouldPassThrough(browserPosition);
        }

        private static bool IsValidBrowserPosition(Vector2 browserPosition)
        {
            return IsFinite(browserPosition.x)
                && IsFinite(browserPosition.y)
                && browserPosition.x >= 0f
                && browserPosition.x <= 1f
                && browserPosition.y >= 0f
                && browserPosition.y <= 1f;
        }

        private static bool IsFinite(float value)
        {
            return !float.IsNaN(value) && !float.IsInfinity(value);
        }
    }
}
