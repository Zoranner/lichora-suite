using System;

namespace KimoTech.LichoraHost
{
    public static class LegacyPassMapAdapter
    {
        public static InputOwnershipMap FromPassMap(PassMap passMap)
        {
            var ownershipMap = CreateDefaultWebMap();

            if (passMap == null)
            {
                return ownershipMap;
            }

            ownershipMap.StaticRegions = FromPassRegionsInternal(passMap.StaticPassRects);
            return ownershipMap;
        }

        public static InputOwnershipMap FromPassRegions(PassRegion[] passRegions)
        {
            var ownershipMap = CreateDefaultWebMap();
            ownershipMap.StaticRegions = FromPassRegionsInternal(passRegions);
            return ownershipMap;
        }

        public static InputRegion[] ToHostRegions(PassRegion[] passRegions)
        {
            return FromPassRegionsInternal(passRegions);
        }

        private static InputOwnershipMap CreateDefaultWebMap()
        {
            return new InputOwnershipMap { DefaultOwner = InputOwner.Web, Enabled = true };
        }

        private static InputRegion[] FromPassRegionsInternal(PassRegion[] passRegions)
        {
            if (passRegions == null || passRegions.Length == 0)
            {
                return Array.Empty<InputRegion>();
            }

            var regions = new InputRegion[passRegions.Length];
            var count = 0;

            foreach (var passRegion in passRegions)
            {
                if (!passRegion.IsValid)
                {
                    continue;
                }

                regions[count] = new InputRegion(
                    (uint)(count + 1),
                    InputOwner.Host,
                    passRegion.Rect
                );
                count++;
            }

            if (count == regions.Length)
            {
                return regions;
            }

            if (count == 0)
            {
                return Array.Empty<InputRegion>();
            }

            var compactRegions = new InputRegion[count];
            Array.Copy(regions, compactRegions, count);
            return compactRegions;
        }
    }
}
