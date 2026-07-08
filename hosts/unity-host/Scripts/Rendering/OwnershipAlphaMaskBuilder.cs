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

            var index = 0;
            for (var y = 0; y < height; y++)
            {
                var normalizedY = (y + 0.5f) / height;
                for (var x = 0; x < width; x++)
                {
                    var normalizedX = (x + 0.5f) / width;
                    var owner = snapshot.ResolveOwner(new Vector2(normalizedX, normalizedY));
                    mask[index++] = owner == InputOwner.Host ? (byte)0 : byte.MaxValue;
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
