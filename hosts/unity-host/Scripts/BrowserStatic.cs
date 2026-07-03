using System;
using System.Collections.Generic;
using KimoTech.SingleBehaviours;
using KimoTech.SuperEvents;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    public class BrowserStatic : SingleBehaviour<BrowserStatic>
    {
        private const float RESTART_DELAY_SECONDS = 1f;

        private BrowserHandler _Handler;
        private readonly Dictionary<string, PageRegistration> _Pages =
            new Dictionary<string, PageRegistration>();
        private string _HandlerGuid;
        private bool _ShuttingDown;
        private bool _RestartRequested;
        private float _RestartAt;
        private readonly object _ProcessExitLock = new object();
        private bool _HasPendingProcessExit;
        private ProcessExitInfo _PendingProcessExit;
        private BrowserIpcClient _IpcClient;
        private BrowserIpcControlWriter _IpcControlWriter;
        private ulong _IpcControlSequence;

        public SuperEvent<bool> BrowserRestartingEvent { get; set; } = new SuperEvent<bool>();
        public SuperEvent<bool> BrowserRestartedEvent { get; set; } = new SuperEvent<bool>();
        public bool IsIpcOpen => _IpcClient != null && _IpcClient.IsOpen;

        protected override void Awake()
        {
            base.Awake();
            StartBrowserProcess();
        }

        private void Update()
        {
            ConsumeProcessExitSignal();
            TryRestartBrowser();
        }

        public void ResizePage(string guid, int width, int height)
        {
            if (_Pages.TryGetValue(guid, out var page))
            {
                _Pages[guid] = page.WithSize(width, height);
            }

            SendIpcControl(
                "ResizePage",
                (writer, sequence) => writer.ResizeBrowser(sequence, guid, width, height)
            );
        }

        public void AddPage(string guid, int width, int height, string address)
        {
            _Pages[guid] = new PageRegistration(guid, width, height, address);
            SendIpcControl(
                "AddPage",
                (writer, sequence) => writer.AddBrowser(sequence, guid, width, height, address)
            );
        }

        public void RemovePage(string guid)
        {
            _Pages.Remove(guid);
            SendIpcControl(
                "RemovePage",
                (writer, sequence) => writer.RemoveBrowser(sequence, guid)
            );
        }

        private void StartBrowserProcess()
        {
            _HandlerGuid = Guid.NewGuid().ToString();
            _Handler = new BrowserHandler(_HandlerGuid);
            _Handler.ProcessExitedEvent.AddListener(OnProcessExited);
            _Handler.Start();
            OpenBrowserIpcClient();
        }

        private void OpenBrowserIpcClient()
        {
            DisposeBrowserIpcClient();
            _IpcControlSequence = 0;

            try
            {
                _IpcClient = BrowserIpcClient.Open(_HandlerGuid);
                _IpcControlWriter = _IpcClient.CreateControlWriter();
            }
            catch (Exception exception)
            {
                DisposeBrowserIpcClient();
                Debug.LogWarning(
                    $"[BrowserStatic] Failed to open BrowserIpcClient for handler {_HandlerGuid}. {exception.Message}"
                );
            }
        }

        internal BrowserIpcInputWriter CreateIpcInputWriter(string browserId)
        {
            return _IpcClient?.CreateInputWriter(browserId);
        }

        internal BrowserIpcFrameReader CreateIpcFrameReader(string browserId)
        {
            return _IpcClient?.CreateFrameReader(browserId);
        }

        internal BrowserIpcOutputReader CreateIpcOutputReader(string browserId)
        {
            return _IpcClient?.CreateOutputReader(browserId);
        }

        private void SendIpcControl(
            string operation,
            Action<BrowserIpcControlWriter, ulong> writeControl
        )
        {
            if (_IpcControlWriter == null)
            {
                return;
            }

            try
            {
                writeControl(_IpcControlWriter, ++_IpcControlSequence);
            }
            catch (Exception exception)
            {
                Debug.LogWarning(
                    $"[BrowserStatic] Failed to send Browser IPC control {operation}. {exception.Message}"
                );
            }
        }

        private void DisposeBrowserIpcClient()
        {
            _IpcControlWriter = null;
            _IpcClient?.Dispose();
            _IpcClient = null;
        }

        private void OnProcessExited(ProcessExitInfo exitInfo)
        {
            if (_ShuttingDown || exitInfo.WasStopping)
            {
                return;
            }

            lock (_ProcessExitLock)
            {
                _PendingProcessExit = exitInfo;
                _HasPendingProcessExit = true;
            }
        }

        private void ConsumeProcessExitSignal()
        {
            ProcessExitInfo exitInfo;
            lock (_ProcessExitLock)
            {
                if (!_HasPendingProcessExit)
                {
                    return;
                }

                exitInfo = _PendingProcessExit;
                _HasPendingProcessExit = false;
            }

            Debug.LogWarning(
                $"[BrowserStatic] Lichora runtime exited unexpectedly "
                    + $"(PID={exitInfo.Pid}, code={exitInfo.ExitCode}, wasStopping={exitInfo.WasStopping}). "
                    + "Check Lichora runtime logs for shutdown reason, then scheduling restart."
            );
            RequestRestart();
        }

        private void RequestRestart()
        {
            if (_RestartRequested)
            {
                return;
            }

            _RestartRequested = true;
            _RestartAt = Time.realtimeSinceStartup + RESTART_DELAY_SECONDS;
        }

        private void TryRestartBrowser()
        {
            if (!_RestartRequested || Time.realtimeSinceStartup < _RestartAt)
            {
                return;
            }

            _RestartRequested = false;
            RestartBrowserProcess();
        }

        private void RestartBrowserProcess()
        {
            BrowserRestartingEvent?.Dispatch(true);

            _Handler?.ProcessExitedEvent.RemoveListener(OnProcessExited);
            DisposeBrowserIpcClient();

            StartBrowserProcess();

            foreach (var page in _Pages.Values)
            {
                SendIpcControl(
                    "AddPage",
                    (writer, sequence) =>
                        writer.AddBrowser(
                            sequence,
                            page.Guid,
                            page.Width,
                            page.Height,
                            page.Address
                        )
                );
            }

            BrowserRestartedEvent?.Dispatch(true);
        }

        private void OnDestroy()
        {
            if (_Handler == null)
            {
                return;
            }

            _ShuttingDown = true;

            SendShutdown();

            _Handler.ProcessExitedEvent.RemoveListener(OnProcessExited);
            DisposeBrowserIpcClient();
        }

        /// <summary>
        /// 向 Lichora runtime 发送 IPC v2 Shutdown 控制命令。
        /// Lichora runtime 收到后会清理所有 CEF 资源并退出进程。
        /// </summary>
        private void SendShutdown()
        {
            SendIpcControl("Shutdown", (writer, sequence) => writer.Shutdown(sequence));
        }

        private readonly struct PageRegistration
        {
            public string Guid { get; }
            public int Width { get; }
            public int Height { get; }
            public string Address { get; }

            public PageRegistration(string guid, int width, int height, string address)
            {
                Guid = guid;
                Width = width;
                Height = height;
                Address = address;
            }

            public PageRegistration WithSize(int width, int height)
            {
                return new PageRegistration(Guid, width, height, Address);
            }
        }
    }
}
