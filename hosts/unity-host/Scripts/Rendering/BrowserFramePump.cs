namespace KimoTech.LichoraHost
{
    public sealed class BrowserFramePump
    {
        public bool Update(IBrowserFrameSource frameSource, BrowserSurface surface)
        {
            if (frameSource == null || surface == null || !frameSource.IsReady)
            {
                return false;
            }

            var width = frameSource.FrameWidth;
            var height = frameSource.FrameHeight;
            if (width <= 0 || height <= 0)
            {
                return false;
            }

            surface.EnsureTextureSize(width, height);

            if (!surface.TryGetTextureBuffer(out var buffer))
            {
                return false;
            }

            if (!frameSource.TryCopyFrameTo(buffer))
            {
                return false;
            }

            surface.Apply();
            return true;
        }
    }
}
