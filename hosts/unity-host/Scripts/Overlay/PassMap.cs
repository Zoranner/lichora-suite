using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    [Serializable]
    public sealed class PassMap
    {
        [SerializeField]
        private PassRegion[] _StaticPassRects = Array.Empty<PassRegion>();
        private PassRegion[] _DynamicPassRects = Array.Empty<PassRegion>();

        public PassRegion[] StaticPassRects
        {
            get => _StaticPassRects;
            set => _StaticPassRects = value ?? Array.Empty<PassRegion>();
        }

        public int DynamicPassRectCount => _DynamicPassRects?.Length ?? 0;

        public bool ContainsStaticPassRect(Vector2 browserNormalizedPosition)
        {
            return ContainsPassRect(_StaticPassRects, browserNormalizedPosition);
        }

        public void SetDynamicPassRects(PassRegion[] passRects)
        {
            _DynamicPassRects = passRects ?? Array.Empty<PassRegion>();
        }

        public void ClearDynamicPassRects()
        {
            _DynamicPassRects = Array.Empty<PassRegion>();
        }

        public bool ContainsDynamicPassRect(Vector2 browserNormalizedPosition)
        {
            return ContainsPassRect(_DynamicPassRects, browserNormalizedPosition);
        }

        private static bool ContainsPassRect(
            PassRegion[] regions,
            Vector2 browserNormalizedPosition
        )
        {
            if (regions == null || regions.Length == 0)
            {
                return false;
            }

            foreach (var region in regions)
            {
                if (region.Contains(browserNormalizedPosition))
                {
                    return true;
                }
            }

            return false;
        }
    }
}
