// ============================================================
// Project: EmbeddedBrowser
// Author: yangxinran@EN01-210826-09
// Datetime: 2024-04-08 10:41:15
// Description: Browser process launcher.
// ============================================================

using KimoTech.GlobalConfigs;
using UnityEngine;

namespace KimoTech.EmbeddedBrowser
{
    public class BrowserHandler : ProcessHandler
    {
        public bool Started { get; private set; } = false;

        public BrowserHandler(string guid)
            : base(GetPath(), BuildArguments(guid)) { }

        private static string GetPath()
        {
            var path = GlobalConfig<BrowserConfig>.Get().Path;
            if (string.IsNullOrWhiteSpace(path))
            {
                Debug.LogError(
                    "[BrowserHandler] HeadlessBrowser path is not configured. "
                        + "Create BrowserConfig.json in the application directory "
                        + "with content: { \"Path\": \"/path/to/HeadlessBrowser\" }"
                );
            }

            return path;
        }

        private static string BuildArguments(string guid)
        {
            var config = GlobalConfig<BrowserConfig>.Get();
            var graphicsMode = NormalizeGraphicsMode(config.GraphicsMode);
            return $"{guid} --graphics-mode={graphicsMode}";
        }

        private static string NormalizeGraphicsMode(string graphicsMode)
        {
            if (string.IsNullOrWhiteSpace(graphicsMode))
            {
                return "auto";
            }

            switch (graphicsMode.Trim().ToLowerInvariant())
            {
                case "off":
                case "false":
                    return "off";
                case "on":
                case "true":
                    return "on";
                default:
                    return "auto";
            }
        }

        public override void AfterStart()
        {
            base.AfterStart();
        }

        public override void BeforeStop()
        {
            base.BeforeStop();
        }

        ~BrowserHandler()
        {
            State = false;
        }
    }
}
