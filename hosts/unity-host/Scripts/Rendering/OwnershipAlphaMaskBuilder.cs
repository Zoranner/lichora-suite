using UnityEngine;

namespace KimoTech.LichoraHost
{
    internal static class OwnershipAlphaMaskBuilder
    {
        public static void Build(
            InputOwnershipRenderSnapshot snapshot,
            int width,
            int height,
            byte[] mask
        )
        {
            if (mask == null || width <= 0 || height <= 0 || mask.Length < width * height)
            {
                return;
            }

            if (!snapshot.Enabled || !snapshot.HasHostVisibleRegion)
            {
                Fill(mask, width * height, byte.MaxValue);
                return;
            }

            if (!snapshot.HasWebVisibleRegion)
            {
                Fill(mask, width * height, 0);
                return;
            }

            Fill(
                mask,
                width * height,
                snapshot.DefaultOwner == InputOwner.Host ? (byte)0 : byte.MaxValue
            );
            ApplyRegions(snapshot.StaticRegions, width, height, mask);
            ApplyRegions(snapshot.DynamicRegions, width, height, mask);
        }

        private static void ApplyRegions(InputRegion[] regions, int width, int height, byte[] mask)
        {
            if (regions == null || regions.Length == 0)
            {
                return;
            }

            foreach (var region in regions)
            {
                if (!region.IsValid)
                {
                    continue;
                }

                ApplyRegion(region, width, height, mask);
            }
        }

        private static void ApplyRegion(InputRegion region, int width, int height, byte[] mask)
        {
            var rect = region.Rect;
            var xMin = Mathf.Clamp(Mathf.FloorToInt(rect.xMin * width), 0, width);
            var xMax = Mathf.Clamp(Mathf.CeilToInt(rect.xMax * width), 0, width);
            var yMin = Mathf.Clamp(Mathf.FloorToInt(rect.yMin * height), 0, height);
            var yMax = Mathf.Clamp(Mathf.CeilToInt(rect.yMax * height), 0, height);
            if (xMin >= xMax || yMin >= yMax)
            {
                return;
            }

            var value = region.Owner == InputOwner.Host ? (byte)0 : byte.MaxValue;
            if (region.Shape == InputRegionShape.RoundedRect && region.Radius > 0f)
            {
                ApplyRoundedRect(region, width, height, mask, xMin, xMax, yMin, yMax, value);
                return;
            }

            ApplyRect(mask, width, xMin, xMax, yMin, yMax, value);
        }

        private static void ApplyRect(
            byte[] mask,
            int width,
            int xMin,
            int xMax,
            int yMin,
            int yMax,
            byte value
        )
        {
            for (var y = yMin; y < yMax; y++)
            {
                var rowIndex = y * width;
                for (var x = xMin; x < xMax; x++)
                {
                    mask[rowIndex + x] = value;
                }
            }
        }

        private static void ApplyRoundedRect(
            InputRegion region,
            int width,
            int height,
            byte[] mask,
            int xMin,
            int xMax,
            int yMin,
            int yMax,
            byte value
        )
        {
            for (var y = yMin; y < yMax; y++)
            {
                var normalizedY = (y + 0.5f) / height;
                var rowIndex = y * width;
                for (var x = xMin; x < xMax; x++)
                {
                    var normalizedX = (x + 0.5f) / width;
                    if (region.Contains(new Vector2(normalizedX, normalizedY)))
                    {
                        mask[rowIndex + x] = value;
                    }
                }
            }
        }

        private static void Fill(byte[] mask, int length, byte value)
        {
            for (var index = 0; index < length; index++)
            {
                mask[index] = value;
            }
        }
    }
}
