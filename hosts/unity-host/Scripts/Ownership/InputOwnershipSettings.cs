using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    [Serializable]
    public sealed class InputOwnershipSettings
    {
        [SerializeField]
        private InputOwnershipMap _OwnershipMap = new InputOwnershipMap();

        public InputOwnershipMap OwnershipMap
        {
            get => _OwnershipMap;
            set => _OwnershipMap = value ?? new InputOwnershipMap();
        }

        public void SetDynamicRegions(InputRegion[] regions)
        {
            OwnershipMap.SetDynamicRegions(regions);
        }

        public void ClearDynamicRegions()
        {
            OwnershipMap.ClearDynamicRegions();
        }

        public InputOwner ResolveOwner(Vector2 normalizedPosition)
        {
            return OwnershipMap.ResolveOwner(normalizedPosition);
        }
    }
}
