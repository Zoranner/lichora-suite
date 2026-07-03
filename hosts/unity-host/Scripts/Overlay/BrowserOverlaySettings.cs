using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    [Serializable]
    public sealed class BrowserOverlaySettings
    {
        [SerializeField]
        private PointerHitMode _PointerHitMode = PointerHitMode.FullBrowserSurface;

        [SerializeField]
        private PassMap _PassMap = new PassMap();

        public PointerHitMode PointerHitMode
        {
            get => _PointerHitMode;
            set => _PointerHitMode = value;
        }

        public PassMap PassMap
        {
            get => _PassMap;
            set => _PassMap = value ?? new PassMap();
        }

        public bool ShouldPassThrough(Vector2 browserNormalizedPosition)
        {
            if (_PointerHitMode != PointerHitMode.StaticPassRects)
            {
                return false;
            }

            return _PassMap != null && _PassMap.ContainsStaticPassRect(browserNormalizedPosition);
        }
    }
}
