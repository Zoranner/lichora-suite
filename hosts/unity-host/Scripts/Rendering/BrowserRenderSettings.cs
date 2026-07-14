using UnityEngine;

namespace KimoTech.LichoraHost
{
    public readonly struct BrowserRenderSettings
    {
        public const string DefaultMaterialResourcePath = "Materials/BrowserRender";

        public BrowserRenderSettings(
            BrowserTransparencyMode transparencyMode,
            Color filterColor,
            float colorThreshold,
            bool flipY,
            string materialResourcePath
        )
        {
            TransparencyMode = transparencyMode;
            FilterColor = filterColor;
            ColorThreshold = colorThreshold;
            FlipY = flipY;
            MaterialResourcePath = string.IsNullOrWhiteSpace(materialResourcePath)
                ? DefaultMaterialResourcePath
                : materialResourcePath;
        }

        public BrowserTransparencyMode TransparencyMode { get; }
        public Color FilterColor { get; }
        public float ColorThreshold { get; }
        public bool FlipY { get; }
        public string MaterialResourcePath { get; }

        public static BrowserRenderSettings OpaqueDefault =>
            new BrowserRenderSettings(
                BrowserTransparencyMode.Opaque,
                Color.white,
                0f,
                true,
                DefaultMaterialResourcePath
            );

        public static BrowserRenderSettings FromLegacyFilteredColor(bool filteredColor)
        {
            return new BrowserRenderSettings(
                filteredColor ? BrowserTransparencyMode.ColorKey : BrowserTransparencyMode.Opaque,
                Color.white,
                filteredColor ? 1f : 0f,
                true,
                DefaultMaterialResourcePath
            );
        }
    }
}
