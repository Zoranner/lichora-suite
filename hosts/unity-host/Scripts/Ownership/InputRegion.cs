using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    [Serializable]
    public struct InputRegion
    {
        [SerializeField]
        private uint _Id;

        [SerializeField]
        private InputOwner _Owner;

        [SerializeField]
        private InputRegionShape _Shape;

        [SerializeField]
        private Rect _Rect;

        [SerializeField]
        private float _Radius;

        [SerializeField]
        private bool _Disabled;

        public InputRegion(uint id, InputOwner owner, Rect rect)
            : this(id, owner, InputRegionShape.Rect, rect, 0f, false) { }

        public InputRegion(
            uint id,
            InputOwner owner,
            InputRegionShape shape,
            Rect rect,
            float radius,
            bool disabled
        )
        {
            _Id = id;
            _Owner = owner;
            _Shape = shape;
            _Rect = rect;
            _Radius = radius;
            _Disabled = disabled;
        }

        public uint Id => _Id;
        public InputOwner Owner => IsValidOwner(_Owner) ? _Owner : InputOwner.Web;
        public InputRegionShape Shape => IsValidShape(_Shape) ? _Shape : InputRegionShape.Rect;
        public Rect Rect => _Rect;
        public float Radius => _Radius;
        public bool Disabled => _Disabled;

        public bool Contains(Vector2 normalizedPosition)
        {
            if (!IsValid)
            {
                return false;
            }

            switch (Shape)
            {
                case InputRegionShape.RoundedRect:
                    return ContainsRoundedRect(normalizedPosition);
                default:
                    return _Rect.Contains(normalizedPosition, true);
            }
        }

        public bool IsValid =>
            !_Disabled
            && IsValidOwner(_Owner)
            && IsValidShape(_Shape)
            && IsFinite(_Rect.x)
            && IsFinite(_Rect.y)
            && IsFinite(_Rect.width)
            && IsFinite(_Rect.height)
            && IsFinite(_Radius)
            && _Rect.x >= 0f
            && _Rect.y >= 0f
            && _Rect.width > 0f
            && _Rect.height > 0f
            && _Rect.xMax <= 1f
            && _Rect.yMax <= 1f
            && _Radius >= 0f;

        private bool ContainsRoundedRect(Vector2 normalizedPosition)
        {
            if (!_Rect.Contains(normalizedPosition, true))
            {
                return false;
            }

            var radius = Mathf.Min(_Radius, _Rect.width * 0.5f, _Rect.height * 0.5f);
            if (radius <= 0f)
            {
                return true;
            }

            var left = _Rect.xMin + radius;
            var right = _Rect.xMax - radius;
            var bottom = _Rect.yMin + radius;
            var top = _Rect.yMax - radius;

            if (
                (normalizedPosition.x >= left && normalizedPosition.x <= right)
                || (normalizedPosition.y >= bottom && normalizedPosition.y <= top)
            )
            {
                return true;
            }

            var centerX = normalizedPosition.x < left ? left : right;
            var centerY = normalizedPosition.y < bottom ? bottom : top;
            var deltaX = normalizedPosition.x - centerX;
            var deltaY = normalizedPosition.y - centerY;
            return deltaX * deltaX + deltaY * deltaY <= radius * radius;
        }

        private static bool IsValidOwner(InputOwner owner)
        {
            return owner == InputOwner.Web || owner == InputOwner.Host;
        }

        private static bool IsValidShape(InputRegionShape shape)
        {
            return shape == InputRegionShape.Rect || shape == InputRegionShape.RoundedRect;
        }

        private static bool IsFinite(float value)
        {
            return !float.IsNaN(value) && !float.IsInfinity(value);
        }
    }
}
