using System;
using System.IO;
using System.Runtime.InteropServices;
using Cysharp.Threading.Tasks;
using Unity.Collections;
using UnityEngine;
using UnityEngine.UI;

namespace KimoTech.EmbeddedBrowser
{
    [RequireComponent(typeof(RawImage))]
    public class PageRenderer : PointableUI
    {
        private RawImage _RawImage;
        private Material _RenderMaterial;
        private Material _Material;
        private PageHandler _Handler;
        private NativeArray<byte> _TextureArray;
        private Texture2D _Texture2D;
        private int _LastCaptureFrameVersion = -1;
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

            _RawImage = GetComponent<RawImage>();
            _Texture2D = new Texture2D(Width, Height, TextureFormat.BGRA32, false, true)
            {
                filterMode = FilterMode.Bilinear,
            };
            _TextureArray = _Texture2D.GetRawTextureData<byte>();
            _RenderMaterial = Resources.Load<Material>("Materials/BrowserRender");
            _RawImage.texture = _Texture2D;

            if (_RenderMaterial == null)
            {
                Debug.LogWarning(
                    "[PageRenderer] Material 'Materials/BrowserRender' not found, fallback to RawImage default material."
                );
                _RawImage.material = null;
                _RawImage.uvRect = new Rect(0f, 1f, 1f, -1f);
            }
            else
            {
                _RawImage.uvRect = new Rect(0f, 0f, 1f, 1f);
                _Material = Instantiate(_RenderMaterial);
                _Material.mainTexture = _Texture2D;
                _Material.SetFloat("_FlipY", 1f);
                _Material.SetFloat("_ColorThreshold", FilteredColor ? 1 : 0);
                _RawImage.material = _Material;
            }

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
            if (CopyLatestTexture())
            {
                ApplyTexture();
            }
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
            _LastCaptureFrameVersion = -1;
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

        private void ApplyTexture()
        {
            if (!_TextureArray.IsCreated || _Texture2D == null)
            {
                return;
            }

            _Texture2D.Apply(false);
        }

        private bool CopyLatestTexture()
        {
            if (_Handler == null || !_Handler.State)
            {
                return false;
            }

            var dataWidth =
                _Handler.NativeFrameWidth > 0 ? _Handler.NativeFrameWidth : _Handler.DataWidth;
            var dataHeight =
                _Handler.NativeFrameHeight > 0 ? _Handler.NativeFrameHeight : _Handler.DataHeight;
            var expectedSize = dataWidth * dataHeight * 4;

            if (_TextureArray.Length != expectedSize && dataWidth > 0 && dataHeight > 0)
            {
                RecreateTextureWithSize(dataWidth, dataHeight);
                _LastCaptureFrameVersion = -1;
            }

            if (!_TextureArray.IsCreated || _TextureArray.Length != expectedSize)
            {
                return false;
            }

            return _Handler.TryCopyCaptureTo(_TextureArray, ref _LastCaptureFrameVersion);
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

        private void RecreateTextureWithSize(int width, int height)
        {
            _TextureArray = default;

            if (_Texture2D != null)
            {
                Destroy(_Texture2D);
            }

            _Texture2D = new Texture2D(width, height, TextureFormat.BGRA32, false, true)
            {
                filterMode = FilterMode.Bilinear,
            };
            _TextureArray = _Texture2D.GetRawTextureData<byte>();
            _RawImage.texture = _Texture2D;
            if (_Material != null)
            {
                _Material.mainTexture = _Texture2D;
            }
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

            _TextureArray = default;

            if (_Material != null)
            {
                Destroy(_Material);
            }

            if (_Texture2D != null)
            {
                Destroy(_Texture2D);
            }

            _RawImage = null;
        }
    }
}
