using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    [Serializable]
    public sealed class PassMap
    {
        [SerializeField]
        private PassRegion[] _StaticPassRects = Array.Empty<PassRegion>();

        public PassRegion[] StaticPassRects
        {
            get => _StaticPassRects;
            set => _StaticPassRects = value ?? Array.Empty<PassRegion>();
        }

        public bool ContainsStaticPassRect(Vector2 browserNormalizedPosition)
        {
            if (_StaticPassRects == null || _StaticPassRects.Length == 0)
            {
                return false;
            }

            foreach (var region in _StaticPassRects)
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
