using System;
using System.IO;
using System.Runtime.InteropServices;
using Cysharp.Threading.Tasks;
using UnityEngine;
using UnityEngine.UI;

namespace KimoTech.LichoraHost
{
    [RequireComponent(typeof(RawImage))]
    public class PageRenderer : PointableUI
    {
        private PageHandler _Handler;
        private BrowserSurface _Surface;
        private BrowserFramePump _FramePump;
        private bool _PreviousHasFocus = false;
        private bool _BrowserRestartPending;

        public string GUID { get; private set; }
        public int Width { get; private set; } = 800;
        public int Height { get; private set; } = 600;

        private string _RealAddress;
        public string Address = "https://www.bing.com";

        public bool FilteredColor = false;

        // BrowserRender.shader 按 Unity UI 管线适配，统一处理 Y 轴翻转和背景色剔除。
        private static bool IsLinux => RuntimeInformation.IsOSPlatform(OSPlatform.Linux);

        private void Awake()
        {
            GUID = Guid.NewGuid().ToString();
            Width = (int)RectTransform.rect.width;
            Height = (int)RectTransform.rect.height;
        }

        private void Start()
        {
            if (!BrowserStatic.Instanced)
            {
                BrowserStatic.Instance.WakeUp();
            }

            _RealAddress = Address.StartsWith("local://")
                ? new Uri(
                    Path.Combine(
                        Environment.CurrentDirectory,
                        Address
                            .Substring("local://".Length)
                            .Replace('/', Path.DirectorySeparatorChar)
                    )
                ).AbsoluteUri
                : Address;
            BrowserStatic.Instance.AddPage(GUID, Width, Height, _RealAddress);

            _Surface = new BrowserSurface(
                GetComponent<RawImage>(),
                BrowserRenderSettings.FromLegacyFilteredColor(FilteredColor)
            );
            _Surface.Initialize(Width, Height);
            _FramePump = new BrowserFramePump();

            _Handler = CreatePageHandler();
            _ImeInputSink = _Handler;
            if (IsLinux)
            {
                _LinuxNativeImeModule = LinuxNativeImeModule.TryCreate(_Handler);
                _LinuxNativeImeModule?.SetSurroundingTextProvider(_Handler.SurroundingTextProvider);
            }

            BrowserStatic.Instance.BrowserRestartingEvent.AddListener(OnBrowserRestarting);
            BrowserStatic.Instance.BrowserRestartedEvent.AddListener(OnBrowserRestarted);
        }

        private void LateUpdate()
        {
            HandleBrowserRestart();
            CheckSizeChanged();
            UpdateMouseState();
            _Handler?.ApplyMainThreadUpdates();
            _LinuxNativeImeModule?.Update();
            UpdateKeyboardState();
            UpdateImeState();
            _FramePump?.Update(_Handler, _Surface);
            ResetScrollDelta();
        }

        private void HandleBrowserRestart()
        {
            if (!_BrowserRestartPending)
            {
                return;
            }

            _BrowserRestartPending = false;
            RecreateHandlerAfterBrowserRestart();
        }

        private void OnBrowserRestarted(bool _)
        {
            _BrowserRestartPending = true;
        }

        private void OnBrowserRestarting(bool _)
        {
            DestroyHandlerForBrowserRestart();
        }

        private void DestroyHandlerForBrowserRestart()
        {
            _LinuxNativeImeModule?.Dispose();
            _LinuxNativeImeModule = null;

            _Handler?.DestroyForBrowserRestart();
            _Handler = null;
            _ImeInputSink = null;
        }

        private void RecreateHandlerAfterBrowserRestart()
        {
            DestroyHandlerForBrowserRestart();

            _Handler = CreatePageHandler();
            _ImeInputSink = _Handler;
            _LinuxNativeImeModule = IsLinux ? LinuxNativeImeModule.TryCreate(_Handler) : null;
            _LinuxNativeImeModule?.SetSurroundingTextProvider(_Handler.SurroundingTextProvider);
        }

        private PageHandler CreatePageHandler()
        {
            return new PageHandler(
                GUID,
                _RealAddress,
                Width,
                Height,
                RectTransform,
                CreateIpcInputWriter(),
                CreateIpcFrameReader(),
                CreateIpcOutputReader()
            );
        }

        private BrowserIpcInputWriter CreateIpcInputWriter()
        {
            return BrowserStatic.Instanced
                ? BrowserStatic.Instance.CreateIpcInputWriter(GUID)
                : null;
        }

        private BrowserIpcFrameReader CreateIpcFrameReader()
        {
            return BrowserStatic.Instanced
                ? BrowserStatic.Instance.CreateIpcFrameReader(GUID)
                : null;
        }

        private BrowserIpcOutputReader CreateIpcOutputReader()
        {
            return BrowserStatic.Instanced
                ? BrowserStatic.Instance.CreateIpcOutputReader(GUID)
                : null;
        }

        private void CheckSizeChanged()
        {
            if (_Handler == null)
            {
                return;
            }

            var newWidth = (int)RectTransform.rect.width;
            var newHeight = (int)RectTransform.rect.height;

            if (newWidth == Width && newHeight == Height)
            {
                return;
            }

            if (newWidth <= 0 || newHeight <= 0)
            {
                return;
            }

            Width = newWidth;
            Height = newHeight;

            _Handler.Resize(Width, Height);
        }

        private void UpdateKeyboardState()
        {
            if (_Handler == null || !_Handler.State || !_HasFocus)
            {
                return;
            }

            _Handler.ProcessKeyboardEvents(_KeyboardEvents);
        }

        private void UpdateImeState()
        {
            if (_Handler == null || !_Handler.State)
            {
                return;
            }

            if (_LinuxNativeImeModule != null && _LinuxNativeImeModule.IsAvailable)
            {
                _ImeActive = false;
                return;
            }

            if (_HasFocus && !_PreviousHasFocus)
            {
                _Handler.ResetIme();
            }

            _PreviousHasFocus = _HasFocus;

            if (!_HasFocus)
            {
                return;
            }

            _ImeActive = _Handler.UpdateIme(_CompositionString, _InputString);
        }

        private void UpdateMouseState()
        {
            if (_Handler == null || !_Handler.State)
            {
                return;
            }

            _Handler.MouseState = _MouseState;
        }

        public void ExecuteScript(string script)
        {
            if (_Handler == null)
            {
                Debug.LogWarning($"Page {GUID} is not active, cannot execute script.");
                return;
            }

            _Handler.ExecuteScript(script);
        }

        private void OnDestroy()
        {
            if (BrowserStatic.Instanced)
            {
                BrowserStatic.Instance.BrowserRestartingEvent.RemoveListener(OnBrowserRestarting);
                BrowserStatic.Instance.BrowserRestartedEvent.RemoveListener(OnBrowserRestarted);
            }

            _LinuxNativeImeModule?.Dispose();

            if (BrowserStatic.Instanced)
            {
                BrowserStatic.Instance.RemovePage(GUID);
            }

            _Handler?.Destroy();
            _Surface?.Dispose();
            _Surface = null;
            _FramePump = null;
        }
    }
}
