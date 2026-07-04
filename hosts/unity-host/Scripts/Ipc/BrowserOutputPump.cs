using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    public sealed class BrowserOutputPump : IDisposable
    {
        private readonly BrowserIpcOutputReader _OutputReader;
        private readonly BrowserOutputState _OutputState;

        public BrowserOutputPump(
            BrowserIpcOutputReader outputReader,
            RectTransform rectTransform,
            int width,
            int height,
            BrowserOverlaySettings overlaySettings
        )
        {
            _OutputReader = outputReader ?? throw new ArgumentNullException(nameof(outputReader));
            _OutputState = new BrowserOutputState(rectTransform, width, height, overlaySettings);
        }

        public ISurroundingTextSnapshotProvider SurroundingTextProvider => _OutputState;

        public void ReadLatestAndApplyMainThreadUpdates()
        {
            _OutputState.ReadLatest(_OutputReader);
            _OutputState.ApplyPendingImePosition();
        }

        public void Resize(int width, int height)
        {
            _OutputState.Resize(width, height);
        }

        public void Dispose()
        {
            _OutputState.ClearOverlayPassMap();
            _OutputReader.Dispose();
        }
    }
}
