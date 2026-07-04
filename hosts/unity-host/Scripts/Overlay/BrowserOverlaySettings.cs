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

        public void SetDynamicPassRects(PassRegion[] passRects)
        {
            PassMap.SetDynamicPassRects(passRects);
        }

        public void ClearDynamicPassRects()
        {
            PassMap.ClearDynamicPassRects();
        }

        public bool ShouldPassThrough(Vector2 browserNormalizedPosition)
        {
            switch (_PointerHitMode)
            {
                case PointerHitMode.StaticPassRects:
                    return _PassMap != null
                        && _PassMap.ContainsStaticPassRect(browserNormalizedPosition);
                case PointerHitMode.DomPassMap:
                    return _PassMap != null
                        && _PassMap.ContainsDynamicPassRect(browserNormalizedPosition);
                default:
                    return false;
            }
        }
    }
}
