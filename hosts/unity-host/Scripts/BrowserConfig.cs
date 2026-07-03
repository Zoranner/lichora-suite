using KimoTech.GlobalConfigs;

namespace KimoTech.EmbeddedBrowser
{
    public class BrowserConfig : IBaseConfig
    {
        public string Path { get; set; } = "";

        public string GraphicsMode { get; set; } = "Auto";
    }
}
