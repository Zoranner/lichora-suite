using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    [Serializable]
    public struct PassRegion
    {
        [SerializeField]
        private Rect _Rect;

        public PassRegion(Rect rect)
        {
            _Rect = rect;
        }

        public Rect Rect => _Rect;

        public bool Contains(Vector2 browserNormalizedPosition)
        {
            if (!IsValid)
            {
                return false;
            }

            return _Rect.Contains(browserNormalizedPosition, true);
        }

        public bool IsValid =>
            IsFinite(_Rect.x)
            && IsFinite(_Rect.y)
            && IsFinite(_Rect.width)
            && IsFinite(_Rect.height)
            && _Rect.x >= 0f
            && _Rect.y >= 0f
            && _Rect.width > 0f
            && _Rect.height > 0f
            && _Rect.xMax <= 1f
            && _Rect.yMax <= 1f;

        private static bool IsFinite(float value)
        {
            return !float.IsNaN(value) && !float.IsInfinity(value);
        }
    }
}
