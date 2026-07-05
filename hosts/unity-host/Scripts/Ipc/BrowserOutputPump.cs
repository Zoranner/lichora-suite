using System;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    public sealed class BrowserOutputPump : IDisposable
    {
        private const int OutputBufferSize = 64 * 1024;
        private const int MaxOutputEventsPerFrame = 64;

        private readonly BrowserIpcOutputReader _OutputReader;
        private readonly BrowserOutputState _OutputState;
        private readonly BrowserOverlayPassMapStore _OverlayPassMapStore;
        private readonly byte[] _OutputBuffer = new byte[OutputBufferSize];
        private bool _HasLoggedInvalidOutput;

        public BrowserOutputPump(
            BrowserIpcOutputReader outputReader,
            RectTransform rectTransform,
            int width,
            int height,
            BrowserOverlaySettings overlaySettings
        )
        {
            _OutputReader = outputReader ?? throw new ArgumentNullException(nameof(outputReader));
            _OutputState = new BrowserOutputState(rectTransform, width, height);
            _OverlayPassMapStore = new BrowserOverlayPassMapStore(overlaySettings);
        }

        public ISurroundingTextSnapshotProvider SurroundingTextProvider => _OutputState;

        public void ReadLatestAndApplyMainThreadUpdates()
        {
            if (!ReadQueue())
            {
                ReadLatest();
            }
            _OutputState.ApplyPendingImePosition();
        }

        public void Resize(int width, int height)
        {
            _OutputState.Resize(width, height);
            _OverlayPassMapStore.Clear();
        }

        public void Dispose()
        {
            _OverlayPassMapStore.Clear();
            _OutputReader.Dispose();
        }

        private bool ReadQueue()
        {
            var hasEvents = false;

            for (var index = 0; index < MaxOutputEventsPerFrame; index += 1)
            {
                int written;
                try
                {
                    written = _OutputReader.TryPopEvent(
                        _OutputBuffer,
                        out var outputKind,
                        out var outputSequence
                    );
                }
                catch (Exception exception)
                {
                    LogInvalidOutput($"queue read failed: {exception.Message}");
                    return hasEvents;
                }

                if (written <= 0)
                {
                    return hasEvents;
                }

                hasEvents = true;
                _ = outputKind;
                _ = outputSequence;
                DecodeAndApply(written);
            }

            return hasEvents;
        }

        private void ReadLatest()
        {
            int written;
            try
            {
                written = _OutputReader.TryRead(_OutputBuffer);
            }
            catch (Exception exception)
            {
                LogInvalidOutput($"read failed: {exception.Message}");
                return;
            }

            if (written <= 0)
            {
                return;
            }

            DecodeAndApply(written);
        }

        private void DecodeAndApply(int written)
        {
            if (
                !BrowserIpcOutputPayload.TryDecode(
                    _OutputBuffer,
                    written,
                    out var payload,
                    out var error
                )
            )
            {
                LogInvalidOutput(error);
                _OverlayPassMapStore.Clear();
                return;
            }

            Apply(payload);
        }

        private void Apply(BrowserIpcOutputPayload payload)
        {
            switch (payload.Kind)
            {
                case BrowserIpcOutputPayloadKind.Caret:
                    _OutputState.ApplyCaret(
                        payload.CaretX,
                        payload.CaretY,
                        payload.CaretHeight,
                        payload.CaretVisible
                    );
                    break;
                case BrowserIpcOutputPayloadKind.SurroundingText:
                    _OutputState.ApplySurroundingText(
                        payload.Text,
                        payload.SelectionStart,
                        payload.SelectionEnd
                    );
                    break;
                case BrowserIpcOutputPayloadKind.OverlayPassMap:
                    _OverlayPassMapStore.Apply(payload.OverlayPassMap);
                    break;
                case BrowserIpcOutputPayloadKind.ScriptResult:
                case BrowserIpcOutputPayloadKind.PageEvent:
                    break;
            }
        }

        private void LogInvalidOutput(string message)
        {
            if (_HasLoggedInvalidOutput)
            {
                return;
            }

            _HasLoggedInvalidOutput = true;
            Debug.LogWarning($"Browser IPC output ignored: {message}");
        }
    }
}
