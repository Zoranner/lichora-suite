using UnityEngine;

namespace KimoTech.LichoraHost
{
    public sealed class LegacyPassOwnershipResolver : IInputOwnershipResolver
    {
        private BrowserOverlaySettings _Settings;

        public LegacyPassOwnershipResolver(BrowserOverlaySettings settings)
        {
            _Settings = settings;
        }

        public BrowserOverlaySettings Settings
        {
            get => _Settings;
            set => _Settings = value;
        }

        public InputOwner ResolveOwner(Vector2 browserNormalizedPosition)
        {
            if (_Settings == null)
            {
                return InputOwner.Web;
            }

            return _Settings.ShouldPassThrough(browserNormalizedPosition)
                ? InputOwner.Host
                : InputOwner.Web;
        }
    }
}
