using UnityEngine;
using UnityEngine.EventSystems;

namespace KimoTech.LichoraHost
{
    public sealed class BrowserInputRouter
    {
        private readonly BrowserCoordinateMapper _CoordinateMapper;
        private IInputOwnershipResolver _OwnershipResolver;

        public BrowserInputRouter(
            BrowserCoordinateMapper coordinateMapper,
            IInputOwnershipResolver ownershipResolver
        )
        {
            _CoordinateMapper = coordinateMapper;
            _OwnershipResolver = ownershipResolver;
        }

        public IInputOwnershipResolver OwnershipResolver
        {
            get => _OwnershipResolver;
            set => _OwnershipResolver = value;
        }

        public InputOwner ResolveOwner(PointerEventData eventData)
        {
            return ResolveOwner(
                eventData.position,
                eventData.pressEventCamera != null
                    ? eventData.pressEventCamera
                    : eventData.enterEventCamera
            );
        }

        public InputOwner ResolveOwner(Vector2 screenPosition, Camera eventCamera)
        {
            if (_CoordinateMapper == null)
            {
                return InputOwner.Web;
            }

            var browserPosition = _CoordinateMapper.ScreenPointToBrowserNormalized(
                screenPosition,
                eventCamera
            );

            if (!IsValidBrowserPosition(browserPosition))
            {
                return InputOwner.Web;
            }

            if (_OwnershipResolver == null)
            {
                return InputOwner.Web;
            }

            var owner = _OwnershipResolver.ResolveOwner(browserPosition);
            return IsValidOwner(owner) ? owner : InputOwner.Web;
        }

        public bool IsBrowserOwner(Vector2 screenPosition, Camera eventCamera)
        {
            return ResolveOwner(screenPosition, eventCamera) == InputOwner.Web;
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
