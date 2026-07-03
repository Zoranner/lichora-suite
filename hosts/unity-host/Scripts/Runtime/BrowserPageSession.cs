using System;

namespace KimoTech.LichoraHost
{
    public sealed class BrowserPageSession
    {
        public BrowserPageSession(int width, int height)
        {
            GUID = Guid.NewGuid().ToString();
            Width = width;
            Height = height;
        }

        public string GUID { get; }
        public string Address { get; private set; }
        public int Width { get; private set; }
        public int Height { get; private set; }

        public void Register(string address)
        {
            Address = address ?? throw new ArgumentNullException(nameof(address));
            BrowserStatic.Instance.AddPage(GUID, Width, Height, Address);
        }

        public void Resize(int width, int height)
        {
            if (width == Width && height == Height)
            {
                return;
            }

            Width = width;
            Height = height;
            BrowserStatic.Instance.ResizePage(GUID, Width, Height);
        }

        public void Remove()
        {
            if (BrowserStatic.Instanced)
            {
                BrowserStatic.Instance.RemovePage(GUID);
            }
        }

        public BrowserIpcInputWriter CreateIpcInputWriter()
        {
            return BrowserStatic.Instanced
                ? BrowserStatic.Instance.CreateIpcInputWriter(GUID)
                : null;
        }

        public BrowserIpcFrameReader CreateIpcFrameReader()
        {
            return BrowserStatic.Instanced
                ? BrowserStatic.Instance.CreateIpcFrameReader(GUID)
                : null;
        }

        public BrowserIpcOutputReader CreateIpcOutputReader()
        {
            return BrowserStatic.Instanced
                ? BrowserStatic.Instance.CreateIpcOutputReader(GUID)
                : null;
        }
    }
}
