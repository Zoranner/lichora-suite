using System;
using System.IO;
using Cysharp.Threading.Tasks;
using UnityEngine;
using UnityEngine.UI;

namespace KimoTech.LichoraHost
{
    [RequireComponent(typeof(RawImage))]
    [RequireComponent(typeof(RectTransform))]
    [RequireComponent(typeof(BrowserInputController))]
    public class PageRenderer : MonoBehaviour
    {
        private PageHandler _Handler;
        private BrowserPageSession _Session;
        private BrowserSurface _Surface;
        private BrowserFramePump _FramePump;
        private BrowserInputController _InputController;
        private RectTransform _RectTransform;
        private bool _BrowserRestartPending;

        public string GUID => _Session?.GUID;
        public int Width => _Session?.Width ?? 800;
        public int Height => _Session?.Height ?? 600;

        public string Address = "https://www.bing.com";

        public bool FilteredColor = false;

        [SerializeField]
        private InputOwnershipSettings _InputOwnershipSettings = new InputOwnershipSettings();

        public InputOwnershipSettings InputOwnershipSettings => _InputOwnershipSettings;

        // BrowserRender.shader 按 Unity UI 管线适配，统一处理 Y 轴翻转和背景色剔除。
        private void Awake()
        {
            _RectTransform = GetComponent<RectTransform>();
            _InputController = GetComponent<BrowserInputController>();
            if (_InputController == null)
            {
                _InputController = gameObject.AddComponent<BrowserInputController>();
            }

            InitializeInputOwnership();

            var width = (int)RectTransform.rect.width;
            var height = (int)RectTransform.rect.height;
            _Session = new BrowserPageSession(width, height);
        }

        private void Start()
        {
            if (!BrowserStatic.Instanced)
            {
                BrowserStatic.Instance.WakeUp();
            }

            var realAddress = Address.StartsWith("local://")
                ? new Uri(
                    Path.Combine(
                        Environment.CurrentDirectory,
                        Address
                            .Substring("local://".Length)
                            .Replace('/', Path.DirectorySeparatorChar)
                    )
                ).AbsoluteUri
                : Address;

            _Session.Register(realAddress);

            _Surface = new BrowserSurface(GetComponent<RawImage>(), CreateRenderSettings());
            _Surface.Initialize(Width, Height);
            _FramePump = new BrowserFramePump();

            _Handler = CreatePageHandler();
            BindFocusControllerToHandler();

            BrowserStatic.Instance.BrowserRestartingEvent.AddListener(OnBrowserRestarting);
            BrowserStatic.Instance.BrowserRestartedEvent.AddListener(OnBrowserRestarted);
        }

        private void LateUpdate()
        {
            HandleBrowserRestart();
            CheckSizeChanged();
            UpdateMouseState();
            _Handler?.ApplyMainThreadUpdates();
            _InputController.FocusController.Update(
                _Handler,
                _InputController.CompositionString,
                _InputController.InputString
            );
            _FramePump?.Update(_Handler, _Surface);
            _InputController.ResetFrameDeltas();
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
            _InputController.ResetPointerState();
            _InputController.FocusController.UnbindImeInputSink();

            _Handler?.DestroyForBrowserRestart();
            _Handler = null;
        }

        private void RecreateHandlerAfterBrowserRestart()
        {
            DestroyHandlerForBrowserRestart();

            _Handler = CreatePageHandler();
            BindFocusControllerToHandler();
        }

        private PageHandler CreatePageHandler()
        {
            return new PageHandler(
                GUID,
                _Session.Address,
                Width,
                Height,
                RectTransform,
                _InputOwnershipSettings,
                _Session.CreateIpcInputWriter(),
                _Session.CreateIpcFrameReader(),
                _Session.CreateIpcOutputReader()
            );
        }

        private BrowserRenderSettings CreateRenderSettings()
        {
            return new BrowserRenderSettings(
                BrowserTransparencyMode.BrowserAlpha,
                Color.white,
                FilteredColor ? 1f : 0f,
                true,
                BrowserRenderSettings.DefaultMaterialResourcePath
            );
        }

        private void InitializeInputOwnership()
        {
            if (_InputOwnershipSettings == null)
            {
                _InputOwnershipSettings = new InputOwnershipSettings();
            }

            _InputOwnershipSettings.OwnershipMap.DefaultOwner = InputOwner.Web;
            _InputController.OwnershipResolver = _InputOwnershipSettings;
        }

        private void BindFocusControllerToHandler()
        {
            _InputController.FocusController.BindImeInputSink(
                _Handler,
                _Handler.SurroundingTextProvider
            );
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

            _Session.Resize(newWidth, newHeight);
            _Handler.Resize(Width, Height);
        }

        private void UpdateMouseState()
        {
            if (_Handler == null || !_Handler.State)
            {
                return;
            }

            _Handler.MouseState = _InputController.MouseState;
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

        public void ReleaseFocus()
        {
            _InputController?.ReleaseFocus();
        }

        private void OnDestroy()
        {
            if (BrowserStatic.Instanced)
            {
                BrowserStatic.Instance.BrowserRestartingEvent.RemoveListener(OnBrowserRestarting);
                BrowserStatic.Instance.BrowserRestartedEvent.RemoveListener(OnBrowserRestarted);
            }

            _InputController.DisposeFocusController();

            _Session?.Remove();

            _Handler?.Destroy();
            _Surface?.Dispose();
            _Surface = null;
            _FramePump = null;
        }

        private RectTransform RectTransform
        {
            get
            {
                if (_RectTransform == null)
                {
                    _RectTransform = GetComponent<RectTransform>();
                }

                return _RectTransform;
            }
        }
    }
}
