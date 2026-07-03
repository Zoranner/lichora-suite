using UnityEngine;

namespace KimoTech.LichoraHost
{
    public sealed class BrowserCoordinateMapper
    {
        private readonly RectTransform _RectTransform;

        public BrowserCoordinateMapper(RectTransform rectTransform)
        {
            _RectTransform = rectTransform;
        }

        public Vector2 ScreenPointToBrowserNormalized(
            Vector2 screenPosition,
            Camera eventCamera = null
        )
        {
            RectTransformUtility.ScreenPointToLocalPointInRectangle(
                _RectTransform,
                screenPosition,
                eventCamera,
                out var localPoint
            );

            var rect = _RectTransform.rect;
            var browserX = (localPoint.x - rect.min.x) / rect.width;
            var browserY = 1 - (localPoint.y - rect.min.y) / rect.height;

            return new Vector2(browserX, browserY);
        }
    }
}
