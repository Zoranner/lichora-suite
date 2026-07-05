namespace KimoTech.LichoraHost
{
    public sealed class BrowserPointerCapture
    {
        private InputOwner _Owner = InputOwner.Web;

        public bool HasCapture { get; private set; }
        public InputOwner Owner => HasCapture ? _Owner : InputOwner.Web;

        public void Capture(InputOwner owner)
        {
            _Owner = IsValidOwner(owner) ? owner : InputOwner.Web;
            HasCapture = true;
        }

        public void Release()
        {
            HasCapture = false;
            _Owner = InputOwner.Web;
        }

        public InputOwner ResolveEffectiveOwner(InputOwner currentOwner, bool allowCapturedPointer)
        {
            if (allowCapturedPointer && HasCapture)
            {
                return Owner;
            }

            return IsValidOwner(currentOwner) ? currentOwner : InputOwner.Web;
        }

        private static bool IsValidOwner(InputOwner owner)
        {
            return owner == InputOwner.Web || owner == InputOwner.Host;
        }
    }
}
