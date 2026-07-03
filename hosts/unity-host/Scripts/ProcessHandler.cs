// ============================================================
// Project: ProcessHandler
// Author: yangxinran@EN01-210826-09
// Datetime: 2024-04-08 20:35:49
// ============================================================

using System;
using System.Threading;
using KimoTech.SuperEvents;
using UnityEngine;

namespace KimoTech.EmbeddedBrowser
{
    public class ProcessHandler
    {
        private int _NativePid = -1;
        private Thread _ProcessThread;
        private volatile bool _Stopping;

        public bool State { get; protected set; }

        public string ExecutablePath { get; private set; }

        public string Arguments { get; private set; }

        public SuperEvent<bool> StateChangedEvent { get; set; } = new SuperEvent<bool>();

        public SuperEvent<ProcessExitInfo> ProcessExitedEvent { get; set; } =
            new SuperEvent<ProcessExitInfo>();

        public ProcessHandler(string executablePath, string arguments)
        {
            ExecutablePath = executablePath;
            Arguments = arguments;
        }

        public void Start(bool debug = false)
        {
            if (_NativePid > 0)
            {
                Stop();
            }

            _Stopping = false;

            if (!debug)
            {
                _ProcessThread = new Thread(StartProcessThread);
                _ProcessThread.Start();
            }

            SetState(true);
            AfterStart();
        }

        private void StartProcessThread()
        {
            try
            {
                Debug.Log($"[ProcessHandler] Starting process: {ExecutablePath} {Arguments}");

                _NativePid = NativeProcessSpawner.Spawn(ExecutablePath, Arguments);

                if (_NativePid <= 0)
                {
                    Debug.LogError(
                        $"[ProcessHandler] Failed to spawn. Check path and permissions: {ExecutablePath}"
                    );
                    SetState(false);
                    return;
                }

                var pid = _NativePid;
                var exitCode = NativeProcessSpawner.Wait(pid);
                var wasStopping = _Stopping;
                var exitInfo = new ProcessExitInfo(pid, exitCode, wasStopping);
                Debug.Log($"[ProcessHandler] Process exited (code={exitCode}, PID={pid})");

                _NativePid = -1;
                SetState(false);
                ProcessExitedEvent?.Dispatch(exitInfo);
            }
            catch (ThreadAbortException)
            {
                _Stopping = true;
                throw;
            }
        }

        public virtual void AfterStart() { }

        public void Stop()
        {
            BeforeStop();
            _Stopping = true;

            if (_NativePid <= 0)
            {
                SetState(false);
                return;
            }

            try
            {
                // Graceful termination first
                NativeProcessSpawner.Kill(_NativePid, force: false);

                // Wait up to 2 seconds for the process to exit on its own
                var elapsed = 0;
                while (elapsed < 2000 && NativeProcessSpawner.IsRunning(_NativePid))
                {
                    Thread.Sleep(100);
                    elapsed += 100;
                }

                // Force kill if still running
                if (NativeProcessSpawner.IsRunning(_NativePid))
                {
                    NativeProcessSpawner.Kill(_NativePid, force: true);
                }
            }
            catch (Exception ex)
            {
                Debug.LogWarning($"[ProcessHandler] Stop error: {ex.Message}");
            }
            finally
            {
                _NativePid = -1;
                SetState(false);
            }
        }

        public virtual void BeforeStop() { }

        private void SetState(bool value)
        {
            if (State == value)
            {
                return;
            }

            State = value;
            StateChangedEvent?.Dispatch(value);
        }
    }

    public readonly struct ProcessExitInfo
    {
        public int Pid { get; }
        public int ExitCode { get; }
        public bool WasStopping { get; }

        public ProcessExitInfo(int pid, int exitCode, bool wasStopping)
        {
            Pid = pid;
            ExitCode = exitCode;
            WasStopping = wasStopping;
        }
    }
}
