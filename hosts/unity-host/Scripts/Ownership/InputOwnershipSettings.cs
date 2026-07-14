using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    [Serializable]
    public sealed class InputOwnershipSettings : IInputOwnershipResolver
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

        public void SetStaticRegions(InputRegion[] regions)
        {
            OwnershipMap.StaticRegions = regions;
        }

        public void ClearDynamicRegions()
        {
            OwnershipMap.ClearDynamicRegions();
        }

        public void ResetDynamicOwnership()
        {
            OwnershipMap.ResetDynamicOwnership();
        }

        public InputOwner ResolveOwner(Vector2 normalizedPosition)
        {
            return OwnershipMap.ResolveOwner(normalizedPosition);
        }
    }
}
